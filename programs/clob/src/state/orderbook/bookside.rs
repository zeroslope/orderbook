use super::heap_orderbook::{AskOrderBook, BidOrderBook};
use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(Default)]
#[repr(C)]
pub struct AskSide {
    pub orderbook: AskOrderBook,
}

#[account(zero_copy)]
#[derive(Default)]
#[repr(C)]
pub struct BidSide {
    pub orderbook: BidOrderBook,
}
