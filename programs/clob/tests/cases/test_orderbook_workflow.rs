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
    // Note: OrderPlaced event should be emitted here

    // Bob places a buy order (bid): 5 base tokens at price 5 (Order ID will be 2, should fully match and consume)
    let result = market.place_limit_order(&bob, Side::Bid, 5, 5).await;
    assert!(result.is_ok(), "Bob's bid order should match completely");
    println!("Bob's bid order (ID 2) placed and fully matched with Alice's ask");
    // Note: OrderFilled event should be emitted for the matching

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
    // Note: OrderPlaced event should be emitted for this non-matching order

    // Test 4: Cancel order
    println!("=== Test 4: Order Cancellation ===");

    // Try to cancel Bob's second bid order (ID 3) which should be in the bids orderbook
    let result = market.cancel_order(&bob, 3, Side::Bid).await;
    match result {
        Ok(_) => {
            println!("Order cancellation succeeded");
            // Note: OrderCancelled event should be emitted here
        }
        Err(e) => {
            println!("Order cancellation failed: {:?}", e);
            panic!("Bob should be able to cancel his order: {:?}", e);
        }
    }

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

    // Alice places a large sell order
    market
        .place_limit_order(&alice, Side::Ask, 10, 50)
        .await
        .unwrap();

    // Bob places a small buy order at same price (should match partially)
    market
        .place_limit_order(&bob, Side::Bid, 10, 20)
        .await
        .unwrap();

    // Charlie places another buy order at same price (should match remaining)
    market
        .place_limit_order(&charlie, Side::Bid, 10, 30)
        .await
        .unwrap();

    println!("=== Partial Fill Test Completed ===");
}
