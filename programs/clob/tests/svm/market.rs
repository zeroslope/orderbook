use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use clob::instructions::*;
use litesvm::types::TransactionResult;
use solana_sdk::signature::{Keypair, Signer};
use std::{cell::RefCell, rc::Rc};

use super::{spl::MintFixture, SvmContext};

pub struct MarketFixture {
    ctx: Rc<RefCell<SvmContext>>,
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
}

impl MarketFixture {
    pub async fn new(
        ctx: Rc<RefCell<SvmContext>>,
        base_mint: &MintFixture,
        quote_mint: &MintFixture,
    ) -> Self {
        let ctx_ref = ctx.clone();
        let mut ctx = ctx.borrow_mut();

        let (market, _) = Pubkey::find_program_address(
            &[b"market", base_mint.mint.as_ref(), quote_mint.mint.as_ref()],
            &clob::ID,
        );

        let (base_vault, _) = get_vault_pda(&market, &base_mint.mint);
        let (quote_vault, _) = get_vault_pda(&market, &quote_mint.mint);

        let authority = ctx.payer.pubkey();
        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::Initialize {
                authority,
                market,
                base_vault,
                quote_vault,
                base_mint: base_mint.mint,
                quote_mint: quote_mint.mint,
                base_token_program: anchor_spl::token::ID,
                quote_token_program: anchor_spl::token::ID,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: clob::instruction::Initialize {
                params: InitializeParams {
                    base_mint: base_mint.mint,
                    quote_mint: quote_mint.mint,
                },
            }
            .data(),
        };

        ctx.submit_transaction(&[ix], &[])
            .expect("Failed to initialize market");

        Self {
            ctx: ctx_ref,
            market,
            base_mint: base_mint.mint,
            quote_mint: quote_mint.mint,
            base_vault,
            quote_vault,
        }
    }

    pub async fn deposit(
        &self,
        user: &Keypair,
        mint: Pubkey,
        user_token_account: Pubkey,
        amount: u64,
    ) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        let (user_balance_pda, _) = get_user_balance_pda(&user.pubkey(), &self.market);
        let (vault_token_account, _) = get_vault_pda(&self.market, &mint);
        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::Deposit {
                user: user.pubkey(),
                market: self.market,
                user_balance: user_balance_pda,
                user_token_account,
                vault_token_account,
                mint,
                token_program: anchor_spl::token::ID,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: clob::instruction::Deposit {
                params: DepositParams { amount },
            }
            .data(),
        };

        ctx.submit_transaction(&[ix], &[user])
    }

    pub async fn withdraw(
        &self,
        user: &Keypair,
        mint: Pubkey,
        user_token_account: Pubkey,
        amount: u64,
    ) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        let (user_balance_pda, _) = get_user_balance_pda(&user.pubkey(), &self.market);
        let (vault_token_account, _) = get_vault_pda(&self.market, &mint);
        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::Withdraw {
                user: user.pubkey(),
                market: self.market,
                user_balance: user_balance_pda,
                user_token_account,
                vault_token_account,
                mint,
                token_program: anchor_spl::token::ID,
            }
            .to_account_metas(None),
            data: clob::instruction::Withdraw {
                params: WithdrawParams { amount },
            }
            .data(),
        };

        ctx.submit_transaction(&[ix], &[user])
    }

    pub async fn close_user_balance(&self, user: &Keypair) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        let (user_balance_pda, _) = get_user_balance_pda(&user.pubkey(), &self.market);

        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::CloseUserBalance {
                market: self.market,
                user_balance: user_balance_pda,
                user: user.pubkey(),
            }
            .to_account_metas(None),
            data: clob::instruction::CloseUserBalance {}.data(),
        };

        ctx.submit_transaction(&[ix], &[user])
    }
}

pub fn get_user_balance_pda(user: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_balance", user.as_ref(), market.as_ref()],
        &clob::ID,
    )
}

pub fn get_vault_pda(market: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", market.as_ref(), mint.as_ref()], &clob::ID)
}
