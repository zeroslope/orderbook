use clob::state::{Side, TimeInForce};

use crate::svm::{TradingScenario, TwoUserScenario};

#[tokio::test]
async fn test_gtc_orders() {
    let scenario = TwoUserScenario::new().await;
    let market = &scenario.market;
    let alice = &scenario.alice.keypair;
    let bob = &scenario.bob.keypair;

    println!("=== Test: GTC (Good-Till-Cancelled) Orders ==>");

    // Alice places a GTC sell order that won't match immediately
    let result = market
        .place_limit_order_with_tif(alice, Side::Ask, 10, 50, TimeInForce::GTC)
        .await;
    assert!(
        result.is_ok(),
        "GTC ask order should be placed successfully"
    );

    // Verify the order is in the orderbook (should remain active)
    let alice_order = market.find_order_in_asks(1);
    assert!(
        alice_order.is_some(),
        "GTC order should remain in orderbook"
    );
    println!("GTC ask order placed and remains active in orderbook");

    // Bob places a GTC bid that partially matches
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 10, 30, TimeInForce::GTC)
        .await;
    assert!(result.is_ok(), "GTC bid order should match partially");

    // Verify Alice's order has reduced quantity but still exists (GTC behavior)
    let alice_order_after = market.find_order_in_asks(1);
    assert!(
        alice_order_after.is_some(),
        "Alice's GTC order should still exist"
    );
    assert_eq!(
        alice_order_after.unwrap().remaining_quantity,
        20,
        "Alice should have 20 remaining after partial fill"
    );

    // Bob's order should be fully consumed
    let bob_order = market.find_order_in_bids(2);
    assert!(bob_order.is_none(), "Bob's order should be fully filled");

    println!("GTC orders work correctly - remaining quantity stays in orderbook");
}

#[tokio::test]
async fn test_ioc_orders() {
    let scenario = TwoUserScenario::new().await;
    let market = &scenario.market;
    let alice = &scenario.alice.keypair;
    let bob = &scenario.bob.keypair;

    println!("=== Test: IOC (Immediate-Or-Cancel) Orders ==>");

    // Alice places a GTC sell order
    market
        .place_limit_order_with_tif(alice, Side::Ask, 10, 30, TimeInForce::GTC)
        .await
        .unwrap();

    // Bob places an IOC bid that partially matches - remaining quantity should be cancelled
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 10, 50, TimeInForce::IOC)
        .await;
    assert!(result.is_ok(), "IOC bid order should execute successfully");

    // Verify Alice's order is fully consumed
    let alice_order = market.find_order_in_asks(1);
    assert!(
        alice_order.is_none(),
        "Alice's order should be fully filled"
    );

    // Verify Bob's IOC order is NOT in the orderbook (cancelled remaining quantity)
    let bob_order = market.find_order_in_bids(2);
    assert!(
        bob_order.is_none(),
        "Bob's IOC order should not remain in orderbook"
    );

    // Verify orderbooks are empty (IOC doesn't leave resting orders)
    assert!(
        market.orderbooks_are_empty(),
        "Orderbooks should be empty after IOC"
    );

    println!("IOC orders work correctly - unfilled portion is cancelled");

    // Test IOC with no match - should not create any resting orders
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 5, 10, TimeInForce::IOC)
        .await;
    assert!(result.is_ok(), "IOC order with no match should succeed");

    let bids_count_after = market.get_orderbook_order_count(Side::Bid);
    assert_eq!(
        bids_count_after, 0,
        "IOC order with no match should not create resting order"
    );
    println!("IOC with no match correctly creates no resting orders");
}

#[tokio::test]
async fn test_fok_orders() {
    let scenario = TwoUserScenario::new().await;
    let market = &scenario.market;
    let alice = &scenario.alice.keypair;
    let bob = &scenario.bob.keypair;

    println!("=== Test: FOK (Fill-Or-Kill) Orders ==>");

    // Alice places a sell order
    market
        .place_limit_order_with_tif(alice, Side::Ask, 10, 30, TimeInForce::GTC)
        .await
        .unwrap();

    // Test 1: FOK order that can be completely filled
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 10, 30, TimeInForce::FOK)
        .await;
    assert!(
        result.is_ok(),
        "FOK order that can be completely filled should succeed"
    );

    // Verify both orders are consumed
    let alice_order = market.find_order_in_asks(1);
    assert!(
        alice_order.is_none(),
        "Alice's order should be fully filled"
    );
    let bob_order = market.find_order_in_bids(2);
    assert!(
        bob_order.is_none(),
        "Bob's FOK order should be fully filled"
    );

    println!("FOK order with complete fill succeeded");

    // Test 2: FOK order that cannot be completely filled - should be rejected
    // Alice places another sell order, but smaller than what Bob wants
    market
        .place_limit_order_with_tif(alice, Side::Ask, 10, 20, TimeInForce::GTC)
        .await
        .unwrap();

    // Bob tries FOK for more than available - should fail
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 10, 50, TimeInForce::FOK)
        .await;
    assert!(
        result.is_err(),
        "FOK order that cannot be completely filled should fail"
    );

    // Verify Alice's order is still there (no partial execution occurred)
    let alice_order_after = market.find_order_in_asks(3);
    assert!(
        alice_order_after.is_some(),
        "Alice's order should remain after failed FOK"
    );
    assert_eq!(
        alice_order_after.unwrap().remaining_quantity,
        20,
        "Alice's order should be unchanged after failed FOK"
    );

    println!("FOK order with incomplete fill correctly rejected");

    // Test 3: FOK order with no matching orders - should be rejected
    let result = market
        .place_limit_order_with_tif(bob, Side::Bid, 5, 10, TimeInForce::FOK)
        .await;
    assert!(result.is_err(), "FOK order with no match should fail");

    println!("FOK order with no match correctly rejected");
}

#[tokio::test]
async fn test_mixed_time_in_force_scenarios() {
    let scenario = TradingScenario::new().await;
    let market = &scenario.market;
    let alice = &scenario.alice.keypair;
    let bob = &scenario.bob.keypair;
    let charlie = &scenario.charlie.keypair;

    println!("=== Test: Mixed Time-In-Force Scenarios ==>");

    // Alice places a large GTC ask
    market
        .place_limit_order_with_tif(alice, Side::Ask, 10, 100, TimeInForce::GTC)
        .await
        .unwrap();

    // Bob places an IOC bid that partially matches - remaining is cancelled
    market
        .place_limit_order_with_tif(bob, Side::Bid, 10, 30, TimeInForce::IOC)
        .await
        .unwrap();

    // Verify Alice's order has reduced quantity
    let alice_order = market.find_order_in_asks(1);
    assert!(
        alice_order.is_some(),
        "Alice's GTC order should still exist"
    );
    assert_eq!(
        alice_order.unwrap().remaining_quantity,
        70,
        "Alice should have 70 remaining after Bob's IOC"
    );

    // Charlie places a FOK bid for exactly the remaining amount - should succeed
    let result = market
        .place_limit_order_with_tif(charlie, Side::Bid, 10, 70, TimeInForce::FOK)
        .await;
    assert!(
        result.is_ok(),
        "Charlie's FOK for exact remaining amount should succeed"
    );

    // Verify both orders are fully consumed
    let alice_order_final = market.find_order_in_asks(1);
    assert!(
        alice_order_final.is_none(),
        "Alice's order should be fully filled"
    );
    let charlie_order = market.find_order_in_bids(3);
    assert!(
        charlie_order.is_none(),
        "Charlie's FOK order should be fully filled"
    );

    // Verify orderbooks are clean
    assert!(
        market.orderbooks_are_empty(),
        "Orderbooks should be empty after mixed scenario"
    );

    println!("Mixed time-in-force scenarios work correctly");
}
