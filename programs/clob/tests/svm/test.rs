use super::{spl::MintFixture, SvmContext};
use solana_sdk::signature::Keypair;
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
