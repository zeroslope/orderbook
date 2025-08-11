use anchor_lang::prelude::*;

declare_id!("FpTyzdMqQS4NWM149ryMWq74waAoHXMBpJnXb4yUNV1F");

#[program]
pub mod orderbook {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
