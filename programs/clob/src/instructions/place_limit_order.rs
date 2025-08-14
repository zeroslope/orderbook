use crate::errors::ErrorCode;
use crate::events::{OrderFilled, OrderPlaced};
use crate::state::{
    AskSide, BidSide, EventQueue, FillEvent, Market, Order, OrderBook, Side, UserBalance,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(params: PlaceLimitOrderParams)]
pub struct PlaceLimitOrder<'info> {
    #[account(
        mut,
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump,
        has_one = bids,
        has_one = asks,
        has_one = event_queue,
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub bids: AccountLoader<'info, BidSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, AskSide>,
    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,

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

        let mut asks = ctx.accounts.asks.load_mut()?;
        let mut bids = ctx.accounts.bids.load_mut()?;

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
            Side::Bid => asks.orderbook.match_orders(&mut new_order)?,
            Side::Ask => bids.orderbook.match_orders(&mut new_order)?,
        };

        // Process fills: update taker balance immediately, queue events for maker balance updates
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

            // 1. Immediately update taker balance
            match params.side {
                Side::Bid => {
                    // Taker is bidding: receive base, pay quote
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
                    // Taker is asking: pay base, receive quote
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

            // 2. Push fill event to queue for maker balance processing
            let mut event_queue = ctx.accounts.event_queue.load_mut()?;
            let fill_event = FillEvent {
                maker_order_id: fill.maker_order_id,
                taker_order_id: fill.taker_order_id,
                price: fill.price,
                quantity: fill.quantity,
                timestamp: Clock::get()?.unix_timestamp,
                maker_owner: fill.maker_owner,
                taker_owner: ctx.accounts.user.key(),
                market: market.key(),
                maker_side: match fill.maker_side {
                    Side::Bid => 0,
                    Side::Ask => 1,
                },
                _padding: [0; 7],
            };
            event_queue.push_event(fill_event)?;

            // 3. Emit fill event
            emit!(OrderFilled {
                maker_order_id: fill.maker_order_id,
                taker_order_id: fill.taker_order_id,
                market: market.key(),
                price: fill.price,
                quantity: fill.quantity,
                maker_owner: fill.maker_owner,
                taker_owner: ctx.accounts.user.key(),
                taker_side: params.side,
            });
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

                    bids.orderbook.insert_order(new_order)?;
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

                    asks.orderbook.insert_order(new_order)?;
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
        }

        Ok(())
    }
}
