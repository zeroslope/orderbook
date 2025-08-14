use crate::errors::ErrorCode;
use crate::state::{EventQueue, FillEvent, Market, UserBalance};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ConsumeEvents<'info> {
    #[account(
        seeds = [b"market", market.base_mint.as_ref(), market.quote_mint.as_ref()],
        bump = market.bump,
        has_one = event_queue,
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,
    // remaining_accounts: maker user balance accounts to update
    // Each account should be a mutable UserBalance PDA for the maker owner
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConsumeEventsParams {
    pub limit: u8, // Maximum number of events to process
}

impl ConsumeEvents<'_> {
    pub fn apply(ctx: Context<ConsumeEvents>, params: ConsumeEventsParams) -> Result<()> {
        let mut event_queue = ctx.accounts.event_queue.load_mut()?;
        let market = &ctx.accounts.market;

        let mut processed = 0;

        // Process events sequentially in order
        while !event_queue.is_empty() && processed < params.limit {
            let event = event_queue.pop_event()?;

            // Find the account for this maker
            let mut found_account = None;
            for account_info in ctx.remaining_accounts.iter() {
                // Verify this is the correct UserBalance PDA for this maker
                let (expected_pda, _) = Pubkey::find_program_address(
                    &[
                        b"user_balance",
                        event.maker_owner.as_ref(),
                        market.key().as_ref(),
                    ],
                    &crate::ID,
                );

                if account_info.key() == expected_pda {
                    found_account = Some(account_info);
                    break;
                }
            }

            if let Some(account_info) = found_account {
                // Update maker balance
                Self::update_maker_balance(account_info, &event, market)?;
                processed += 1;
            } else {
                // We don't have the maker's account, stop processing
                break;
            }
        }

        msg!("Consumed {} events from queue", processed);
        Ok(())
    }

    fn update_maker_balance(
        account_info: &AccountInfo,
        event: &FillEvent,
        market: &Market,
    ) -> Result<()> {
        // Borrow the account data mutably
        let mut account_data = account_info.try_borrow_mut_data()?;

        // Deserialize UserBalance from the full account data (including discriminator)
        let mut user_balance = UserBalance::try_deserialize(&mut account_data.as_ref())?;

        let fill_base_amount = event
            .quantity
            .checked_mul(market.base_lot_size)
            .ok_or(ErrorCode::MathOverflow)?;

        let fill_quote_amount = event
            .price
            .checked_mul(event.quantity)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_mul(market.quote_tick_size)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(market.base_lot_size)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update maker balance based on their order side
        // Note: In place_limit_order, the maker's balance was already reserved/deducted
        // So in consume_events, we only need to apply the settlement:
        // - For bid makers: they already paid quote (reserved), now receive base
        // - For ask makers: they already paid base (reserved), now receive quote
        match event.maker_side {
            0 => {
                // Maker bid order filled: receive base (quote was already deducted in place_limit_order)
                user_balance.base_balance = user_balance
                    .base_balance
                    .checked_add(fill_base_amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                // Note: quote was already deducted when order was placed, no need to subtract again
            }
            1 => {
                // Maker ask order filled: receive quote (base was already deducted in place_limit_order)
                user_balance.quote_balance = user_balance
                    .quote_balance
                    .checked_add(fill_quote_amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                // Note: base was already deducted when order was placed, no need to subtract again
            }
            _ => return Err(ErrorCode::InvalidParameter.into()),
        }

        // Serialize the updated balance back to the account
        let mut cursor = std::io::Cursor::new(account_data.as_mut());
        user_balance.try_serialize(&mut cursor)?;

        Ok(())
    }
}
