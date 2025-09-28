# Honorary DAMM V2 Fee Routing Anchor Program

A permissionless, Anchor-compatible program for creating and managing an "honorary" Meteora DAMM v2 LP position that accrues fees in quote mint only, with automatic distribution to investors and creators.

## Overview

This program implements a fee distribution mechanism for Star's fundraising platform, where:
- An honorary LP position accrues fees exclusively in the quote token
- Fees are distributed pro-rata to investors based on their still-locked token amounts
- The remainder goes to the creator wallet
- Distribution occurs through a permissionless crank callable once per 24 hours

## Key Features

**Quote-Only Fee Accrual**: Position configuration ensures fees accrue only in quote mint
**Program-Owned Position**: Fee position owned by PDA for security
**24-Hour Distribution Window**: Permissionless crank with daily gating
**Pro-Rata Distribution**: Investors receive fees proportional to locked amounts
**Pagination Support**: Handles large investor sets across multiple transactions
**Dust & Cap Management**: Handles minimum payouts and daily caps
**Idempotent Operations**: Safe to retry failed transactions

## Architecture

#### *Scratch understanding from my excalidraw board of the bounty.*

<img width="2874" height="1559" alt="damm-anchor-module" src="https://github.com/user-attachments/assets/4f0312b4-144e-4229-a7fb-eabf32098f8c" />

### Program Structure

```
programs/honorary-dammv2-crank/
├── src/
│   └── lib.rs              # Main program logic
├── Cargo.toml              # Rust dependencies
└── Xargo.toml              # Cross-compilation config
```

### Key Components

1. **Honorary Position**: A DAMM v2 LP position owned by the program PDA
2. **Policy PDA**: Stores distribution parameters (Y0, fee share, caps)
3. **Progress PDA**: Tracks distribution state and pagination
4. **Treasury Accounts**: Token accounts for collecting fees
5. **Crank Mechanism**: Permissionless distribution function

## Installation

### Prerequisites

- Rust 1.75+
- Solana CLI 1.18+
- Anchor CLI 0.31.1
- Node.js 18+
- Yarn

### Setup

```bash
# Clone the repository
git clone https://github.com/your-org/honorary-dammv2-crank
cd honorary-dammv2-crank

# Install dependencies
yarn install

# Build the program
anchor build

# Run tests
anchor test
```

## Program Instructions

### 1. `validate_pool`
Validates that a DAMM v2 pool is configured for quote-only fee collection.

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| pool | Account<Pool> | DAMM v2 pool to validate |
| quote_mint | Account<Mint> | Expected quote mint |

### 2. `initialize_policy`
Initializes the fee distribution policy for a vault.

**Arguments:**
- `y0: u64` - Total investor allocation at TGE
- `investor_fee_share_bps: u16` - Base investor share in basis points
- `daily_cap: Option<u64>` - Optional daily distribution cap
- `min_payout_lamports: u64` - Minimum payout threshold

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| vault | AccountInfo | Vault identifier |
| policy | Account<Policy> | Policy PDA (seeds: ["policy", vault]) |
| payer | Signer | Transaction fee payer |
| system_program | Program | System program |

### 3. `initialize_progress`
Initializes the progress tracking PDA for distribution state.

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| vault | AccountInfo | Vault identifier |
| progress | Account<Progress> | Progress PDA (seeds: ["progress", vault]) |
| payer | Signer | Transaction fee payer |
| system_program | Program | System program |

### 4. `initialize_honorary_position`
Creates the honorary DAMM v2 position for fee accrual.

**Arguments:**
- `tick_lower_index: i32` - Lower tick boundary
- `tick_upper_index: i32` - Upper tick boundary (must be < current tick)
- `liquidity: u128` - Initial liquidity amount

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| vault | AccountInfo | Vault identifier |
| owner_pda | SystemAccount | Position owner PDA |
| position_nft_mint | UncheckedAccount | Position NFT mint |
| position_nft_account | UncheckedAccount | Position NFT token account |
| pool | Account<Pool> | DAMM v2 pool |
| position | UncheckedAccount | Position account |
| pool_authority | UncheckedAccount | Pool authority |
| payer | Signer | Transaction fee payer |
| token_program | Program | Token program |
| system_program | Program | System program |
| event_authority | UncheckedAccount | Event authority PDA |
| damm_program | UncheckedAccount | DAMM v2 program |

### 5. `initialize_treasury_accounts`
Verifies treasury token accounts for fee collection.

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| vault | AccountInfo | Vault identifier |
| owner_pda | SystemAccount | Treasury owner PDA |
| token_mint_a | Account<Mint> | Base token mint |
| quote_mint | Account<Mint> | Quote token mint |
| base_treasury | Account<TokenAccount> | Base token treasury |
| quote_treasury | Account<TokenAccount> | Quote token treasury |
| payer | Signer | Transaction fee payer |
| system_program | Program | System program |
| token_program | Program | Token program |

### 6. `crank`
Permissionless distribution mechanism callable once per 24 hours.

**Arguments:**
- `page_index: u16` - Current page index for pagination
- `locked_total: u64` - Total locked amount across all investors
- `is_final_page: bool` - Whether this is the last page

**Accounts:**
| Account | Type | Description |
|---------|------|-------------|
| vault | AccountInfo | Vault identifier |
| owner_pda | SystemAccount | Position owner PDA |
| progress | Account<Progress> | Progress tracking PDA |
| policy | Account<Policy> | Distribution policy |
| base_treasury | Account<TokenAccount> | Base fee treasury |
| treasury | Account<TokenAccount> | Quote fee treasury |
| creator_ata | Account<TokenAccount> | Creator's quote token account |
| position | Account<Position> | Honorary position |
| ... | ... | Additional DAMM v2 accounts |

**Remaining Accounts:**
Pairs of (Streamflow stream, investor ATA) for each investor on the current page.

## Distribution Formula

The distribution follows these rules:

1. **Calculate locked fraction**: `f_locked(t) = locked_total(t) / Y0`
2. **Determine investor share**: `eligible_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))`
3. **Calculate amounts**:
   - `investor_fee_quote = floor(claimed_quote * eligible_bps / 10000)`
   - Apply daily cap if configured
   - Apply minimum payout threshold
4. **Distribute pro-rata**: `payout_i = floor(investor_fee_quote * locked_i / locked_total)`
5. **Creator receives remainder**: After final page

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| InvalidPoolConfig | 6000 | Pool doesn't guarantee quote-only fees |
| InvalidQuoteMint | 6001 | Quote mint doesn't match expected |
| BaseFeeDetected | 6002 | Non-zero base fees detected |
| InvalidPageIndex | 6003 | Page index doesn't match cursor |
| InvalidVault | 6004 | Vault key mismatch |
| InvalidTickRange | 6005 | Tick range invalid for quote-only position |

## Integration Guide

### Step 1: Deploy and Initialize

```typescript
// 1. Deploy program
const programId = new PublicKey("ddcEKSibupo9XMaeHH66rVkpqCpWybXtAZWaBbMbF3h");

// 2. Initialize policy
await program.methods
  .initializePolicy(
    new BN(1000000000000), // Y0: 1M tokens
    5000,                  // 50% base share
    new BN(100000000000),  // 100k daily cap
    new BN(1000000)        // 1 token minimum
  )
  .accounts({ ... })
  .rpc();

// 3. Initialize progress tracker
await program.methods
  .initializeProgress()
  .accounts({ ... })
  .rpc();

// 4. Create treasury accounts (externally)
const baseTreasury = await createAccount(...);
const quoteTreasury = await createAccount(...);

// 5. Initialize treasury validation
await program.methods
  .initializeTreasuryAccounts()
  .accounts({ ... })
  .rpc();

// 6. Create honorary position with quote-only configuration
await program.methods
  .initializeHonoraryPosition(
    -887272,  // tick_lower (out of range)
    -887270,  // tick_upper (below current tick)
    1000000   // minimal liquidity
  )
  .accounts({ ... })
  .rpc();
```

### Step 2: Run Distribution Crank

```typescript
// Called by anyone, once per 24 hours
const investors = [...]; // Load investor data
const pageSize = 10;
const pages = Math.ceil(investors.length / pageSize);

for (let page = 0; page < pages; page++) {
  const pageInvestors = investors.slice(page * pageSize, (page + 1) * pageSize);
  const remainingAccounts = pageInvestors.flatMap(inv => [
    { pubkey: inv.streamPubkey, isWritable: false, isSigner: false },
    { pubkey: inv.ataAddress, isWritable: true, isSigner: false }
  ]);

  await program.methods
    .crank(
      page,
      calculateLockedTotal(investors),
      page === pages - 1
    )
    .accounts({ ... })
    .remainingAccounts(remainingAccounts)
    .rpc();
}
```
## Here are the security practices I did.

1. **PDA Ownership**: All critical accounts owned by program PDAs
2. **24-Hour Gate**: Prevents rapid draining of fees
3. **Quote-Only Validation**: Fails if base fees detected
4. **Idempotent Pages**: Safe to retry on failure
5. **Slippage Protection**: Uses floor() for all calculations

## Testing

Run the test suite:
```bash
# Unit tests
cargo test

# Integration tests
anchor test

# Specific test
anchor test -- --grep "should distribute fees pro-rata"
```

## Deployment

### Mainnet Deployment
```bash
# Build for mainnet
anchor build --verifiable

# Deploy
anchor deploy --provider.cluster mainnet

# Verify build
anchor verify <PROGRAM_ID>
```

### Configuration
Update `Anchor.toml`:
```toml
[programs.mainnet]
honorary_dammv2_crank = "YOUR_PROGRAM_ID"

[provider]
cluster = "mainnet"
wallet = "~/.config/solana/id.json"
```

## Support

For questions or issues:
- https://x.com/takshakmudgal
- takshakmudgal@gmail.com
- https://takshakmudgal.com
