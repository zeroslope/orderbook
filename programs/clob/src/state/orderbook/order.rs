use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    InitSpace,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Copy,
    Pod,
    Zeroable,
)]
#[repr(C)]
pub struct Order {
    pub order_id: u64,           // Unique order identifier
    pub owner: Pubkey,           // Order owner's public key
    pub price: u64,              // Price in quote_tick_size units
    pub quantity: u64,           // Original quantity in base_lot_size units
    pub remaining_quantity: u64, // Remaining unfilled quantity
    pub timestamp: i64,          // Creation timestamp for price-time priority
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // higher price first, then earlier timestamp for price-time priority
        match self.price.cmp(&other.price) {
            std::cmp::Ordering::Equal => other.timestamp.cmp(&self.timestamp),
            price_ord => price_ord,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Bid, // Buy orders
    Ask, // Sell orders
}

#[derive(
    AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq, Default,
)]
#[repr(u8)]
pub enum TimeInForce {
    #[default]
    GTC = 0, // Good-Till-Cancelled: Order remains active until explicitly cancelled
    IOC = 1, // Immediate-Or-Cancel: Execute immediately, cancel any unfilled portion
    FOK = 2, // Fill-Or-Kill: Either fill the entire order immediately or cancel it completely
}

// Trade execution result
#[derive(Debug, Clone)]
pub struct Fill {
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub maker_owner: Pubkey,
    pub maker_side: Side,
    pub price: u64,
    pub quantity: u64,
}
