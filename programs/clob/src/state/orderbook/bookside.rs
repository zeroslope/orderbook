use super::{order::Side, vec_orderbook::VecOrderBook};
use anchor_lang::prelude::*;

#[account]
pub struct BookSide {
    pub market: Pubkey,          // Associated market
    pub orderbook: VecOrderBook, // Orders for this side
    pub bump: u8,
}

impl anchor_lang::Space for BookSide {
    const INIT_SPACE: usize = 32 + VecOrderBook::INIT_SPACE + 1; // market + orderbook + bump
}

impl BookSide {
    pub fn new(market: Pubkey, side: Side, bump: u8) -> Self {
        Self {
            market,
            orderbook: VecOrderBook::new(side),
            bump,
        }
    }
}
