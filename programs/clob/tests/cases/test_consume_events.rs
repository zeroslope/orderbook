use crate::svm::{market::MarketFixture, test::TestFixture};
use clob::state::Side;
use solana_sdk::signature::Signer;
use std::rc::Rc;

#[tokio::test]
pub async fn test_consume_events_basic() {
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
    let bob_quote_account = fixture.quote_mint.create_token_account(&bob.pubkey()).await;

    // Mint tokens to users
    fixture
        .base_mint
        .mint_to(&alice_base_account, 100_000_000)
        .await; // 100 base tokens
    fixture
        .quote_mint
        .mint_to(&bob_quote_account, 100_000_000)
        .await; // 100 quote tokens

    // Deposit tokens
    market
        .deposit(
            &alice,
            fixture.base_mint.mint,
            alice_base_account,
            50_000_000,
        )
        .await
        .unwrap();
    market
        .deposit(&bob, fixture.quote_mint.mint, bob_quote_account, 50_000_000)
        .await
        .unwrap();

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
    // Initial deposits: Alice = 50M base, Bob = 50M quote
    // Trade: 5 lots at price 2000
    //
    // fill_base_amount = quantity * base_lot_size = 5 * 1_000_000 = 5_000_000
    // fill_quote_amount = price * quantity * quote_tick_size / base_lot_size
    //                   = 2000 * 5 * 1_000 / 1_000_000 = 10_000_000 / 1_000_000 = 10
    //
    // Alice (maker, ask): reserved 5M base in place_limit_order, gains 10 quote in consume_events
    // Bob (taker, bid): immediately gained 5M base and paid 10 quote in place_limit_order

    let expected_alice_base = 45_000_000; // 45M base (50M - 5M reserved)
    let expected_alice_quote = 10; // 10 quote (payment received)
    let expected_bob_base = 5_000_000; // 5M base (received immediately)
    let expected_bob_quote = 49999990; // 49999990 quote (50M - 10 paid)

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
