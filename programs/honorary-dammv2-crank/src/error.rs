use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Quote mint mismatch")]
    QuoteMintMismatch,
    #[msg("Invalid owner PDA")]
    InvalidOwnerPda,
    #[msg("Invalid DAMM id")]
    InvalidDammId,
    #[msg("Too soon for new day")]
    TooSoonForNewDay,
    #[msg("No quote fees claimed")]
    NoQuoteFeesClaimed,
    #[msg("Invalid pagination accounts")]
    InvalidPaginationAccounts,
}
