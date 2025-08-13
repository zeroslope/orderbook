use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

pub use errors::ErrorCode;
use instructions::*;

declare_id!("FpTyzdMqQS4NWM149ryMWq74waAoHXMBpJnXb4yUNV1F");

#[program]
pub mod orderbook {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        Initialize::apply(ctx, params)
    }

    pub fn deposit(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        Deposit::apply(ctx, params)
    }

    pub fn withdraw(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        Withdraw::apply(ctx, params)
    }

    pub fn close_user_balance(ctx: Context<CloseUserBalance>) -> Result<()> {
        CloseUserBalance::apply(ctx)
    }
}
