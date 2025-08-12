use crate::errors::ErrorCode;
use crate::state::{Market, UserBalance};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseUserBalance<'info> {
    #[account(
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        close = user,
        seeds = [b"user_balance", user.key().as_ref(), market.key().as_ref()],
        bump = user_balance.bump,
        constraint = user_balance.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_balance: Account<'info, UserBalance>,

    #[account(mut)]
    pub user: Signer<'info>,
}

impl CloseUserBalance<'_> {
    pub fn apply(ctx: Context<CloseUserBalance>) -> Result<()> {
        let user_balance = &ctx.accounts.user_balance;

        // Ensure balance is zero before closing
        require!(
            user_balance.base_balance == 0 && user_balance.quote_balance == 0,
            ErrorCode::InsufficientBalance
        );

        msg!("User balance closed for user: {}", ctx.accounts.user.key());

        Ok(())
    }
}
