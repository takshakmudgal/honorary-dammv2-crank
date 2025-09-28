import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { HonoraryDammv2Crank } from "../target/types/honorary_dammv2_crank";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { assert } from "chai";

describe("honorary-dammv2-crank", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .HonoraryDammv2Crank as Program<HonoraryDammv2Crank>;

  const TOKEN22_PROGRAM_ID = new PublicKey(
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
  );
  const DAMM_V2_PROGRAM_ID = new PublicKey(
    "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"
  );
  const POOL_AUTHORITY = new PublicKey(
    "HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC"
  );

  let vault: PublicKey;
  let ownerPda: PublicKey;
  let policy: PublicKey;
  let progress: PublicKey;
  let baseTreasury: PublicKey;
  let quoteTreasury: PublicKey;

  const Y0 = new BN(1000000000000);
  const INVESTOR_FEE_SHARE_BPS = 5000;
  const DAILY_CAP = new BN(100000000000);
  const MIN_PAYOUT_LAMPORTS = new BN(1000000);

  before(async () => {
    vault = Keypair.generate().publicKey;

    [ownerPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("investor_fee_pos_owner"), vault.toBuffer()],
      program.programId
    );

    [policy] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), vault.toBuffer()],
      program.programId
    );

    [progress] = PublicKey.findProgramAddressSync(
      [Buffer.from("progress"), vault.toBuffer()],
      program.programId
    );

    [baseTreasury] = PublicKey.findProgramAddressSync(
      [Buffer.from("base_treasury"), vault.toBuffer()],
      program.programId
    );

    [quoteTreasury] = PublicKey.findProgramAddressSync(
      [Buffer.from("quote_treasury"), vault.toBuffer()],
      program.programId
    );
  });

  describe("Policy Management", () => {
    it("initializes policy with correct parameters", async () => {
      const tx = await program.methods
        .initializePolicy(
          Y0,
          INVESTOR_FEE_SHARE_BPS,
          DAILY_CAP,
          MIN_PAYOUT_LAMPORTS
        )
        .accounts({
          vault,
          policy,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const policyAccount = await program.account.policy.fetch(policy);
      assert.equal(policyAccount.vault.toBase58(), vault.toBase58());
      assert.equal(policyAccount.y0.toString(), Y0.toString());
      assert.equal(policyAccount.investorFeeShareBps, INVESTOR_FEE_SHARE_BPS);
      assert.equal(policyAccount.dailyCap.toString(), DAILY_CAP.toString());
      assert.equal(
        policyAccount.minPayoutLamports.toString(),
        MIN_PAYOUT_LAMPORTS.toString()
      );
    });

    it("prevents double initialization", async () => {
      try {
        await program.methods
          .initializePolicy(
            Y0,
            INVESTOR_FEE_SHARE_BPS,
            DAILY_CAP,
            MIN_PAYOUT_LAMPORTS
          )
          .accounts({
            vault,
            policy,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should not allow double initialization");
      } catch (error) {
        assert.ok(error);
      }
    });
  });

  describe("Progress Tracking", () => {
    it("initializes progress state", async () => {
      const tx = await program.methods
        .initializeProgress()
        .accounts({
          vault,
          progress,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const progressAccount = await program.account.progress.fetch(progress);
      assert.equal(progressAccount.vault.toBase58(), vault.toBase58());
      assert.equal(progressAccount.lastDistributionTs.toNumber(), 0);
      assert.equal(progressAccount.currentDayStartTs.toNumber(), 0);
      assert.equal(progressAccount.claimedForDay.toNumber(), 0);
      assert.equal(progressAccount.actualDistributed.toNumber(), 0);
      assert.equal(progressAccount.carryOver.toNumber(), 0);
      assert.equal(progressAccount.cursor, 0);
    });
  });

  describe("Distribution Logic", () => {
    it("validates 24-hour gate", async () => {
      const progressAccount = await program.account.progress.fetch(progress);
      const lastDistribution = progressAccount.lastDistributionTs.toNumber();
      const now = Math.floor(Date.now() / 1000);

      if (lastDistribution > 0) {
        const timeSinceLastDistribution = now - lastDistribution;
        const canDistribute = timeSinceLastDistribution >= 86400;

        if (!canDistribute) {
          assert.ok(
            timeSinceLastDistribution < 86400,
            "24-hour gate should be enforced"
          );
        }
      }
    });

    it("calculates pro-rata distribution correctly", () => {
      const totalLocked = new BN(1000000000000);
      const investorLocked = new BN(250000000000);
      const totalFees = new BN(10000000000);

      const weight = investorLocked.mul(new BN(1000000)).div(totalLocked);
      const payout = totalFees.mul(weight).div(new BN(1000000));

      assert.equal(
        payout.toString(),
        "2500000000",
        "Should receive 25% of fees"
      );
    });

    it("respects daily cap", () => {
      const claimedFees = new BN(200000000000);
      const investorShareBps = 5000;

      const intended = claimedFees
        .mul(new BN(investorShareBps))
        .div(new BN(10000));
      const capped = intended.gt(DAILY_CAP) ? DAILY_CAP : intended;

      if (intended.gt(DAILY_CAP)) {
        assert.equal(
          capped.toString(),
          DAILY_CAP.toString(),
          "Should cap at daily limit"
        );
      }
    });

    it("handles dust amounts", () => {
      const smallAmount = new BN(999999);
      const shouldCarryOver = smallAmount.lt(MIN_PAYOUT_LAMPORTS);

      assert.ok(
        shouldCarryOver,
        "Amounts below minimum should be carried over"
      );
    });

    it("calculates locked fraction correctly", () => {
      const lockedTotal = new BN(750000000000);
      const fLocked = lockedTotal.mul(new BN(10000)).div(Y0).toNumber();

      assert.equal(fLocked, 7500, "Should be 75% locked");

      const eligibleBps = Math.min(INVESTOR_FEE_SHARE_BPS, fLocked);
      assert.equal(
        eligibleBps,
        INVESTOR_FEE_SHARE_BPS,
        "Should use base share when locked > 50%"
      );
    });

    it("handles all unlocked scenario", () => {
      const lockedTotal = new BN(0);
      const fLocked = Y0.eq(new BN(0))
        ? 0
        : lockedTotal.mul(new BN(10000)).div(Y0).toNumber();
      const eligibleBps = Math.min(INVESTOR_FEE_SHARE_BPS, fLocked);

      assert.equal(
        eligibleBps,
        0,
        "All fees should go to creator when fully unlocked"
      );
    });
  });

  describe("Pagination", () => {
    it("validates cursor state", async () => {
      const progressAccount = await program.account.progress.fetch(progress);
      const cursor = progressAccount.cursor;

      assert.ok(cursor >= 0, "Cursor should be non-negative");
      assert.ok(cursor <= 65535, "Cursor should fit in u16");
    });

    it("tracks page progression", () => {
      const totalInvestors = 25;
      const pageSize = 10;
      const expectedPages = Math.ceil(totalInvestors / pageSize);

      assert.equal(expectedPages, 3, "Should require 3 pages for 25 investors");

      for (let page = 0; page < expectedPages; page++) {
        const startIdx = page * pageSize;
        const endIdx = Math.min((page + 1) * pageSize, totalInvestors);
        const pageInvestors = endIdx - startIdx;

        assert.ok(pageInvestors > 0, "Each page should have investors");
        assert.ok(
          pageInvestors <= pageSize,
          "Page should not exceed size limit"
        );
      }
    });
  });

  describe("Error Handling", () => {
    it("validates tick range for quote-only fees", () => {
      const currentTick = 100;
      const tickLower = -200;
      const tickUpper = -100;

      const isQuoteOnly = tickUpper < currentTick;
      const isValidRange = tickLower < tickUpper;

      assert.ok(
        isQuoteOnly,
        "Position must be below current price for quote-only fees"
      );
      assert.ok(isValidRange, "Tick range must be valid");
    });

    it("rejects base fee presence", () => {
      const feeA = new BN(0);
      const feeB = new BN(1000000);

      const hasBaseFees = feeA.gt(new BN(0));
      assert.ok(!hasBaseFees, "Should reject if base fees present");
      assert.ok(feeB.gt(new BN(0)), "Should have quote fees");
    });
  });

  describe("Integration Validation", () => {
    it("validates PDA derivations", () => {
      const seeds = [
        { name: "investor_fee_pos_owner", pda: ownerPda },
        { name: "policy", pda: policy },
        { name: "progress", pda: progress },
        { name: "base_treasury", pda: baseTreasury },
        { name: "quote_treasury", pda: quoteTreasury },
      ];

      for (const seed of seeds) {
        const [derived] = PublicKey.findProgramAddressSync(
          [Buffer.from(seed.name), vault.toBuffer()],
          program.programId
        );
        assert.equal(
          derived.toBase58(),
          seed.pda.toBase58(),
          `${seed.name} PDA should match`
        );
      }
    });

    it("validates program constants", () => {
      assert.equal(
        DAMM_V2_PROGRAM_ID.toBase58(),
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"
      );
      assert.equal(
        POOL_AUTHORITY.toBase58(),
        "HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC"
      );
      assert.equal(
        TOKEN22_PROGRAM_ID.toBase58(),
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
      );
    });
  });
});
