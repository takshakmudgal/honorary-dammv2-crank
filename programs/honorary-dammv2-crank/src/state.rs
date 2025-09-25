use anchor_lang::prelude::*;

pub const OWNER_PDA_SEED: &[u8] = b"investor_fee_pos_owner";
pub const VAULT_SEED: &[u8] = b"vault";

#[account]
pub struct Config {
    pub authority: Pubkey,
    pub investor_fee_share_bps: u16,
    pub y0: u64,
    pub last_distribution_ts: i64,
    pub carry_over: u64,
    pub progress_cursor: u32,
    pub cumulative_distributed_on_day: u64,
    pub min_payout_lamports: u64,
}

#[account(zero_copy)]
pub struct HonoraryPosition {
    pub pool: Pubkey,
    pub position_pda: Pubkey,
    pub owner_pda: Pubkey,
    pub quote_mint: Pubkey,
    pub bump: u8,
    _padding: [u8; 7],
}

#[event]
pub struct HonoraryPositionInitialized {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner_pda: Pubkey,
    pub quote_mint: Pubkey,
}

#[event]
pub struct QuoteFeesClaimed {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub claimed_to: Pubkey,
}

#[event]
pub struct InvestorPayoutPage {
    pub stream: Pubkey,
    pub investor_ata: Pubkey,
    pub amount: u64,
    pub page_cursor: u32,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub pool: Pubkey,
    pub creator_ata: Pubkey,
    pub amount: u64,
    pub ts: i64,
}
