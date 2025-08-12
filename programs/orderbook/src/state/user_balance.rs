use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserBalance {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub base_balance: u64,
    pub quote_balance: u64,
    pub bump: u8,
}
