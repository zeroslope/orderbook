use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, PartialEq)]
pub struct Order {
    pub order_id: u64,           // Unique order identifier
    pub owner: Pubkey,           // Order owner's public key
    pub price: u64,              // Price in quote_tick_size units
    pub quantity: u64,           // Original quantity in base_lot_size units
    pub remaining_quantity: u64, // Remaining unfilled quantity
    pub timestamp: i64,          // Creation timestamp for price-time priority
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Bid, // Buy orders
    Ask, // Sell orders
}

// Trade execution result
#[derive(Debug, Clone)]
pub struct Fill {
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub price: u64,
    pub quantity: u64,
}
