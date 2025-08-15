use crate::svm::TwoUserScenario;
use clob::state::Side;
use solana_sdk::signature::Signer;

#[tokio::test]
pub async fn test_consume_events_basic() {
    let scenario = TwoUserScenario::new().await;
    let market = &scenario.market;
    let alice = &scenario.alice.keypair;
    let bob = &scenario.bob.keypair;

    println!("=== Test: Event Queue and Consume Events ===");

    // Step 1: Alice places ask order (maker)
    market
        .place_limit_order(&alice, Side::Ask, 2000, 5)
        .await
        .unwrap();
    println!("Alice placed ask order: 5 base at price 2000");

    // Step 2: Bob places matching bid order (taker)
    market
        .place_limit_order(&bob, Side::Bid, 2000, 5)
        .await
        .unwrap();
    println!("Bob placed bid order: 5 base at price 2000 (should match)");

    // Step 3: Consume events to update maker (Alice) balance
    let result = market.consume_events(10, &[&alice]).await;
    assert!(result.is_ok(), "Consume events should succeed");

    // Step 4: Verify balances are updated correctly
    let alice_balance = market.get_user_balance(&alice.pubkey());
    let bob_balance = market.get_user_balance(&bob.pubkey());

    // Calculate expected balances after consume_events
    // Market params: base_lot_size = 1_000_000, quote_tick_size = 1_000
    // Initial deposits: Alice = 100M base, Bob = 100M quote (TradingScenario deposits 100M each)
    // Trade: 5 lots at price 2000
    //
    // fill_base_amount = quantity * base_lot_size = 5 * 1_000_000 = 5_000_000
    // fill_quote_amount = price * quantity * quote_tick_size / base_lot_size
    //                   = 2000 * 5 * 1_000 / 1_000_000 = 10_000_000 / 1_000_000 = 10
    //
    // Alice (maker, ask): reserved 5M base in place_limit_order, gains 10 quote in consume_events
    // Bob (taker, bid): immediately gained 5M base and paid 10 quote in place_limit_order

    let expected_alice_base = 95_000_000; // 95M base (100M - 5M reserved)
    let expected_alice_quote = 100_000_010; // 100M + 10 quote (initial + payment received)
    let expected_bob_base = 105_000_000; // 105M base (100M + 5M received)
    let expected_bob_quote = 99_999_990; // ~100M - 10 quote (100M - 10 paid)

    assert_eq!(
        alice_balance.base_balance, expected_alice_base,
        "Alice should have {} base, got {}",
        expected_alice_base, alice_balance.base_balance
    );
    assert_eq!(
        alice_balance.quote_balance, expected_alice_quote,
        "Alice should have {} quote, got {}",
        expected_alice_quote, alice_balance.quote_balance
    );
    assert_eq!(
        bob_balance.base_balance, expected_bob_base,
        "Bob should have {} base, got {}",
        expected_bob_base, bob_balance.base_balance
    );
    assert_eq!(
        bob_balance.quote_balance, expected_bob_quote,
        "Bob should have {} quote, got {}",
        expected_bob_quote, bob_balance.quote_balance
    );

    println!("=== Test Complete ===");
}
