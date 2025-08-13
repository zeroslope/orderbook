use std::rc::Rc;

use solana_sdk::signature::Signer;

use crate::svm::{market::MarketFixture, test::TestFixture};

#[tokio::test]
async fn test_vault_workflow() {
    let fixture = TestFixture::new().await;
    let ctx = Rc::clone(&fixture.ctx);

    let user = fixture.ctx.borrow_mut().gen_and_fund_key();

    // Setup initial token amounts (1000 tokens with 6 decimals)
    let initial_amount = 1_000_000_000;

    // Create ATAs and mint initial tokens to user
    let user_base_account = fixture
        .base_mint
        .create_and_mint(&user.pubkey(), initial_amount)
        .await;
    let user_quote_account = fixture
        .quote_mint
        .create_and_mint(&user.pubkey(), initial_amount)
        .await;

    // Step 1: Initialize market
    println!("=== Testing Market Initialization ===");
    let market = MarketFixture::new(ctx.clone(), &fixture.base_mint, &fixture.quote_mint).await;
    println!("Market initialized successfully at: {}", market.market);

    // Step 2: Test deposits
    println!("\n=== Testing Deposits ===");

    // Test deposit with sufficient balance
    let deposit_amount = 100_000_000; // 100 tokens
    println!(
        "Testing deposit with sufficient balance: {} tokens",
        deposit_amount / 1_000_000
    );

    // Deposit base tokens
    match market
        .deposit(
            &user,
            fixture.base_mint.mint,
            user_base_account,
            deposit_amount,
        )
        .await
    {
        Ok(_) => println!(
            "  ✓ Deposit of {} base tokens succeeded",
            deposit_amount / 1_000_000
        ),
        Err(_) => panic!("Expected deposit to succeed"),
    }

    // Deposit quote tokens
    match market
        .deposit(
            &user,
            fixture.quote_mint.mint,
            user_quote_account,
            deposit_amount,
        )
        .await
    {
        Ok(_) => println!(
            "  ✓ Deposit of {} quote tokens succeeded",
            deposit_amount / 1_000_000
        ),
        Err(_) => panic!("Expected deposit to succeed"),
    }

    // Test deposit with insufficient balance (should fail)
    let excessive_amount = 2_000_000_000; // 2000 tokens (more than available)
    println!(
        "Testing deposit with insufficient balance: {} tokens",
        excessive_amount / 1_000_000
    );

    match market
        .deposit(
            &user,
            fixture.base_mint.mint,
            user_base_account,
            excessive_amount,
        )
        .await
    {
        Ok(_) => panic!("Expected deposit to fail"),
        Err(_) => println!(
            "  ✓ Deposit of {} tokens failed as expected",
            excessive_amount / 1_000_000
        ),
    }

    // Step 3: Test withdrawals
    println!("\n=== Testing Withdrawals ===");

    // Test withdraw with sufficient balance
    let withdraw_amount = 50_000_000; // 50 tokens
    println!(
        "Testing withdraw with sufficient balance: {} tokens",
        withdraw_amount / 1_000_000
    );

    match market
        .withdraw(
            &user,
            fixture.base_mint.mint,
            user_base_account,
            withdraw_amount,
        )
        .await
    {
        Ok(_) => println!(
            "  ✓ Withdraw of {} base tokens succeeded",
            withdraw_amount / 1_000_000
        ),
        Err(err) => panic!("Expected withdraw to succeed: {:?}", err),
    }

    // Test withdraw with insufficient balance (should fail)
    let excessive_withdraw = 200_000_000; // 200 tokens (more than deposited)
    println!(
        "Testing withdraw with insufficient balance: {} tokens",
        excessive_withdraw / 1_000_000
    );

    match market
        .withdraw(
            &user,
            fixture.quote_mint.mint,
            user_quote_account,
            excessive_withdraw,
        )
        .await
    {
        Ok(_) => panic!("Expected withdraw to fail"),
        Err(_) => println!(
            "  ✓ Withdraw of {} tokens failed as expected",
            excessive_withdraw / 1_000_000
        ),
    }

    // Step 4: Test close user balance
    println!("\n=== Testing Close User Balance ===");

    // Test close with non-zero balance (should fail)
    println!("Testing close user balance with non-zero balance (should fail)");
    match market.close_user_balance(&user).await {
        Ok(_) => panic!("Expected close to fail with non-zero balance"),
        Err(_) => println!("  ✓ Close user balance failed as expected"),
    }

    // Withdraw remaining balances to make them zero
    let remaining_base = 50_000_000; // 50 tokens remaining from earlier deposit
    let remaining_quote = 100_000_000; // 100 tokens remaining from earlier deposit

    println!("Withdrawing remaining balances to zero...");

    match market
        .withdraw(
            &user,
            fixture.base_mint.mint,
            user_base_account,
            remaining_base,
        )
        .await
    {
        Ok(_) => println!(
            "  ✓ Withdraw remaining {} base tokens succeeded",
            remaining_base / 1_000_000
        ),
        Err(err) => panic!("Expected withdraw to succeed: {:?}", err),
    }

    match market
        .withdraw(
            &user,
            fixture.quote_mint.mint,
            user_quote_account,
            remaining_quote,
        )
        .await
    {
        Ok(_) => println!(
            "  ✓ Withdraw remaining {} quote tokens succeeded",
            remaining_quote / 1_000_000
        ),
        Err(err) => panic!("Expected withdraw to succeed: {:?}", err),
    }

    // Test close with zero balance (should succeed)
    println!("Testing close user balance with zero balance (should succeed)");
    match market.close_user_balance(&user).await {
        Ok(_) => println!("  ✓ Close user balance succeeded"),
        Err(err) => panic!("Expected close to succeed: {:?}", err),
    }

    println!("\n=== All tests completed successfully! ===");
}
