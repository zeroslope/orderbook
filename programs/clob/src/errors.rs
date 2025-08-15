use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Math operation overflow")]
    MathOverflow,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Same mint addresses")]
    SameMintAddresses,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid parameter")]
    InvalidParameter,
    #[msg("Order not found")]
    OrderNotFound,
    #[msg("Orderbook full")]
    OrderbookFull,
    #[msg("Invalid order size")]
    InvalidOrderSize,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Event queue is full")]
    EventQueueFull,
    #[msg("Event queue is empty")]
    EventQueueEmpty,
    #[msg("Fill-or-kill order not completely filled")]
    FillOrKillNotFilled,
}
