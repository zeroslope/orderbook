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
}
