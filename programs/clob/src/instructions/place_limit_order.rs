use crate::errors::ErrorCode;
use crate::events::{OrderFilled, OrderPlaced};
use crate::state::{BookSide, Market, Order, OrderBook, Side, UserBalance};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(params: PlaceLimitOrderParams)]
pub struct PlaceLimitOrder<'info> {
    #[account(
        mut,
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [b"bids", market.key().as_ref()],
        bump = bids_book.bump,
        constraint = bids_book.market == market.key() @ ErrorCode::InvalidParameter
    )]
    pub bids_book: Account<'info, BookSide>,

    #[account(
        mut,
        seeds = [b"asks", market.key().as_ref()],
        bump = asks_book.bump,
        constraint = asks_book.market == market.key() @ ErrorCode::InvalidParameter
    )]
    pub asks_book: Account<'info, BookSide>,

    #[account(
        mut,
        seeds = [b"user_balance", user.key().as_ref(), market.key().as_ref()],
        bump = user_balance.bump,
        constraint = user_balance.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_balance: Account<'info, UserBalance>,

    #[account(
        mut,
        constraint = base_vault.key() == market.base_vault @ ErrorCode::InvalidTokenMint
    )]
    pub base_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = quote_vault.key() == market.quote_vault @ ErrorCode::InvalidTokenMint
    )]
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,

    pub user: Signer<'info>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PlaceLimitOrderParams {
    pub side: Side,    // Buy or Sell
    pub price: u64,    // Price in quote_tick_size units
    pub quantity: u64, // Quantity in base_lot_size units
}

impl PlaceLimitOrder<'_> {
    pub fn apply(ctx: Context<PlaceLimitOrder>, params: PlaceLimitOrderParams) -> Result<()> {
        // Enhanced parameter validation
        require!(params.price > 0, ErrorCode::InvalidPrice);
        require!(params.quantity > 0, ErrorCode::InvalidOrderSize);

        // Check if orderbook has space for new order (if not fully matched)
        let orderbook_len = match params.side {
            Side::Bid => ctx.accounts.bids_book.orderbook.len(),
            Side::Ask => ctx.accounts.asks_book.orderbook.len(),
        };
        require!(orderbook_len < 50, ErrorCode::OrderbookFull); // Max 50 orders

        let market = &mut ctx.accounts.market;
        let user_balance = &mut ctx.accounts.user_balance;

        // Check if user has sufficient balance
        match params.side {
            Side::Bid => {
                let required_quote = params
                    .price
                    .checked_mul(params.quantity)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_mul(market.quote_tick_size)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(market.base_lot_size)
                    .ok_or(ErrorCode::MathOverflow)?;

                require!(
                    user_balance.quote_balance >= required_quote,
                    ErrorCode::InsufficientBalance
                );
            }
            Side::Ask => {
                let required_base = params
                    .quantity
                    .checked_mul(market.base_lot_size)
                    .ok_or(ErrorCode::MathOverflow)?;

                require!(
                    user_balance.base_balance >= required_base,
                    ErrorCode::InsufficientBalance
                );
            }
        }

        // Create new order
        let mut new_order = Order {
            order_id: market.next_order_id,
            owner: ctx.accounts.user.key(),
            price: params.price,
            quantity: params.quantity,
            remaining_quantity: params.quantity,
            timestamp: Clock::get()?.unix_timestamp,
        };

        // Increment order ID counter
        market.next_order_id = market
            .next_order_id
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;

        // Match against opposite side orderbook
        let fills = match params.side {
            Side::Bid => {
                // Bid order matches against asks
                ctx.accounts
                    .asks_book
                    .orderbook
                    .match_orders(&mut new_order)?
            }
            Side::Ask => {
                // Ask order matches against bids
                ctx.accounts
                    .bids_book
                    .orderbook
                    .match_orders(&mut new_order)?
            }
        };

        // Process fills and update balances
        for fill in fills.iter() {
            let fill_base_amount = fill
                .quantity
                .checked_mul(market.base_lot_size)
                .ok_or(ErrorCode::MathOverflow)?;

            let fill_quote_amount = fill
                .price
                .checked_mul(fill.quantity)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_mul(market.quote_tick_size)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(market.base_lot_size)
                .ok_or(ErrorCode::MathOverflow)?;

            match params.side {
                Side::Bid => {
                    // User is bidding: receive base, pay quote
                    user_balance.base_balance = user_balance
                        .base_balance
                        .checked_add(fill_base_amount)
                        .ok_or(ErrorCode::MathOverflow)?;

                    user_balance.quote_balance = user_balance
                        .quote_balance
                        .checked_sub(fill_quote_amount)
                        .ok_or(ErrorCode::InsufficientBalance)?;
                }
                Side::Ask => {
                    // User is asking: pay base, receive quote
                    user_balance.base_balance = user_balance
                        .base_balance
                        .checked_sub(fill_base_amount)
                        .ok_or(ErrorCode::InsufficientBalance)?;

                    user_balance.quote_balance = user_balance
                        .quote_balance
                        .checked_add(fill_quote_amount)
                        .ok_or(ErrorCode::MathOverflow)?;
                }
            }

            // Emit fill event
            emit!(OrderFilled {
                maker_order_id: fill.maker_order_id,
                taker_order_id: fill.taker_order_id,
                market: market.key(),
                price: fill.price,
                quantity: fill.quantity,
                maker_owner: Pubkey::default(), // We'll need to store this in the future
                taker_owner: ctx.accounts.user.key(),
            });

            msg!(
                "Order filled: maker_id={}, taker_id={}, price={}, quantity={}",
                fill.maker_order_id,
                fill.taker_order_id,
                fill.price,
                fill.quantity
            );
        }

        // If order still has remaining quantity, add to appropriate orderbook
        if new_order.remaining_quantity > 0 {
            // Reserve required balance for the remaining order
            match params.side {
                Side::Bid => {
                    let required_quote = new_order
                        .price
                        .checked_mul(new_order.remaining_quantity)
                        .ok_or(ErrorCode::MathOverflow)?
                        .checked_mul(market.quote_tick_size)
                        .ok_or(ErrorCode::MathOverflow)?
                        .checked_div(market.base_lot_size)
                        .ok_or(ErrorCode::MathOverflow)?;

                    user_balance.quote_balance = user_balance
                        .quote_balance
                        .checked_sub(required_quote)
                        .ok_or(ErrorCode::InsufficientBalance)?;

                    ctx.accounts
                        .bids_book
                        .orderbook
                        .insert_order(new_order.clone())?;
                }
                Side::Ask => {
                    let required_base = new_order
                        .remaining_quantity
                        .checked_mul(market.base_lot_size)
                        .ok_or(ErrorCode::MathOverflow)?;

                    user_balance.base_balance = user_balance
                        .base_balance
                        .checked_sub(required_base)
                        .ok_or(ErrorCode::InsufficientBalance)?;

                    ctx.accounts
                        .asks_book
                        .orderbook
                        .insert_order(new_order.clone())?;
                }
            }

            // Emit order placed event for remaining quantity
            emit!(OrderPlaced {
                order_id: new_order.order_id,
                owner: ctx.accounts.user.key(),
                market: market.key(),
                side: params.side,
                price: new_order.price,
                quantity: new_order.remaining_quantity,
                timestamp: new_order.timestamp,
            });

            msg!(
                "Order placed: id={}, side={:?}, price={}, quantity={}",
                new_order.order_id,
                params.side,
                new_order.price,
                new_order.remaining_quantity
            );
        }

        Ok(())
    }
}
