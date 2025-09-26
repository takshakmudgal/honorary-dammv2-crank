use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use solana_program::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use std::str::FromStr;

pub mod error;
pub use error::*;
pub mod state;
pub use state::*;

declare_id!("FEEPosCrank11111111111111111111111111111111");

pub const DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";

#[program]
pub mod honorary_dammv2_crank {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        y0: u64,
        investor_fee_share_bps: u16,
        min_payout_lamports: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = *ctx.accounts.authority.key;
        cfg.y0 = y0;
        cfg.investor_fee_share_bps = investor_fee_share_bps;
        cfg.last_distribution_ts = 0;
        cfg.carry_over = 0;
        cfg.progress_cursor = 0;
        cfg.cumulative_distributed_on_day = 0;
        cfg.min_payout_lamports = min_payout_lamports;
        Ok(())
    }

    pub fn create_honorary_position(
        ctx: Context<CreateHonorary>,
        _pool_pubkey: Pubkey,
        quote_mint: Pubkey,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.quote_mint.key(),
            quote_mint,
            ErrorCode::QuoteMintMismatch
        );

        let (owner_pda, _bump) = Pubkey::find_program_address(
            &[OWNER_PDA_SEED, ctx.accounts.vault.key().as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            owner_pda,
            ctx.accounts.owner_pda.key(),
            ErrorCode::InvalidOwnerPda
        );

        let damm_program = Pubkey::from_str(DAMM_V2_PROGRAM_ID)
            .map_err(|_| Error::from(ErrorCode::InvalidDammId))?;
        let discrim = &hash(b"global:create_position").to_bytes()[..8];

        // order of AccountMeta must match DAMM's create_position accounts
        let ix = Instruction {
            program_id: damm_program,
            accounts: vec![
                AccountMeta::new_readonly(ctx.accounts.owner_pda.key(), false),
                AccountMeta::new(ctx.remaining_accounts[0].key(), true), // position_nft_mint (must be signer)
                AccountMeta::new(ctx.remaining_accounts[1].key(), false), // position_nft_account
                AccountMeta::new(ctx.accounts.pool.key(), false),
                AccountMeta::new(ctx.remaining_accounts[2].key(), false), // position PDA
                AccountMeta::new(ctx.remaining_accounts[3].key(), false), // pool_authority
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new_readonly(ctx.remaining_accounts[4].key(), false), // token_program
                AccountMeta::new_readonly(ctx.remaining_accounts[5].key(), false), // system_program
            ],
            data: discrim.to_vec(),
        };

        // invoke signed (owner_pda not required to sign create_position, but keep pattern)
        let mut acct_infos = vec![
            ctx.accounts.owner_pda.to_account_info().clone(),
            ctx.remaining_accounts[0].clone(),
            ctx.remaining_accounts[1].clone(),
            ctx.accounts.pool.to_account_info().clone(),
            ctx.remaining_accounts[2].clone(),
            ctx.remaining_accounts[3].clone(),
            ctx.accounts.payer.to_account_info().clone(),
            ctx.remaining_accounts[4].clone(),
            ctx.remaining_accounts[5].clone(),
        ];

        let bump = *ctx.bumps.get("owner_pda").unwrap();
        let seeds = &[OWNER_PDA_SEED, ctx.accounts.vault.key().as_ref(), &[bump]];
        invoke_signed(
            &ix,
            &acct_infos,
            &[&[seeds.concat().as_slice()] /*wrapped below*/],
        )?;

        // initialize HonoraryPosition account
        let mut pos = ctx.accounts.honorary_position.load_init()?;
        pos.pool = ctx.accounts.pool.key();
        pos.position_pda = ctx.remaining_accounts[2].key();
        pos.owner_pda = ctx.accounts.owner_pda.key();
        pos.quote_mint = quote_mint;
        pos.bump = ctx.bumps["honorary_position"];
        emit!(HonoraryPositionInitialized {
            pool: pos.pool,
            position: pos.position_pda,
            owner_pda: pos.owner_pda,
            quote_mint,
        });
        Ok(())
    }

    /// Crank distribute. Per-page invocation. Client must pass per-page stream/investor pairs as remaining_accounts after fixed accounts.
    /// Fixed required accounts (in context): owner_pda(signer), position, pool, position_quote_ata, creator_quote_ata, config, token_program, system_program
    pub fn crank_distribute(
        ctx: Context<Crank>,
        page_total_locked: u64,
        page_cursor: u32,
        is_final_page: bool,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let cfg = &mut ctx.accounts.config;

        // enforce 24h gating: if last_distribution_ts is zero treat as first day allowed
        if cfg.last_distribution_ts != 0 {
            require!(
                clock.unix_timestamp >= cfg.last_distribution_ts + 86400,
                ErrorCode::TooSoonForNewDay
            );
        }

        // CPI into DAMM claim_position_fee -> expecting claimed tokens land in position_quote_ata
        let damm_program = Pubkey::from_str(DAMM_V2_PROGRAM_ID)
            .map_err(|_| Error::from(ErrorCode::InvalidDammId))?;
        let claim_disc = &hash(b"global:claim_position_fee").to_bytes()[..8];
        let ix = Instruction {
            program_id: damm_program,
            accounts: vec![
                AccountMeta::new_readonly(ctx.accounts.owner_pda.key(), true),
                AccountMeta::new(ctx.accounts.position.key(), false),
                AccountMeta::new(ctx.accounts.pool.key(), false),
                AccountMeta::new(ctx.accounts.position_quote_ata.key(), false),
                AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            ],
            data: claim_disc.to_vec(),
        };

        // signer seeds
        let bump = ctx.accounts.owner_pda_bump;
        let signer_seeds: &[&[u8]] =
            &[&[OWNER_PDA_SEED, ctx.accounts.vault.key().as_ref(), &[bump]]];

        let acct_infos = &[
            ctx.accounts.owner_pda.to_account_info().clone(),
            ctx.accounts.position.to_account_info().clone(),
            ctx.accounts.pool.to_account_info().clone(),
            ctx.accounts.position_quote_ata.to_account_info().clone(),
            ctx.accounts.token_program.to_account_info().clone(),
            ctx.accounts.system_program.to_account_info().clone(),
        ];
        invoke_signed(&ix, acct_infos, &[signer_seeds])?;
        emit!(QuoteFeesClaimed {
            pool: ctx.accounts.pool.key(),
            position: ctx.accounts.position.key(),
            claimed_to: ctx.accounts.position_quote_ata.key(),
        });

        let claimed_amount = ctx.accounts.position_quote_ata.amount;
        require!(claimed_amount > 0, ErrorCode::NoQuoteFeesClaimed);

        // compute eligible bps
        let f_locked_bps = if cfg.y0 == 0 {
            0
        } else {
            let bps = page_total_locked
                .saturating_mul(10_000u64)
                .checked_div(cfg.y0)
                .unwrap_or(0);
            core::cmp::min(cfg.investor_fee_share_bps as u64, bps) as u16
        };

        let investor_fee_quote = (claimed_amount as u128)
            .checked_mul(f_locked_bps as u128)
            .unwrap()
            .checked_div(10_000u128)
            .unwrap() as u64;

        // distribute across page pairs: remaining_accounts: [ stream_pubkey_i, investor_ata_i, stream_pubkey_j, investor_ata_j, ... ]
        let rem = &ctx.remaining_accounts;
        require!(rem.len() % 2 == 0, ErrorCode::InvalidPaginationAccounts);

        let mut distributed_on_page: u64 = 0;
        for i in (0..rem.len()).step_by(2) {
            let stream_pub = rem[i].key();
            let inv_ata_info = rem[i + 1].clone();

            // TODO: replace stub with real Streamflow parse
            let locked_i = get_locked_amount_from_streamflow_stub(stream_pub)?;
            if locked_i == 0 {
                continue;
            }

            let share = (investor_fee_quote as u128)
                .checked_mul(locked_i as u128)
                .unwrap()
                .checked_div(page_total_locked as u128)
                .unwrap() as u64;

            if share < cfg.min_payout_lamports {
                cfg.carry_over = cfg.carry_over.saturating_add(share);
                continue;
            }

            // transfer share from position_quote_ata -> investor_ata using owner_pda signer
            let cpi_accounts = Transfer {
                from: ctx.accounts.position_quote_ata.to_account_info().clone(),
                to: inv_ata_info.clone(),
                authority: ctx.accounts.owner_pda.to_account_info().clone(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &[signer_seeds]);
            token::transfer(cpi_ctx, share)?;
            distributed_on_page = distributed_on_page.saturating_add(share);
            emit!(InvestorPayoutPage {
                stream: stream_pub,
                investor_ata: inv_ata_info.key(),
                amount: share,
                page_cursor,
            });
        }

        if is_final_page {
            let remaining = claimed_amount
                .saturating_sub(distributed_on_page)
                .saturating_sub(cfg.carry_over);
            if remaining > 0 {
                // transfer to creator
                let cpi_accounts = Transfer {
                    from: ctx.accounts.position_quote_ata.to_account_info().clone(),
                    to: ctx.accounts.creator_quote_ata.to_account_info().clone(),
                    authority: ctx.accounts.owner_pda.to_account_info().clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx =
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, &[signer_seeds]);
                token::transfer(cpi_ctx, remaining)?;
            }
            cfg.last_distribution_ts = clock.unix_timestamp;
            cfg.cumulative_distributed_on_day = cfg
                .cumulative_distributed_on_day
                .saturating_add(distributed_on_page);
            cfg.carry_over = 0;
            emit!(CreatorPayoutDayClosed {
                pool: ctx.accounts.pool.key(),
                creator_ata: ctx.accounts.creator_quote_ata.key(),
                amount: remaining,
                ts: cfg.last_distribution_ts,
            });
        } else {
            cfg.progress_cursor = page_cursor;
            cfg.cumulative_distributed_on_day = cfg
                .cumulative_distributed_on_day
                .saturating_add(distributed_on_page);
        }

        Ok(())
    }
}

// Stub: replace with Streamflow parsing
fn get_locked_amount_from_streamflow_stub(_stream_pub: Pubkey) -> Result<u64> {
    // TODO parse Streamflow stream account to compute still-locked amount at now
    Ok(1)
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 64)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateHonorary<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// vault is arbitrary seed owner, used to derive owner_pda
    pub vault: UncheckedAccount<'info>,
    /// PDA that will own the DAMM position
    /// CHECK
    #[account(mut, seeds = [OWNER_PDA_SEED, vault.key().as_ref()], bump)]
    pub owner_pda: UncheckedAccount<'info>,
    #[account(init_if_needed, payer = payer, space = 8 + std::mem::size_of::<HonoraryPosition>())]
    pub honorary_position: AccountLoader<'info, HonoraryPosition>,
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// token mint to validate quote token
    pub quote_mint: Account<'info, Mint>,
    #[account(mut)]
    pub payer_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub honorary_position_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// system + token programs are passed in remaining_accounts as well for invoke if desired
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Crank<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    /// owner PDA that signs token transfers from position_quote_ata
    #[account(mut, seeds = [OWNER_PDA_SEED, vault.key().as_ref()], bump = owner_pda_bump)]
    pub owner_pda: UncheckedAccount<'info>,
    pub vault: UncheckedAccount<'info>,
    pub owner_pda_bump: u8,
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    #[account(mut)]
    pub position_quote_ata: Account<'info, TokenAccount>, // where DAMM claim deposits quote tokens
    #[account(mut)]
    pub creator_quote_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
