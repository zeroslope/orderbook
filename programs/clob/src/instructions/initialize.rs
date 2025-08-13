use crate::errors::ErrorCode;
use crate::state::Market;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(params: InitializeParams)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", params.base_mint.as_ref(), params.quote_mint.as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = authority,
        token::mint = base_mint,
        token::authority = market,
        token::token_program = base_token_program,
        seeds = [b"vault", market.key().as_ref(), base_mint.key().as_ref()],
        bump
    )]
    pub base_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = quote_mint,
        token::authority = market,
        token::token_program = quote_token_program,
        seeds = [b"vault", market.key().as_ref(), quote_mint.key().as_ref()],
        bump
    )]
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,

    pub base_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,

    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
}

impl Initialize<'_> {
    pub fn apply(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        // Validate that base and quote mints are different
        require!(
            params.base_mint != params.quote_mint,
            ErrorCode::SameMintAddresses
        );

        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.base_mint = params.base_mint;
        market.quote_mint = params.quote_mint;
        market.base_vault = ctx.accounts.base_vault.key();
        market.quote_vault = ctx.accounts.quote_vault.key();
        market.bump = ctx.bumps.market;

        msg!(
            "Market initialized: base={}, quote={}",
            params.base_mint,
            params.quote_mint
        );

        Ok(())
    }
}
