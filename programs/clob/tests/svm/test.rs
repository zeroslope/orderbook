use super::{market::MarketFixture, spl::MintFixture, SvmContext};
use anchor_lang::prelude::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::{cell::RefCell, rc::Rc};

pub struct TestFixture {
    pub ctx: Rc<RefCell<SvmContext>>,
    pub base_mint: MintFixture,
    pub quote_mint: MintFixture,
}

impl TestFixture {
    pub async fn new() -> Self {
        let mut ctx = SvmContext::new();
        ctx.svm
            .add_program_from_file(clob::ID, "../../target/deploy/clob.so")
            .expect("Failed to add clob program");

        let ctx = Rc::new(RefCell::new(ctx));

        // Create base mint (6 decimals for typical token)
        let base_mint_keypair = Keypair::new();
        let base_mint = MintFixture::new(
            ctx.clone(),
            base_mint_keypair,
            6, // decimals
        )
        .await;

        // Create quote mint (6 decimals for typical token)
        let quote_mint_keypair = Keypair::new();
        let quote_mint = MintFixture::new(
            ctx.clone(),
            quote_mint_keypair,
            6, // decimals
        )
        .await;

        Self {
            ctx,
            base_mint,
            quote_mint,
        }
    }
}

/// Pre-configured trading scenario with users and market ready for testing
pub struct TradingScenario {
    pub fixture: TestFixture,
    pub market: MarketFixture,
    pub alice: TradingUser,
    pub bob: TradingUser,
    pub charlie: TradingUser,
}

impl TradingScenario {
    pub async fn new() -> Self {
        let fixture = TestFixture::new().await;
        let ctx = Rc::clone(&fixture.ctx);

        // Initialize market
        let market = MarketFixture::new(ctx.clone(), &fixture.base_mint, &fixture.quote_mint).await;

        // Create pre-configured users
        let alice = TradingUser::new(ctx.clone(), &fixture, &market, "alice").await;
        let bob = TradingUser::new(ctx.clone(), &fixture, &market, "bob").await;
        let charlie = TradingUser::new(ctx.clone(), &fixture, &market, "charlie").await;

        Self {
            fixture,
            market,
            alice,
            bob,
            charlie,
        }
    }
}

/// Simplified two-user trading scenario
pub struct TwoUserScenario {
    pub market: MarketFixture,
    pub alice: TradingUser,
    pub bob: TradingUser,
}

impl TwoUserScenario {
    pub async fn new() -> Self {
        let scenario = TradingScenario::new().await;
        Self {
            market: scenario.market,
            alice: scenario.alice,
            bob: scenario.bob,
        }
    }
}

/// Pre-configured user with tokens and market deposits ready
pub struct TradingUser {
    pub keypair: Keypair,
    pub base_account: Pubkey,
    pub quote_account: Pubkey,
}

impl TradingUser {
    pub async fn new(
        ctx: Rc<RefCell<SvmContext>>,
        fixture: &TestFixture,
        market: &MarketFixture,
        _name: &str, // for debugging/logging purposes
    ) -> Self {
        // Generate and fund user
        let keypair = ctx.borrow_mut().gen_and_fund_key();

        // Create token accounts
        let base_account = fixture
            .base_mint
            .create_token_account(&keypair.pubkey())
            .await;
        let quote_account = fixture
            .quote_mint
            .create_token_account(&keypair.pubkey())
            .await;

        // Mint initial tokens (1000 tokens with 6 decimals = 1000_000_000)
        fixture.base_mint.mint_to(&base_account, 1000_000_000).await;
        fixture
            .quote_mint
            .mint_to(&quote_account, 1000_000_000)
            .await;

        // Deposit tokens to market (100 tokens with 6 decimals = 100_000_000)
        market
            .deposit(&keypair, fixture.base_mint.mint, base_account, 100_000_000)
            .await
            .expect("Failed to deposit base tokens");

        market
            .deposit(
                &keypair,
                fixture.quote_mint.mint,
                quote_account,
                100_000_000,
            )
            .await
            .expect("Failed to deposit quote tokens");

        Self {
            keypair,
            base_account,
            quote_account,
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Creates a lightweight user for vault testing - no market deposits
    pub async fn new_for_vault_testing(
        ctx: Rc<RefCell<SvmContext>>,
        fixture: &TestFixture,
    ) -> Self {
        // Generate and fund user
        let keypair = ctx.borrow_mut().gen_and_fund_key();

        // Create token accounts and mint initial tokens
        let base_account = fixture
            .base_mint
            .create_and_mint(&keypair.pubkey(), 1000_000_000)
            .await;
        let quote_account = fixture
            .quote_mint
            .create_and_mint(&keypair.pubkey(), 1000_000_000)
            .await;

        Self {
            keypair,
            base_account,
            quote_account,
        }
    }
}
