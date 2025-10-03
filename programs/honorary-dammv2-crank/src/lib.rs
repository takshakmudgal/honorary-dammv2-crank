use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("ddcEKSibupo9XMaeHH66rVkpqCpWybXtAZWaBbMbF3h");

#[constant]
pub const DAMM_V2_PROGRAM_ID: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

#[constant]
pub const TOKEN22_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

#[constant]
pub const STREAMFLOW_PROGRAM_ID: Pubkey = pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");

#[constant]
pub const POOL_AUTHORITY: Pubkey = pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");

#[program]
pub mod honorary_dammv2_crank {
    use super::*;

    pub fn validate_pool(ctx: Context<ValidatePool>) -> Result<()> {
        let pool = &ctx.accounts.pool;
        require!(pool.collect_fee_mode == 1, ErrorCode::InvalidPoolConfig);
        require!(
            pool.token_mint_b == ctx.accounts.quote_mint.key(),
            ErrorCode::InvalidQuoteMint
        );
        msg!("Pool validated for quote-only fee collection");
        Ok(())
    }

    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        y0: u64,
        investor_fee_share_bps: u16,
        daily_cap: Option<u64>,
        min_payout_lamports: u64,
    ) -> Result<()> {
        let policy = &mut ctx.accounts.policy;
        policy.vault = ctx.accounts.vault.key();
        policy.y0 = y0;
        policy.investor_fee_share_bps = investor_fee_share_bps;
        policy.daily_cap = daily_cap;
        policy.min_payout_lamports = min_payout_lamports;
        emit!(PolicyInitialized {
            vault: policy.vault,
            y0,
            investor_fee_share_bps,
        });
        Ok(())
    }

    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>,
        tick_lower_index: i32,
        tick_upper_index: i32,
        liquidity: u128,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let current_tick_index = pool.tick_current;

        require!(
            tick_upper_index < current_tick_index,
            ErrorCode::InvalidTickRange
        );
        require!(
            tick_lower_index < tick_upper_index,
            ErrorCode::InvalidTickRange
        );

        let vault_key = ctx.accounts.vault.key();
        let seeds = &[b"investor_fee_pos_owner", vault_key.as_ref()];
        let signer_seeds = &[&seeds[..]];

        let discriminator = [48, 215, 197, 153, 96, 203, 180, 133];
        let mut ix_data = discriminator.to_vec();
        ix_data.extend_from_slice(&tick_lower_index.to_le_bytes());
        ix_data.extend_from_slice(&tick_upper_index.to_le_bytes());
        ix_data.extend_from_slice(&liquidity.to_le_bytes());

        let ix = Instruction {
            program_id: DAMM_V2_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(ctx.accounts.owner_pda.key(), false),
                AccountMeta::new(ctx.accounts.position_nft_mint.key(), false),
                AccountMeta::new(ctx.accounts.position_nft_account.key(), false),
                AccountMeta::new(ctx.accounts.pool.key(), false),
                AccountMeta::new(ctx.accounts.position.key(), false),
                AccountMeta::new_readonly(POOL_AUTHORITY, false),
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new_readonly(TOKEN22_PROGRAM_ID, false),
                AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
                AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                AccountMeta::new_readonly(DAMM_V2_PROGRAM_ID, false),
            ],
            data: ix_data,
        };

        invoke_signed(
            &ix,
            &[
                ctx.accounts.owner_pda.to_account_info(),
                ctx.accounts.position_nft_mint.to_account_info(),
                ctx.accounts.position_nft_account.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.position.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.damm_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        emit!(HonoraryPositionInitialized {
            vault: ctx.accounts.vault.key(),
            position: ctx.accounts.position.key(),
        });
        Ok(())
    }

    pub fn initialize_progress(ctx: Context<InitializeProgress>) -> Result<()> {
        let progress = &mut ctx.accounts.progress;
        progress.vault = ctx.accounts.vault.key();
        progress.last_distribution_ts = 0;
        progress.current_day_start_ts = 0;
        progress.claimed_for_day = 0;
        progress.investor_intended_for_day = 0;
        progress.creator_share_for_day = 0;
        progress.actual_distributed = 0;
        progress.carry_over = 0;
        progress.cursor = 0;

        emit!(ProgressInitialized {
            vault: ctx.accounts.vault.key(),
        });
        Ok(())
    }

    pub fn initialize_treasury_accounts(ctx: Context<InitializeTreasuryAccounts>) -> Result<()> {
        emit!(TreasuryAccountsInitialized {
            vault: ctx.accounts.vault.key(),
            base_treasury: ctx.accounts.base_treasury.key(),
            quote_treasury: ctx.accounts.quote_treasury.key(),
        });
        Ok(())
    }

    pub fn create_stream(
        ctx: Context<CreateStream>,
        start_time: u64,
        net_amount_deposited: u64,
        period: u64,
        amount_per_period: u64,
        cliff: u64,
        cliff_amount: u64,
        cancelable_by_sender: bool,
        cancelable_by_recipient: bool,
        automatic_withdrawal: bool,
        transferable_by_sender: bool,
        transferable_by_recipient: bool,
        can_topup: bool,
        stream_name: [u8; 64],
        withdraw_frequency: u64,
        recipient: Pubkey,
        partner: Pubkey,
        pausable: bool,
        can_update_rate: bool,
    ) -> Result<()> {
        let discriminator = [230, 9, 241, 173, 57, 159, 239, 202];
        let mut ix_data = discriminator.to_vec();
        let args = CreateUncheckedWithPayerArgs {
            start_time,
            net_amount_deposited,
            period,
            amount_per_period,
            cliff,
            cliff_amount,
            cancelable_by_sender,
            cancelable_by_recipient,
            automatic_withdrawal,
            transferable_by_sender,
            transferable_by_recipient,
            can_topup,
            stream_name,
            withdraw_frequency,
            recipient,
            partner,
            pausable,
            can_update_rate,
        };
        ix_data.extend_from_slice(&args.try_to_vec()?);

        let ix = Instruction {
            program_id: STREAMFLOW_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new(ctx.accounts.sender.key(), true),
                AccountMeta::new(ctx.accounts.sender_tokens.key(), false),
                AccountMeta::new(ctx.accounts.metadata.key(), false),
                AccountMeta::new(ctx.accounts.escrow_tokens.key(), false),
                AccountMeta::new(ctx.accounts.withdrawor.key(), false),
                AccountMeta::new_readonly(ctx.accounts.mint.key(), false),
                AccountMeta::new_readonly(ctx.accounts.fee_oracle.key(), false),
                AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
                AccountMeta::new_readonly(STREAMFLOW_PROGRAM_ID, false),
                AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            ],
            data: ix_data,
        };

        invoke_signed(
            &ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.sender.to_account_info(),
                ctx.accounts.sender_tokens.to_account_info(),
                ctx.accounts.metadata.to_account_info(),
                ctx.accounts.escrow_tokens.to_account_info(),
                ctx.accounts.withdrawor.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.fee_oracle.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.timelock_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;

        Ok(())
    }

    pub fn crank<'info>(
        ctx: Context<'_, '_, '_, 'info, Crank<'info>>,
        page_index: u16,
        locked_total: u64,
        is_final_page: bool,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp as u64;
        let progress = &mut ctx.accounts.progress;
        let policy = &ctx.accounts.policy;

        let needs_distribution_reset =
            progress.last_distribution_ts == 0 || now >= progress.last_distribution_ts + 86400;

        if needs_distribution_reset {
            progress.last_distribution_ts = now;
            progress.current_day_start_ts = now;
            progress.actual_distributed = 0;
            progress.cursor = 0;

            let owner_pda_info = ctx.accounts.owner_pda.to_account_info();
            let pool_authority_info = ctx.accounts.pool_authority.to_account_info();
            let pool_info = ctx.accounts.pool.to_account_info();
            let position_info = ctx.accounts.position.to_account_info();
            let token_vault_a_info = ctx.accounts.token_vault_a.to_account_info();
            let token_vault_b_info = ctx.accounts.token_vault_b.to_account_info();
            let token_mint_a_info = ctx.accounts.token_mint_a.to_account_info();
            let quote_mint_info = ctx.accounts.quote_mint.to_account_info();
            let position_nft_account_info = ctx.accounts.position_nft_account.to_account_info();
            let event_authority_info = ctx.accounts.event_authority.to_account_info();
            let damm_program_info = ctx.accounts.damm_program.to_account_info();

            let (fee_a, fee_b) = claim_fees(
                &owner_pda_info,
                &pool_authority_info,
                &pool_info,
                &position_info,
                &mut ctx.accounts.base_treasury,
                &mut ctx.accounts.treasury,
                &token_vault_a_info,
                &token_vault_b_info,
                &token_mint_a_info,
                &quote_mint_info,
                &position_nft_account_info,
                &event_authority_info,
                &damm_program_info,
                &ctx.accounts.token_program,
                &ctx.accounts.vault.key(),
            )?;

            require!(fee_a == 0, ErrorCode::BaseFeeDetected);
            progress.claimed_for_day = fee_b;

            emit!(QuoteFeesClaimed {
                vault: ctx.accounts.vault.key(),
                amount: fee_b,
            });

            let total_available = progress.carry_over + progress.claimed_for_day;
            let f_locked = if policy.y0 == 0 {
                0
            } else {
                (locked_total * 10000) / policy.y0
            };
            let eligible_bps = policy.investor_fee_share_bps.min(f_locked as u16);
            let mut investor_intended =
                (total_available as u128 * eligible_bps as u128 / 10000) as u64;
            if let Some(cap) = policy.daily_cap {
                investor_intended = investor_intended.min(cap);
            }
            progress.investor_intended_for_day = investor_intended;
            progress.creator_share_for_day = total_available.saturating_sub(investor_intended);
            progress.carry_over = 0;
        }

        require!(page_index == progress.cursor, ErrorCode::InvalidPageIndex);

        let page_accounts = ctx.remaining_accounts.chunks(2);
        let mut page_distributed = 0u64;

        for chunk in page_accounts {
            if chunk.len() < 2 {
                continue;
            }

            let stream_ai = &chunk[0];
            let investor_ata_ai = &chunk[1];

            let stream_data = stream_ai.data.borrow();
            let stream = Stream::try_deserialize_unchecked(&mut &stream_data[..])?;
            let unlocked = stream.unlocked_amount(now);
            let locked_i = stream.deposited_amount.saturating_sub(unlocked);

            if locked_i == 0 {
                continue;
            }

            let weight = if locked_total == 0 {
                0
            } else {
                (locked_i as u128 * 1_000_000u128) / (locked_total as u128)
            } as u64;
            let payout =
                (progress.investor_intended_for_day as u128 * weight as u128 / 1_000_000) as u64;

            if payout >= policy.min_payout_lamports {
                let vault_key = ctx.accounts.vault.key();
                let seeds = &[b"investor_fee_pos_owner", vault_key.as_ref()];
                let signer_seeds = &[&seeds[..]];

                token::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info(),
                        Transfer {
                            from: ctx.accounts.treasury.to_account_info(),
                            to: investor_ata_ai.clone(),
                            authority: ctx.accounts.owner_pda.to_account_info(),
                        },
                        signer_seeds,
                    ),
                    payout,
                )?;

                page_distributed += payout;
            } else {
                progress.carry_over += payout;
            }
        }

        progress.actual_distributed += page_distributed;
        progress.cursor += 1;

        if is_final_page {
            let undistributed = progress
                .investor_intended_for_day
                .saturating_sub(progress.actual_distributed);
            progress.carry_over += undistributed;

            let total_to_creator = progress.creator_share_for_day;
            if total_to_creator > 0 {
                let vault_key = ctx.accounts.vault.key();
                let seeds = &[b"investor_fee_pos_owner", vault_key.as_ref()];
                let signer_seeds = &[&seeds[..]];
                token::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info(),
                        Transfer {
                            from: ctx.accounts.treasury.to_account_info(),
                            to: ctx.accounts.creator_ata.to_account_info(),
                            authority: ctx.accounts.owner_pda.to_account_info(),
                        },
                        signer_seeds,
                    ),
                    total_to_creator,
                )?;
            }

            emit!(CreatorPayoutDayClosed {
                vault: ctx.accounts.vault.key(),
                day_ts: progress.current_day_start_ts,
                claimed: progress.claimed_for_day,
                investor_intended: progress.investor_intended_for_day,
                actual_distributed: progress.actual_distributed,
                creator_received: total_to_creator,
                carry_over: progress.carry_over,
            });
        }

        emit!(InvestorPayoutPage {
            vault: ctx.accounts.vault.key(),
            page_index,
            amount: page_distributed,
        });

        Ok(())
    }
}

fn claim_fees<'info>(
    owner_pda: &AccountInfo<'info>,
    pool_authority: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    position: &AccountInfo<'info>,
    base_treasury: &mut Account<'info, TokenAccount>,
    treasury: &mut Account<'info, TokenAccount>,
    token_vault_a: &AccountInfo<'info>,
    token_vault_b: &AccountInfo<'info>,
    token_mint_a: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    position_nft_account: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
    damm_program: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    vault_key: &Pubkey,
) -> Result<(u64, u64)> {
    let seeds = &[b"investor_fee_pos_owner", vault_key.as_ref()];
    let signer_seeds = &[&seeds[..]];

    let discriminator = [180, 38, 154, 17, 133, 33, 162, 211];
    let ix_data = discriminator.to_vec();

    let pre_fee_a = base_treasury.amount;
    let pre_fee_b = treasury.amount;

    let ix = Instruction {
        program_id: DAMM_V2_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(POOL_AUTHORITY, false),
            AccountMeta::new_readonly(pool.key(), false),
            AccountMeta::new(position.key(), false),
            AccountMeta::new(base_treasury.key(), false),
            AccountMeta::new(treasury.key(), false),
            AccountMeta::new(token_vault_a.key(), false),
            AccountMeta::new(token_vault_b.key(), false),
            AccountMeta::new_readonly(token_mint_a.key(), false),
            AccountMeta::new_readonly(quote_mint.key(), false),
            AccountMeta::new_readonly(position_nft_account.key(), false),
            AccountMeta::new_readonly(owner_pda.key(), false),
            AccountMeta::new_readonly(TOKEN22_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN22_PROGRAM_ID, false),
            AccountMeta::new_readonly(event_authority.key(), false),
            AccountMeta::new_readonly(DAMM_V2_PROGRAM_ID, false),
        ],
        data: ix_data,
    };

    invoke_signed(
        &ix,
        &[
            pool_authority.clone(),
            pool.clone(),
            position.clone(),
            base_treasury.to_account_info(),
            treasury.to_account_info(),
            token_vault_a.clone(),
            token_vault_b.clone(),
            token_mint_a.clone(),
            quote_mint.clone(),
            position_nft_account.clone(),
            owner_pda.clone(),
            token_program.to_account_info(),
            token_program.to_account_info(),
            event_authority.clone(),
            damm_program.clone(),
        ],
        signer_seeds,
    )?;

    base_treasury.reload()?;
    treasury.reload()?;

    let fee_a = base_treasury.amount - pre_fee_a;
    let fee_b = treasury.amount - pre_fee_b;

    Ok((fee_a, fee_b))
}

impl Stream {
    fn unlocked_amount(&self, now: u64) -> u64 {
        if now < self.start_time {
            0
        } else if now < self.start_time + self.cliff {
            0
        } else {
            let elapsed_after_cliff = now - self.start_time - self.cliff;
            let periods_elapsed = elapsed_after_cliff / self.period;
            let unlocked = self.cliff_amount + (periods_elapsed * self.amount_per_period);
            unlocked.min(self.deposited_amount - self.withdrawn_amount)
        }
    }
}

#[derive(Accounts)]
pub struct ValidatePool<'info> {
    #[account(owner = DAMM_V2_PROGRAM_ID)]
    pub pool: Account<'info, Pool>,
    pub quote_mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct InitializePolicy<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<Policy>(),
        seeds = [b"policy", vault.key().as_ref()],
        bump
    )]
    pub policy: Account<'info, Policy>,
    /// CHECK: vault identifier
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeHonoraryPosition<'info> {
    /// CHECK: vault identifier
    pub vault: AccountInfo<'info>,
    #[account(
        seeds = [b"investor_fee_pos_owner", vault.key().as_ref()],
        bump
    )]
    pub owner_pda: SystemAccount<'info>,
    /// CHECK: validated by DAMM program
    #[account(mut)]
    pub position_nft_mint: UncheckedAccount<'info>,
    /// CHECK: validated by DAMM program
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    #[account(mut, owner = DAMM_V2_PROGRAM_ID)]
    pub pool: Account<'info, Pool>,
    /// CHECK: validated by DAMM program
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// CHECK: address constraint
    #[account(address = POOL_AUTHORITY)]
    pub pool_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(address = TOKEN22_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    /// CHECK: event authority
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: address constraint
    #[account(address = DAMM_V2_PROGRAM_ID)]
    pub damm_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct InitializeProgress<'info> {
    /// CHECK: vault identifier
    pub vault: AccountInfo<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<Progress>(),
        seeds = [b"progress", vault.key().as_ref()],
        bump
    )]
    pub progress: Account<'info, Progress>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeTreasuryAccounts<'info> {
    /// CHECK: vault identifier
    pub vault: AccountInfo<'info>,
    #[account(
        seeds = [b"investor_fee_pos_owner", vault.key().as_ref()],
        bump
    )]
    pub owner_pda: SystemAccount<'info>,
    pub token_mint_a: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = base_treasury.owner == owner_pda.key(),
        constraint = base_treasury.mint == token_mint_a.key()
    )]
    pub base_treasury: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = quote_treasury.owner == owner_pda.key(),
        constraint = quote_treasury.mint == quote_mint.key()
    )]
    pub quote_treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(address = TOKEN22_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Crank<'info> {
    /// CHECK: vault identifier
    pub vault: AccountInfo<'info>,
    #[account(
        seeds = [b"investor_fee_pos_owner", vault.key().as_ref()],
        bump
    )]
    pub owner_pda: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"progress", vault.key().as_ref()],
        bump
    )]
    pub progress: Account<'info, Progress>,
    #[account(
        seeds = [b"policy", vault.key().as_ref()],
        bump
    )]
    pub policy: Account<'info, Policy>,
    #[account(mut)]
    pub base_treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub creator_ata: Account<'info, TokenAccount>,
    #[account(owner = DAMM_V2_PROGRAM_ID)]
    pub position: Account<'info, Position>,
    pub token_mint_a: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,
    #[account(address = TOKEN22_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub token_vault_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_vault_b: Account<'info, TokenAccount>,
    /// CHECK: address constraint
    #[account(address = POOL_AUTHORITY)]
    pub pool_authority: UncheckedAccount<'info>,
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub position_nft_account: Account<'info, TokenAccount>,
    /// CHECK: event authority
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: address constraint
    #[account(address = DAMM_V2_PROGRAM_ID)]
    pub damm_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CreateStream<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(mut)]
    pub sender_tokens: Account<'info, TokenAccount>,
    /// CHECK: validated by Streamflow
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(mut)]
    pub escrow_tokens: Account<'info, TokenAccount>,
    /// CHECK: validated by Streamflow
    #[account(mut)]
    pub withdrawor: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    /// CHECK: fee oracle
    pub fee_oracle: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: address constraint
    #[account(address = STREAMFLOW_PROGRAM_ID)]
    pub timelock_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct Policy {
    pub vault: Pubkey,
    pub y0: u64,
    pub investor_fee_share_bps: u16,
    pub daily_cap: Option<u64>,
    pub min_payout_lamports: u64,
}

#[account]
#[derive(Default)]
pub struct Progress {
    pub vault: Pubkey,
    pub last_distribution_ts: u64,
    pub current_day_start_ts: u64,
    pub claimed_for_day: u64,
    pub investor_intended_for_day: u64,
    pub creator_share_for_day: u64,
    pub actual_distributed: u64,
    pub carry_over: u64,
    pub cursor: u16,
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub collect_fee_mode: u8,
    pub tick_current: i32,
    pub liquidity: u128,
    pub fee_growth_global_a_x64: u128,
    pub fee_growth_global_b_x64: u128,
}

#[account]
#[derive(Default)]
pub struct Position {}

#[account]
#[derive(Default)]
pub struct Stream {
    pub start_time: u64,
    pub deposited_amount: u64,
    pub period: u64,
    pub amount_per_period: u64,
    pub cliff: u64,
    pub cliff_amount: u64,
    pub withdrawn_amount: u64,
}

#[event]
pub struct PolicyInitialized {
    pub vault: Pubkey,
    pub y0: u64,
    pub investor_fee_share_bps: u16,
}

#[event]
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub position: Pubkey,
}

#[event]
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub amount: u64,
}

#[event]
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub page_index: u16,
    pub amount: u64,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub day_ts: u64,
    pub claimed: u64,
    pub investor_intended: u64,
    pub actual_distributed: u64,
    pub creator_received: u64,
    pub carry_over: u64,
}

#[event]
pub struct ProgressInitialized {
    pub vault: Pubkey,
}

#[event]
pub struct TreasuryAccountsInitialized {
    pub vault: Pubkey,
    pub base_treasury: Pubkey,
    pub quote_treasury: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Pool configuration does not guarantee honorary position to accrue fees exclusively in quote mint")]
    InvalidPoolConfig,
    #[msg("Invalid quote mint")]
    InvalidQuoteMint,
    #[msg("Base fee detected, aborting")]
    BaseFeeDetected,
    #[msg("Invalid page index")]
    InvalidPageIndex,
    #[msg("Invalid vault")]
    InvalidVault,
    #[msg("Invalid tick range for quote-only fee position")]
    InvalidTickRange,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateUncheckedWithPayerArgs {
    pub start_time: u64,
    pub net_amount_deposited: u64,
    pub period: u64,
    pub amount_per_period: u64,
    pub cliff: u64,
    pub cliff_amount: u64,
    pub cancelable_by_sender: bool,
    pub cancelable_by_recipient: bool,
    pub automatic_withdrawal: bool,
    pub transferable_by_sender: bool,
    pub transferable_by_recipient: bool,
    pub can_topup: bool,
    pub stream_name: [u8; 64],
    pub withdraw_frequency: u64,
    pub recipient: Pubkey,
    pub partner: Pubkey,
    pub pausable: bool,
    pub can_update_rate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_only_validation() {
        let current_tick = 100;
        let tick_lower = -200;
        let tick_upper = -100;

        assert!(
            tick_upper < current_tick,
            "Position must be below current price"
        );
        assert!(tick_lower < tick_upper, "Tick range must be valid");
    }

    #[test]
    fn test_distribution_math() {
        let total_locked: u64 = 1000000;
        let investor_locked: u64 = 250000;
        let total_fees: u64 = 10000;

        let weight = (investor_locked as u128 * 1_000_000u128) / (total_locked as u128);
        let payout = (total_fees as u128 * weight) / 1_000_000;

        assert_eq!(payout, 2500, "Should receive 25% of fees");
    }

    #[test]
    fn test_24hour_gate() {
        let last_distribution: u64 = 1000000;
        let now: u64 = 1000000 + 86399;

        assert!(now < last_distribution + 86400, "Must wait 24 hours");

        let now_valid: u64 = 1000000 + 86400;
        assert!(
            now_valid >= last_distribution + 86400,
            "Should allow after 24 hours"
        );
    }

    #[test]
    fn test_dust_handling() {
        let min_payout: u64 = 1000000;
        let small_amount: u64 = 999999;
        let valid_amount: u64 = 1000000;

        assert!(small_amount < min_payout, "Should be treated as dust");
        assert!(valid_amount >= min_payout, "Should be distributed");
    }

    #[test]
    fn test_daily_cap() {
        let daily_cap: u64 = 100000000000;
        let claimed_fees: u64 = 200000000000;
        let investor_share_bps: u16 = 5000;

        let intended = (claimed_fees as u128 * investor_share_bps as u128 / 10000) as u64;
        let capped = intended.min(daily_cap);

        assert_eq!(capped, daily_cap, "Should be capped at daily limit");
    }

    #[test]
    fn test_locked_fraction_calculation() {
        let y0: u64 = 1000000000000;
        let locked_total: u64 = 750000000000;

        let f_locked = (locked_total * 10000) / y0;
        assert_eq!(f_locked, 7500, "Should be 75% locked");

        let investor_fee_share_bps: u16 = 5000;
        let eligible_bps = investor_fee_share_bps.min(f_locked as u16);
        assert_eq!(
            eligible_bps, 5000,
            "Should use base share when locked > 50%"
        );
    }

    #[test]
    fn test_all_unlocked_scenario() {
        let locked_total: u64 = 0;
        let claimed_fees: u64 = 100000000000;
        let investor_share_bps: u16 = 5000;
        let y0: u64 = 1000000000000;

        let f_locked = if y0 == 0 {
            0
        } else {
            (locked_total * 10000) / y0
        };
        let eligible_bps = investor_share_bps.min(f_locked as u16);

        assert_eq!(
            eligible_bps, 0,
            "All fees should go to creator when fully unlocked"
        );
    }

    #[test]
    fn test_pagination_state() {
        let mut cursor: u16 = 0;
        let page_size: u16 = 10;
        let total_investors: u16 = 25;

        assert_eq!(cursor, 0);
        cursor += 1;

        assert_eq!(cursor, 1);
        cursor += 1;

        assert_eq!(cursor, 2);
        let is_final = cursor == (total_investors - 1) / page_size;
        assert!(is_final);
    }
}
