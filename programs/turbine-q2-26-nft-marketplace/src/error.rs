use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Marketplace name cannot be empty")]
    EmptyName,

    #[msg("Marketplace name is too long")]
    NameTooLong,

    #[msg("Invalid fee basis points")]
    InvalidFee,

    #[msg("Price must be greater than zero")]
    InvalidPrice,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Treasury has insufficient funds")]
    InsufficientTreasuryFunds,
}
