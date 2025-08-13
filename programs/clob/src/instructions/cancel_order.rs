use crate::errors::ErrorCode;
use crate::state::{Market, BookSide, UserBalance, OrderBook, Side};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(params: CancelOrderParams)]
pub struct CancelOrder<'info> {
    #[account(
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

    pub user: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CancelOrderParams {
    pub order_id: u64,
    pub side: Side,  // Specify which orderbook to search
}

impl CancelOrder<'_> {
    pub fn apply(ctx: Context<CancelOrder>, params: CancelOrderParams) -> Result<()> {
        let market = &ctx.accounts.market;
        let user_balance = &mut ctx.accounts.user_balance;

        // Try to remove order from the specified orderbook
        let removed_order = match params.side {
            Side::Bid => ctx.accounts.bids_book.orderbook.remove_order(params.order_id)?,
            Side::Ask => ctx.accounts.asks_book.orderbook.remove_order(params.order_id)?,
        };

        let order = removed_order.ok_or(ErrorCode::InvalidParameter)?; // Order not found

        // Verify the order belongs to the user
        require!(order.owner == ctx.accounts.user.key(), ErrorCode::Unauthorized);

        // Return reserved funds to user balance
        match params.side {
            Side::Bid => {
                // Return reserved quote tokens
                let reserved_quote = order.price
                    .checked_mul(order.remaining_quantity)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_mul(market.quote_tick_size)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(market.base_lot_size)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                user_balance.quote_balance = user_balance.quote_balance
                    .checked_add(reserved_quote)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
            Side::Ask => {
                // Return reserved base tokens
                let reserved_base = order.remaining_quantity
                    .checked_mul(market.base_lot_size)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                user_balance.base_balance = user_balance.base_balance
                    .checked_add(reserved_base)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }

        msg!(
            "Order cancelled: id={}, remaining_quantity={}",
            order.order_id,
            order.remaining_quantity
        );

        Ok(())
    }
}