use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use clob::instructions::*;
use clob::state::{orderbook::OrderBook, Side};
use litesvm::types::TransactionResult;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction::create_account;
use std::{cell::RefCell, rc::Rc};

use super::{spl::MintFixture, SvmContext};

pub struct MarketFixture {
    ctx: Rc<RefCell<SvmContext>>,
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
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

        // Step 1: Create bids, asks, and event_queue accounts manually using fresh keypairs
        let bids_keypair = Keypair::new();
        let asks_keypair = Keypair::new();
        let event_queue_keypair = Keypair::new();

        let bids_size = 8 + std::mem::size_of::<clob::state::BidSide>();
        let asks_size = 8 + std::mem::size_of::<clob::state::AskSide>();
        let event_queue_size = 8 + std::mem::size_of::<clob::state::EventQueue>();

        let bids_rent = ctx.minimum_balance_for_rent_exemption(bids_size);
        let asks_rent = ctx.minimum_balance_for_rent_exemption(asks_size);
        let event_queue_rent = ctx.minimum_balance_for_rent_exemption(event_queue_size);

        let create_bids_ix = create_account(
            &authority,
            &bids_keypair.pubkey(),
            bids_rent,
            bids_size as u64,
            &clob::ID,
        );

        let create_asks_ix = create_account(
            &authority,
            &asks_keypair.pubkey(),
            asks_rent,
            asks_size as u64,
            &clob::ID,
        );

        let create_event_queue_ix = create_account(
            &authority,
            &event_queue_keypair.pubkey(),
            event_queue_rent,
            event_queue_size as u64,
            &clob::ID,
        );

        ctx.submit_transaction(
            &[create_bids_ix, create_asks_ix, create_event_queue_ix],
            &[&bids_keypair, &asks_keypair, &event_queue_keypair],
        )
        .expect("Failed to create orderbook accounts");

        // Update the addresses to use the created accounts
        let bids = bids_keypair.pubkey();
        let asks = asks_keypair.pubkey();
        let event_queue = event_queue_keypair.pubkey();

        // Step 2: Initialize market (with order books)
        let init_ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::Initialize {
                authority,
                market,
                base_vault,
                quote_vault,
                base_mint: base_mint.mint,
                quote_mint: quote_mint.mint,
                bids,
                asks,
                event_queue,
                base_token_program: anchor_spl::token::ID,
                quote_token_program: anchor_spl::token::ID,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: clob::instruction::Initialize {
                params: InitializeParams {
                    base_mint: base_mint.mint,
                    quote_mint: quote_mint.mint,
                    base_lot_size: 1_000_000, // 1.0 base token
                    quote_tick_size: 1_000,   // 0.001 quote token
                },
            }
            .data(),
        };

        ctx.submit_transaction(&[init_ix], &[])
            .expect("Failed to initialize market");

        Self {
            ctx: ctx_ref,
            market,
            base_mint: base_mint.mint,
            quote_mint: quote_mint.mint,
            base_vault,
            quote_vault,
            bids,
            asks,
            event_queue,
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

    pub async fn place_limit_order(
        &self,
        user: &Keypair,
        side: Side,
        price: u64,
        quantity: u64,
    ) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        let (user_balance_pda, _) = get_user_balance_pda(&user.pubkey(), &self.market);

        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::PlaceLimitOrder {
                market: self.market,
                bids: self.bids,
                asks: self.asks,
                event_queue: self.event_queue,
                user_balance: user_balance_pda,
                base_vault: self.base_vault,
                quote_vault: self.quote_vault,
                user: user.pubkey(),
                base_token_program: anchor_spl::token::ID,
                quote_token_program: anchor_spl::token::ID,
            }
            .to_account_metas(None),
            data: clob::instruction::PlaceLimitOrder {
                params: PlaceLimitOrderParams {
                    side,
                    price,
                    quantity,
                },
            }
            .data(),
        };

        ctx.submit_transaction(&[ix], &[user])
    }

    pub async fn cancel_order(
        &self,
        user: &Keypair,
        order_id: u64,
        side: Side,
    ) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        let (user_balance_pda, _) = get_user_balance_pda(&user.pubkey(), &self.market);

        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::CancelOrder {
                market: self.market,
                bids: self.bids,
                asks: self.asks,
                user_balance: user_balance_pda,
                user: user.pubkey(),
            }
            .to_account_metas(None),
            data: clob::instruction::CancelOrder {
                params: CancelOrderParams { order_id, side },
            }
            .data(),
        };

        ctx.submit_transaction(&[ix], &[user])
    }

    pub async fn consume_events(&self, limit: u8, maker_users: &[&Keypair]) -> TransactionResult {
        let mut ctx = self.ctx.borrow_mut();

        // Collect maker user balance PDAs
        let mut remaining_accounts = Vec::new();
        for maker_user in maker_users.iter() {
            let (user_balance_pda, _) = get_user_balance_pda(&maker_user.pubkey(), &self.market);
            remaining_accounts.push(AccountMeta::new(user_balance_pda, false));
        }

        let ix = Instruction {
            program_id: clob::ID,
            accounts: clob::accounts::ConsumeEvents {
                market: self.market,
                event_queue: self.event_queue,
            }
            .to_account_metas(None),
            data: clob::instruction::ConsumeEvents {
                params: ConsumeEventsParams { limit },
            }
            .data(),
        };

        // Append remaining accounts for maker balance updates
        let mut final_ix = ix;
        final_ix.accounts.extend(remaining_accounts);

        ctx.submit_transaction(&[final_ix], &[])
    }

    pub fn get_user_balance(&self, user: &Pubkey) -> clob::state::UserBalance {
        let (user_balance_pda, _) = get_user_balance_pda(user, &self.market);
        self.ctx.borrow().load_and_deserialize(&user_balance_pda)
    }

    pub fn get_bids_orderbook(&self) -> clob::state::BidSide {
        self.ctx.borrow().load_and_deserialize(&self.bids)
    }

    pub fn get_asks_orderbook(&self) -> clob::state::AskSide {
        self.ctx.borrow().load_and_deserialize(&self.asks)
    }

    pub fn find_order_in_bids(&self, order_id: u64) -> Option<clob::state::Order> {
        let bids = self.get_bids_orderbook();
        bids.orderbook.find_order_by_id(order_id)
    }

    pub fn find_order_in_asks(&self, order_id: u64) -> Option<clob::state::Order> {
        let asks = self.get_asks_orderbook();
        asks.orderbook.find_order_by_id(order_id)
    }

    pub fn get_orderbook_order_count(&self, side: clob::state::Side) -> usize {
        match side {
            clob::state::Side::Bid => self.get_bids_orderbook().orderbook.len(),
            clob::state::Side::Ask => self.get_asks_orderbook().orderbook.len(),
        }
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
