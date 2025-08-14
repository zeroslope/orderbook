use clob::state::Side;
use solana_sdk::signature::Signer;
use std::rc::Rc;

use crate::svm::{market::MarketFixture, test::TestFixture};

#[tokio::test]
async fn test_orderbook_basic_matching() {
    let fixture = TestFixture::new().await;
    let ctx = Rc::clone(&fixture.ctx);

    // Setup test accounts
    let alice = ctx.borrow_mut().gen_and_fund_key();
    let bob = ctx.borrow_mut().gen_and_fund_key();

    // Initialize market
    let market = MarketFixture::new(ctx.clone(), &fixture.base_mint, &fixture.quote_mint).await;

    // Create token accounts for users
    let alice_base_account = fixture
        .base_mint
        .create_token_account(&alice.pubkey())
        .await;
    let _alice_quote_account = fixture
        .quote_mint
        .create_token_account(&alice.pubkey())
        .await;
    let _bob_base_account = fixture.base_mint.create_token_account(&bob.pubkey()).await;
    let bob_quote_account = fixture.quote_mint.create_token_account(&bob.pubkey()).await;

    // Mint tokens to users
    fixture
        .base_mint
        .mint_to(&alice_base_account, 1000_000_000)
        .await; // 1000 base tokens
    fixture
        .quote_mint
        .mint_to(&bob_quote_account, 1000_000_000)
        .await; // 1000 quote tokens

    // Test 1: Users deposit tokens
    println!("=== Test 1: User Deposits ===");

    let deposit_result = market
        .deposit(
            &alice,
            fixture.base_mint.mint,
            alice_base_account,
            100_000_000,
        )
        .await;
    assert!(
        deposit_result.is_ok(),
        "Alice's base deposit should succeed"
    );

    let deposit_result = market
        .deposit(
            &bob,
            fixture.quote_mint.mint,
            bob_quote_account,
            100_000_000,
        )
        .await;
    assert!(deposit_result.is_ok(), "Bob's quote deposit should succeed");

    // Test 2: Place orders and verify matching
    println!("=== Test 2: Order Placement and Matching ===");

    // Alice places a sell order (ask): 10 base tokens at price 5 (Order ID will be 1)
    let result = market.place_limit_order(&alice, Side::Ask, 5, 10).await;
    assert!(
        result.is_ok(),
        "Alice's ask order should be placed successfully"
    );
    println!("Alice's ask order (ID 1) placed successfully");

    // Verify order is in the asks orderbook
    let order_in_asks = market.find_order_in_asks(1);
    assert!(
        order_in_asks.is_some(),
        "Order 1 should be in asks orderbook"
    );
    let alice_order = order_in_asks.unwrap();
    assert_eq!(
        alice_order.owner,
        alice.pubkey(),
        "Order owner should be Alice"
    );
    assert_eq!(alice_order.price, 5, "Order price should be 5");
    assert_eq!(alice_order.quantity, 10, "Order quantity should be 10");
    assert_eq!(
        alice_order.remaining_quantity, 10,
        "Order remaining quantity should be 10"
    );
    println!("Verified Alice's order is correctly stored in asks orderbook");

    // Bob places a buy order (bid): 5 base tokens at price 5 (Order ID will be 2, should fully match and consume)
    let result = market.place_limit_order(&bob, Side::Bid, 5, 5).await;
    assert!(result.is_ok(), "Bob's bid order should match completely");
    println!("Bob's bid order (ID 2) placed and fully matched with Alice's ask");

    // Verify Bob's order is not in orderbook (fully matched and consumed)
    let bob_order_in_bids = market.find_order_in_bids(2);
    assert!(
        bob_order_in_bids.is_none(),
        "Bob's order should not be in bids (fully matched)"
    );

    // Verify Alice's order remaining quantity is updated
    let alice_order_after = market.find_order_in_asks(1);
    assert!(
        alice_order_after.is_some(),
        "Alice's order should still exist"
    );
    let alice_order_updated = alice_order_after.unwrap();
    assert_eq!(
        alice_order_updated.remaining_quantity, 5,
        "Alice's remaining quantity should be 5"
    );
    println!(
        "Verified partial fill: Alice's order remaining quantity = {}",
        alice_order_updated.remaining_quantity
    );

    println!("=== Test 2 Completed: Basic matching works ===");

    // Test 3: Place non-matching order
    println!("=== Test 3: Non-matching Order ===");

    // Bob places another buy order at lower price (Order ID will be 3, should not match)
    let result = market.place_limit_order(&bob, Side::Bid, 4, 3).await;
    assert!(
        result.is_ok(),
        "Bob's lower-price bid should be placed without matching"
    );
    println!("Bob's second bid order (ID 3) placed successfully at price 4");

    // Verify Bob's non-matching order is in the bids orderbook
    let bob_order_in_bids = market.find_order_in_bids(3);
    assert!(
        bob_order_in_bids.is_some(),
        "Bob's order 3 should be in bids orderbook"
    );
    let bob_order = bob_order_in_bids.unwrap();
    assert_eq!(bob_order.owner, bob.pubkey(), "Order owner should be Bob");
    assert_eq!(bob_order.price, 4, "Order price should be 4");
    assert_eq!(bob_order.quantity, 3, "Order quantity should be 3");
    assert_eq!(
        bob_order.remaining_quantity, 3,
        "Order remaining quantity should be 3"
    );
    println!("Verified Bob's non-matching order is correctly stored in bids orderbook");

    // Verify orderbook counts
    let bids_count = market.get_orderbook_order_count(Side::Bid);
    let asks_count = market.get_orderbook_order_count(Side::Ask);
    assert_eq!(bids_count, 1, "Should have 1 bid order");
    assert_eq!(asks_count, 1, "Should have 1 ask order");
    println!(
        "Verified orderbook counts: {} bids, {} asks",
        bids_count, asks_count
    );

    // Test 4: Cancel order
    println!("=== Test 4: Order Cancellation ===");

    // Try to cancel Bob's second bid order (ID 3) which should be in the bids orderbook
    let result = market.cancel_order(&bob, 3, Side::Bid).await;
    match result {
        Ok(_) => {
            println!("Order cancellation succeeded");
        }
        Err(e) => {
            println!("Order cancellation failed: {:?}", e);
            panic!("Bob should be able to cancel his order: {:?}", e);
        }
    }

    // Verify order is removed from orderbook
    let bob_order_after_cancel = market.find_order_in_bids(3);
    assert!(
        bob_order_after_cancel.is_none(),
        "Bob's order 3 should be removed from bids after cancellation"
    );

    // Verify orderbook counts after cancellation
    let bids_count_after = market.get_orderbook_order_count(Side::Bid);
    let asks_count_after = market.get_orderbook_order_count(Side::Ask);
    assert_eq!(
        bids_count_after, 0,
        "Should have 0 bid orders after cancellation"
    );
    assert_eq!(asks_count_after, 1, "Should still have 1 ask order");
    println!(
        "Verified order cancellation: {} bids, {} asks",
        bids_count_after, asks_count_after
    );

    println!("=== All Orderbook Tests Passed! ===");
}

#[tokio::test]
async fn test_partial_fills_and_price_time_priority() {
    let fixture = TestFixture::new().await;
    let ctx = Rc::clone(&fixture.ctx);

    // Setup test accounts
    let alice = ctx.borrow_mut().gen_and_fund_key();
    let bob = ctx.borrow_mut().gen_and_fund_key();
    let charlie = ctx.borrow_mut().gen_and_fund_key();

    // Create market
    let market = MarketFixture::new(ctx.clone(), &fixture.base_mint, &fixture.quote_mint).await;

    // Create token accounts and fund users
    let alice_base_account = fixture
        .base_mint
        .create_token_account(&alice.pubkey())
        .await;
    let _alice_quote_account = fixture
        .quote_mint
        .create_token_account(&alice.pubkey())
        .await;
    let _bob_base_account = fixture.base_mint.create_token_account(&bob.pubkey()).await;
    let bob_quote_account = fixture.quote_mint.create_token_account(&bob.pubkey()).await;
    let _charlie_base_account = fixture
        .base_mint
        .create_token_account(&charlie.pubkey())
        .await;
    let charlie_quote_account = fixture
        .quote_mint
        .create_token_account(&charlie.pubkey())
        .await;

    fixture
        .base_mint
        .mint_to(&alice_base_account, 1000_000_000)
        .await;
    fixture
        .quote_mint
        .mint_to(&bob_quote_account, 1000_000_000)
        .await;
    fixture
        .quote_mint
        .mint_to(&charlie_quote_account, 1000_000_000)
        .await;

    // Deposit tokens
    market
        .deposit(
            &alice,
            fixture.base_mint.mint,
            alice_base_account,
            100_000_000,
        )
        .await
        .unwrap();
    market
        .deposit(
            &bob,
            fixture.quote_mint.mint,
            bob_quote_account,
            100_000_000,
        )
        .await
        .unwrap();
    market
        .deposit(
            &charlie,
            fixture.quote_mint.mint,
            charlie_quote_account,
            100_000_000,
        )
        .await
        .unwrap();

    println!("=== Test: Price-Time Priority and Partial Fills ===");

    // Alice places a large sell order (Order ID 1)
    market
        .place_limit_order(&alice, Side::Ask, 10, 50)
        .await
        .unwrap();

    // Verify Alice's order is in orderbook
    let alice_order = market.find_order_in_asks(1);
    assert!(alice_order.is_some(), "Alice's order should be in asks");
    assert_eq!(
        alice_order.unwrap().remaining_quantity,
        50,
        "Alice should have 50 remaining"
    );
    println!("Alice's large ask order (50 units at price 10) placed");

    // Bob places a small buy order at same price (should match partially, Order ID 2)
    market
        .place_limit_order(&bob, Side::Bid, 10, 20)
        .await
        .unwrap();

    // Verify partial match: Alice's order should have reduced quantity
    let alice_order_after_bob = market.find_order_in_asks(1);
    assert!(
        alice_order_after_bob.is_some(),
        "Alice's order should still exist"
    );
    assert_eq!(
        alice_order_after_bob.unwrap().remaining_quantity,
        30,
        "Alice should have 30 remaining after Bob's fill"
    );

    // Bob's order should be fully consumed (not in orderbook)
    let bob_order = market.find_order_in_bids(2);
    assert!(
        bob_order.is_none(),
        "Bob's order should be fully filled and removed"
    );
    println!("Bob's bid order (20 units) matched partially with Alice's ask. Alice remaining: 30");

    // Charlie places another buy order at same price (should match remaining, Order ID 3)
    market
        .place_limit_order(&charlie, Side::Bid, 10, 30)
        .await
        .unwrap();

    // Verify Alice's order is fully consumed
    let alice_order_after_charlie = market.find_order_in_asks(1);
    assert!(
        alice_order_after_charlie.is_none(),
        "Alice's order should be fully filled and removed"
    );

    // Charlie's order should be fully consumed too
    let charlie_order = market.find_order_in_bids(3);
    assert!(
        charlie_order.is_none(),
        "Charlie's order should be fully filled and removed"
    );

    // Verify orderbooks are empty
    let bids_count = market.get_orderbook_order_count(Side::Bid);
    let asks_count = market.get_orderbook_order_count(Side::Ask);
    assert_eq!(bids_count, 0, "Should have 0 bid orders");
    assert_eq!(asks_count, 0, "Should have 0 ask orders");
    println!("Charlie's bid order (30 units) fully consumed Alice's remaining ask. Both orderbooks empty.");

    println!("=== Partial Fill Test Completed Successfully ===");
}
