use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,  // Event queue for fill events
    pub base_lot_size: u64,   // Minimum base asset unit size
    pub quote_tick_size: u64, // Minimum quote asset price tick size
    pub next_order_id: u64,   // Auto-incrementing order ID counter
    pub bump: u8,
}
