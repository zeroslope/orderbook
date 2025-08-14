use crate::errors::ErrorCode;
use crate::events::MarketInitialized;
use crate::state::{AskSide, BidSide, Market};
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

    #[account(zero)]
    pub bids: AccountLoader<'info, BidSide>,
    #[account(zero)]
    pub asks: AccountLoader<'info, AskSide>,

    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_lot_size: u64,   // Minimum base asset unit size
    pub quote_tick_size: u64, // Minimum quote asset price tick size
}

impl Initialize<'_> {
    pub fn apply(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        // Validate that base and quote mints are different
        require!(
            params.base_mint != params.quote_mint,
            ErrorCode::SameMintAddresses
        );

        // Validate orderbook parameters
        require!(params.base_lot_size > 0, ErrorCode::InvalidParameter);
        require!(params.quote_tick_size > 0, ErrorCode::InvalidParameter);

        // Initialize bids book
        let _bids = &mut ctx.accounts.bids.load_init()?;
        // Initialize asks book
        let _asks = &mut ctx.accounts.asks.load_init()?;

        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.base_mint = params.base_mint;
        market.quote_mint = params.quote_mint;
        market.base_vault = ctx.accounts.base_vault.key();
        market.quote_vault = ctx.accounts.quote_vault.key();
        market.asks = ctx.accounts.asks.key();
        market.bids = ctx.accounts.bids.key();
        market.base_lot_size = params.base_lot_size;
        market.quote_tick_size = params.quote_tick_size;
        market.next_order_id = 1; // Start order IDs from 1
        market.bump = ctx.bumps.market;

        // Emit market initialized event
        emit!(MarketInitialized {
            market: market.key(),
            authority: market.authority,
            base_mint: market.base_mint,
            quote_mint: market.quote_mint,
            base_lot_size: market.base_lot_size,
            quote_tick_size: market.quote_tick_size,
        });

        msg!(
            "Market initialized: base={}, quote={}, bids={}, asks={}",
            params.base_mint,
            params.quote_mint,
            ctx.accounts.bids.key(),
            ctx.accounts.asks.key()
        );

        Ok(())
    }
}
