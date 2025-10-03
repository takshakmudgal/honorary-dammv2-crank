# Honorary DAMM V2 Fee Routing Anchor Program

Permissionless Anchor program for managing honorary Meteora DAMM v2 LP positions that accrue fees in quote mint only, with automatic distribution to investors and creators.

## Overview

This program implements a fee distribution mechanism where:
- An honorary LP position accrues fees exclusively in quote tokens
- Fees are distributed pro-rata to investors based on locked token amounts
- Remainder goes to creator wallet
- Distribution occurs through permissionless crank callable once per 24 hours

## Features

- Quote-only fee accrual with validation
- Program-owned positions via PDA
- 24-hour distribution window
- Pro-rata distribution based on Streamflow locked amounts
- Multi-page pagination support
- Dust and cap management
- Idempotent operations

## Installation

Prerequisites:
- Rust 1.75+
- Solana CLI 1.18+
- Anchor CLI 0.31.1
- Node.js 18+
- Yarn

```bash
git clone <repository-url>
cd honorary-dammv2-crank
yarn install
anchor build
anchor test
```

## Program Instructions

### validate_pool
Validates DAMM v2 pool configuration for quote-only fee collection.

### initialize_policy
Initializes fee distribution policy.

Arguments:
- `y0: u64` - Total investor allocation at TGE
- `investor_fee_share_bps: u16` - Base investor share in basis points
- `daily_cap: Option<u64>` - Optional daily distribution cap
- `min_payout_lamports: u64` - Minimum payout threshold

### initialize_progress
Initializes progress tracking PDA.

### initialize_honorary_position
Creates honorary DAMM v2 position for fee accrual.

Arguments:
- `tick_lower_index: i32` - Lower tick boundary
- `tick_upper_index: i32` - Upper tick boundary (must be < current tick)
- `liquidity: u128` - Initial liquidity amount

### initialize_treasury_accounts
Verifies treasury token accounts.

### crank
Permissionless distribution mechanism.

Arguments:
- `page_index: u16` - Current page index
- `locked_total: u64` - Total locked amount across investors
- `is_final_page: bool` - Whether this is last page

Remaining Accounts: Pairs of (Streamflow stream, investor ATA) for each investor on current page.

## Distribution Formula

1. Calculate locked fraction: `f_locked(t) = locked_total(t) / Y0`
2. Determine investor share: `eligible_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))`
3. Calculate amounts:
   - `investor_fee_quote = floor(claimed_quote * eligible_bps / 10000)`
   - Apply daily cap if configured
   - Apply minimum payout threshold
4. Distribute pro-rata: `payout_i = floor(investor_fee_quote * locked_i / locked_total)`
5. Creator receives remainder after final page

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 6000 | InvalidPoolConfig | Pool doesn't guarantee quote-only fees |
| 6001 | InvalidQuoteMint | Quote mint doesn't match expected |
| 6002 | BaseFeeDetected | Non-zero base fees detected |
| 6003 | InvalidPageIndex | Page index doesn't match cursor |
| 6004 | InvalidVault | Vault key mismatch |
| 6005 | InvalidTickRange | Tick range invalid for quote-only position |

## Integration Example

```typescript
const programId = new PublicKey("ddcEKSibupo9XMaeHH66rVkpqCpWybXtAZWaBbMbF3h");

await program.methods
  .initializePolicy(
    new BN(1000000000000),
    5000,
    new BN(100000000000),
    new BN(1000000)
  )
  .accounts({ vault, policy, payer, systemProgram })
  .rpc();

await program.methods
  .initializeProgress()
  .accounts({ vault, progress, payer, systemProgram })
  .rpc();

await program.methods
  .initializeHonoraryPosition(-887272, -887270, 1000000)
  .accounts({ 
    vault, ownerPda, positionNftMint, positionNftAccount,
    pool, position, poolAuthority, payer, tokenProgram,
    systemProgram, eventAuthority, dammProgram
  })
  .rpc();

const investors = [];
const pageSize = 10;
const pages = Math.ceil(investors.length / pageSize);

for (let page = 0; page < pages; page++) {
  const pageInvestors = investors.slice(page * pageSize, (page + 1) * pageSize);
  const remainingAccounts = pageInvestors.flatMap(inv => [
    { pubkey: inv.streamPubkey, isWritable: false, isSigner: false },
    { pubkey: inv.ataAddress, isWritable: true, isSigner: false }
  ]);

  await program.methods
    .crank(page, calculateLockedTotal(investors), page === pages - 1)
    .accounts({ /* required accounts */ })
    .remainingAccounts(remainingAccounts)
    .rpc();
}
```

## Testing

```bash
cargo test

anchor test

yarn test:unit
yarn test:integration
yarn test:all
```

Test coverage includes:
- Quote-only validation
- Pro-rata distribution
- All unlocked scenario
- Dust handling
- Daily cap enforcement
- Base fee rejection
- Multi-page pagination
- 24-hour gate

## Security

- PDA ownership for all critical accounts
- 24-hour gate prevents rapid draining
- Quote-only validation fails on base fees
- Idempotent pages safe to retry
- Floor-based calculations prevent overpayment

## Deployment

```bash
anchor build --verifiable
anchor deploy --provider.cluster <devnet|mainnet>
anchor verify <PROGRAM_ID>
```

## Contact

- Email: takshakmudgal@gmail.com
- Twitter: @takshakmudgal
- Website: https://takshakmudgal.com
