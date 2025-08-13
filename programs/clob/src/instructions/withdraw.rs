use crate::errors::ErrorCode;
use crate::state::{Market, UserBalance};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [b"user_balance", user.key().as_ref(), market.key().as_ref()],
        bump = user_balance.bump,
        constraint = user_balance.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_balance: Account<'info, UserBalance>,

    #[account(
        mut,
        token::mint = mint
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        constraint = mint.key() == market.base_mint || mint.key() == market.quote_mint,
        mint::token_program = token_program
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct WithdrawParams {
    pub amount: u64,
}

impl Withdraw<'_> {
    pub fn apply(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        require!(params.amount > 0, ErrorCode::InvalidAmount);

        let user_balance = &mut ctx.accounts.user_balance;
        let market = &ctx.accounts.market;

        // Check and update user balance record
        if ctx.accounts.mint.key() == market.base_mint {
            require!(
                user_balance.base_balance >= params.amount,
                ErrorCode::InsufficientBalance
            );
            user_balance.base_balance = user_balance
                .base_balance
                .checked_sub(params.amount)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            require!(
                user_balance.quote_balance >= params.amount,
                ErrorCode::InsufficientBalance
            );
            user_balance.quote_balance = user_balance
                .quote_balance
                .checked_sub(params.amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        // Transfer tokens from vault to user using checked transfer
        let seeds: &[&[u8]] = &[
            b"market".as_ref(),
            ctx.accounts.market.base_mint.as_ref(),
            ctx.accounts.market.quote_mint.as_ref(),
            &[ctx.accounts.market.bump],
        ];

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                &[seeds],
            ),
            params.amount,
            ctx.accounts.mint.decimals,
        )?;

        msg!(
            "Withdrawn {} tokens of mint {} from market vault",
            params.amount,
            ctx.accounts.mint.key()
        );

        Ok(())
    }
}
