use crate::errors::ErrorCode;
use crate::state::{Market, UserBalance};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserBalance::INIT_SPACE,
        seeds = [b"user_balance", user.key().as_ref(), market.key().as_ref()],
        bump
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
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DepositParams {
    pub amount: u64,
}

impl Deposit<'_> {
    pub fn apply(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        require!(params.amount > 0, ErrorCode::InvalidAmount);

        let user_balance = &mut ctx.accounts.user_balance;
        let market = &ctx.accounts.market;
        // Initialize user balance if it's first time
        if user_balance.owner == Pubkey::default() {
            user_balance.owner = ctx.accounts.user.key();
            user_balance.market = market.key();
            user_balance.base_balance = 0;
            user_balance.quote_balance = 0;
            user_balance.bump = ctx.bumps.user_balance;
        }

        // Transfer tokens from user to vault using checked transfer
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token_interface::transfer_checked(cpi_ctx, params.amount, ctx.accounts.mint.decimals)?;

        // Update user balance record
        if ctx.accounts.mint.key() == market.base_mint {
            user_balance.base_balance = user_balance
                .base_balance
                .checked_add(params.amount)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            user_balance.quote_balance = user_balance
                .quote_balance
                .checked_add(params.amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        msg!(
            "Deposited {} tokens of mint {} to market vault",
            params.amount,
            ctx.accounts.mint.key()
        );

        Ok(())
    }
}
