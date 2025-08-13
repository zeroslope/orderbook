use crate::errors::ErrorCode;
use crate::events::MarketInitialized;
use crate::state::{Market, BookSide, VecOrderBook, Side};
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

    #[account(
        init,
        payer = authority,
        space = 8 + BookSide::INIT_SPACE,
        seeds = [b"bids", market.key().as_ref()],
        bump
    )]
    pub bids_book: Account<'info, BookSide>,

    #[account(
        init,
        payer = authority,
        space = 8 + BookSide::INIT_SPACE,
        seeds = [b"asks", market.key().as_ref()],
        bump
    )]
    pub asks_book: Account<'info, BookSide>,

    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_lot_size: u64,     // Minimum base asset unit size
    pub quote_tick_size: u64,   // Minimum quote asset price tick size
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

        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.base_mint = params.base_mint;
        market.quote_mint = params.quote_mint;
        market.base_vault = ctx.accounts.base_vault.key();
        market.quote_vault = ctx.accounts.quote_vault.key();
        market.base_lot_size = params.base_lot_size;
        market.quote_tick_size = params.quote_tick_size;
        market.next_order_id = 1; // Start order IDs from 1
        market.bump = ctx.bumps.market;

        // Initialize bids book
        let bids_book = &mut ctx.accounts.bids_book;
        bids_book.market = market.key();
        bids_book.orderbook = VecOrderBook::new(Side::Bid);
        bids_book.bump = ctx.bumps.bids_book;

        // Initialize asks book
        let asks_book = &mut ctx.accounts.asks_book;
        asks_book.market = market.key();
        asks_book.orderbook = VecOrderBook::new(Side::Ask);
        asks_book.bump = ctx.bumps.asks_book;

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
            "Market initialized: base={}, quote={}",
            params.base_mint,
            params.quote_mint
        );

        Ok(())
    }
}
