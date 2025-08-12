use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{get_associated_token_address, spl_associated_token_account},
    token::{spl_token, Mint, TokenAccount},
};
use solana_sdk::{signature::Keypair, signer::Signer, system_instruction::create_account};
use std::{cell::RefCell, rc::Rc};

use super::SvmContext;

#[derive(Clone)]
pub struct MintFixture {
    ctx: Rc<RefCell<SvmContext>>,
    pub mint: Pubkey,
    pub decimals: u8,
    pub token_program: Pubkey,
}

impl MintFixture {
    pub async fn new(
        ctx: Rc<RefCell<SvmContext>>,
        mint_keypair: Keypair,
        mint_decimals: u8,
    ) -> Self {
        let ctx_ref = Rc::clone(&ctx);
        {
            let mut ctx = ctx_ref.borrow_mut();
            let init_account_ix = create_account(
                &ctx.payer.pubkey(),
                &mint_keypair.pubkey(),
                ctx.svm.minimum_balance_for_rent_exemption(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::ID,
            );
            let init_mint_ix = spl_token::instruction::initialize_mint(
                &spl_token::ID,
                &mint_keypair.pubkey(),
                &ctx.payer.pubkey(),
                None,
                mint_decimals,
            )
            .unwrap();

            ctx.submit_transaction(&[init_account_ix, init_mint_ix], &[&mint_keypair])
                .unwrap();
        }

        MintFixture {
            ctx: ctx_ref,
            mint: mint_keypair.pubkey(),
            decimals: mint_decimals,
            token_program: spl_token::ID,
        }
    }

    pub async fn balance(&self, pubkey: Pubkey) -> u64 {
        self.ctx
            .borrow()
            .load_and_deserialize::<TokenAccount>(&pubkey)
            .amount
    }

    // Get the Associated Token Account address for this mint and owner
    pub fn get_ata_address(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.mint)
    }

    // Create an Associated Token Account for this mint
    pub async fn create_token_account(&self, owner: &Pubkey) -> Pubkey {
        let mut ctx = self.ctx.borrow_mut();

        // Calculate the Associated Token Account address
        let ata_address = self.get_ata_address(owner);

        // Check if the ATA already exists
        if ctx.svm.get_account(&ata_address).is_some() {
            return ata_address;
        }

        // Create the Associated Token Account
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &ctx.payer.pubkey(), // payer
                owner,               // wallet
                &self.mint,          // mint
                &spl_token::ID,      // token program
            );

        ctx.submit_transaction(&[create_ata_ix], &[]).unwrap();

        ata_address
    }

    // Mint tokens to a token account
    pub async fn mint_to(&self, token_account: &Pubkey, amount: u64) {
        let mut ctx = self.ctx.borrow_mut();

        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &self.mint,
            token_account,
            &ctx.payer.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        ctx.submit_transaction(&[mint_to_ix], &[]).unwrap();
    }

    pub async fn create_and_mint(&self, owner: &Pubkey, amount: u64) -> Pubkey {
        let ata_address = self.create_token_account(owner).await;
        self.mint_to(&ata_address, amount).await;
        ata_address
    }
}
