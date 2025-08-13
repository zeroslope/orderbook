use anchor_lang::prelude::*;
use crate::state::orderbook::order::Side;

#[event]
pub struct OrderPlaced {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

#[event]
pub struct OrderFilled {
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub market: Pubkey,
    pub price: u64,
    pub quantity: u64,
    pub maker_owner: Pubkey,
    pub taker_owner: Pubkey,
}

#[event]
pub struct OrderCancelled {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: Side,
    pub remaining_quantity: u64,
}

#[event]
pub struct MarketInitialized {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_lot_size: u64,
    pub quote_tick_size: u64,
}

#[event]
pub struct UserDeposit {
    pub user: Pubkey,
    pub market: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
}

#[event]
pub struct UserWithdraw {
    pub user: Pubkey,
    pub market: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
}