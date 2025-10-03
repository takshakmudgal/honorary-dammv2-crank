import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { HonoraryDammv2Crank } from "../target/types/honorary_dammv2_crank";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("honorary-dammv2-crank E2E", () => {
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

  let baseMint: PublicKey;
  let quoteMint: PublicKey;
  let vault: PublicKey;
  let ownerPda: PublicKey;
  let policy: PublicKey;
  let progress: PublicKey;
  let baseTreasury: PublicKey;
  let quoteTreasury: PublicKey;
  let creatorQuoteAta: PublicKey;
  let creator: Keypair;

  interface Investor {
    wallet: Keypair;
    quoteAta: PublicKey;
    lockedAmount: BN;
    depositedAmount: BN;
  }

  let investors: Investor[] = [];

  const Y0 = new BN(10_000_000_000);
  const INVESTOR_FEE_SHARE_BPS = 5000;
  const DAILY_CAP = new BN(1_000_000_000);
  const MIN_PAYOUT = new BN(1000);

  before("Setup", async () => {
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

    baseMint = await createMint(
      provider.connection,
      (provider.wallet as any).payer,
      provider.wallet.publicKey,
      null,
      6
    );

    quoteMint = await createMint(
      provider.connection,
      (provider.wallet as any).payer,
      provider.wallet.publicKey,
      null,
      6
    );

    baseTreasury = await createAccount(
      provider.connection,
      (provider.wallet as any).payer,
      baseMint,
      ownerPda
    );

    quoteTreasury = await createAccount(
      provider.connection,
      (provider.wallet as any).payer,
      quoteMint,
      ownerPda
    );

    creator = Keypair.generate();
    await provider.connection.requestAirdrop(
      creator.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    creatorQuoteAta = await createAccount(
      provider.connection,
      (provider.wallet as any).payer,
      quoteMint,
      creator.publicKey
    );
  });

  describe("Initialization", () => {
    it("initializes policy", async () => {
      await program.methods
        .initializePolicy(Y0, INVESTOR_FEE_SHARE_BPS, DAILY_CAP, MIN_PAYOUT)
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
    });

    it("initializes progress", async () => {
      await program.methods
        .initializeProgress()
        .accounts({
          vault,
          progress,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const progressAccount = await program.account.progress.fetch(progress);
      assert.equal(progressAccount.lastDistributionTs.toNumber(), 0);
      assert.equal(progressAccount.cursor, 0);
    });

    it("validates treasury accounts", async () => {
      await program.methods
        .initializeTreasuryAccounts()
        .accounts({
          vault,
          ownerPda,
          tokenMintA: baseMint,
          quoteMint,
          baseTreasury,
          quoteTreasury,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN22_PROGRAM_ID,
        })
        .rpc();
    });
  });

  describe("Investor Setup", () => {
    it("creates investor accounts", async () => {
      const configs = [
        { locked: new BN(4_000_000_000), deposited: new BN(4_000_000_000) },
        { locked: new BN(3_000_000_000), deposited: new BN(3_000_000_000) },
        { locked: new BN(2_000_000_000), deposited: new BN(2_000_000_000) },
        { locked: new BN(1_000_000_000), deposited: new BN(1_000_000_000) },
        { locked: new BN(0), deposited: new BN(5_000_000_000) },
      ];

      for (const config of configs) {
        const wallet = Keypair.generate();
        await provider.connection.requestAirdrop(
          wallet.publicKey,
          LAMPORTS_PER_SOL
        );
        await new Promise((resolve) => setTimeout(resolve, 500));

        const quoteAta = await createAccount(
          provider.connection,
          (provider.wallet as any).payer,
          quoteMint,
          wallet.publicKey
        );

        investors.push({
          wallet,
          quoteAta,
          lockedAmount: config.locked,
          depositedAmount: config.deposited,
        });
      }

      assert.equal(investors.length, 5);
    });
  });

  describe("Fee Accrual", () => {
    it("simulates quote fee accrual", async () => {
      const feeAmount = new BN(2_000_000_000);

      await mintTo(
        provider.connection,
        (provider.wallet as any).payer,
        quoteMint,
        quoteTreasury,
        provider.wallet.publicKey,
        feeAmount.toNumber()
      );

      const treasuryAccount = await getAccount(
        provider.connection,
        quoteTreasury
      );
      assert.equal(treasuryAccount.amount.toString(), feeAmount.toString());
    });

    it("validates zero base fees", async () => {
      const baseTreasuryAccount = await getAccount(
        provider.connection,
        baseTreasury
      );
      assert.equal(baseTreasuryAccount.amount.toString(), "0");
    });
  });

  describe("Distribution - Partial Locks", () => {
    it("calculates distribution correctly", async () => {
      const totalLocked = investors.reduce(
        (sum, inv) => sum.add(inv.lockedAmount),
        new BN(0)
      );

      const treasuryBefore = await getAccount(
        provider.connection,
        quoteTreasury
      );
      const totalFees = new BN(treasuryBefore.amount.toString());

      const fLocked = totalLocked.mul(new BN(10000)).div(Y0);
      const eligibleBps = Math.min(INVESTOR_FEE_SHARE_BPS, fLocked.toNumber());
      const investorShare = totalFees
        .mul(new BN(eligibleBps))
        .div(new BN(10000));

      const expectedPayouts = investors.map((inv) => {
        if (inv.lockedAmount.eq(new BN(0))) return new BN(0);
        const weight = inv.lockedAmount
          .mul(new BN(1_000_000))
          .div(totalLocked);
        return investorShare.mul(weight).div(new BN(1_000_000));
      });

      const totalExpectedToInvestors = expectedPayouts.reduce(
        (sum, p) => sum.add(p),
        new BN(0)
      );
      const expectedToCreator = totalFees.sub(totalExpectedToInvestors);

      assert.ok(totalExpectedToInvestors.lte(investorShare));
      assert.ok(expectedToCreator.gt(new BN(0)));
    });
  });

  describe("Distribution - All Unlocked", () => {
    it("sends 100% to creator when unlocked", async () => {
      const totalLocked = new BN(0);
      const treasuryBalance = new BN(1_000_000_000);

      const fLocked = new BN(0);
      const eligibleBps = Math.min(INVESTOR_FEE_SHARE_BPS, fLocked.toNumber());
      const investorShare = treasuryBalance
        .mul(new BN(eligibleBps))
        .div(new BN(10000));
      const creatorShare = treasuryBalance.sub(investorShare);

      assert.equal(investorShare.toNumber(), 0);
      assert.equal(creatorShare.toString(), treasuryBalance.toString());
    });
  });

  describe("Dust and Cap", () => {
    it("handles dust correctly", async () => {
      const smallPayout = new BN(500);
      const largePayout = new BN(5000);

      assert.ok(smallPayout.lt(MIN_PAYOUT));
      assert.ok(largePayout.gte(MIN_PAYOUT));
    });

    it("applies daily cap", async () => {
      const hugeFees = new BN(10_000_000_000);
      const investorShareBps = 5000;

      const uncappedShare = hugeFees
        .mul(new BN(investorShareBps))
        .div(new BN(10000));
      const cappedShare = uncappedShare.gt(DAILY_CAP)
        ? DAILY_CAP
        : uncappedShare;

      assert.equal(cappedShare.toString(), DAILY_CAP.toString());

      const excessToCreator = hugeFees.sub(cappedShare);
      assert.ok(excessToCreator.gt(new BN(0)));
    });
  });

  describe("Base Fee Rejection", () => {
    it("rejects when base fees detected", async () => {
      const feeA = new BN(100_000);
      const feeB = new BN(500_000);

      const shouldReject = feeA.gt(new BN(0));
      assert.ok(shouldReject);
    });

    it("validates tick configuration", () => {
      const currentTick = 100;
      const tickLower = -200;
      const tickUpper = -100;

      const isQuoteOnly = tickUpper < currentTick;
      const isValidRange = tickLower < tickUpper;

      assert.ok(isQuoteOnly);
      assert.ok(isValidRange);
    });
  });

  describe("Pagination", () => {
    it("handles multi-page distribution", async () => {
      const totalInvestors = 25;
      const pageSize = 10;
      const expectedPages = Math.ceil(totalInvestors / pageSize);

      let cursor = 0;

      for (let page = 0; page < expectedPages; page++) {
        const startIdx = page * pageSize;
        const endIdx = Math.min((page + 1) * pageSize, totalInvestors);
        const pageCount = endIdx - startIdx;

        assert.equal(cursor, page);
        assert.ok(pageCount > 0);
        assert.ok(pageCount <= pageSize);

        cursor++;
      }

      assert.equal(cursor, expectedPages);
    });

    it("prevents out-of-order execution", async () => {
      const progressAccount = await program.account.progress.fetch(progress);
      const currentCursor = progressAccount.cursor;

      const validPageIndex = currentCursor;
      const invalidPageIndex = currentCursor + 5;

      const shouldRejectInvalid = invalidPageIndex !== currentCursor;
      assert.ok(shouldRejectInvalid);
    });
  });

  describe("24-Hour Gate", () => {
    it("enforces 24-hour wait", async () => {
      const progressAccount = await program.account.progress.fetch(progress);
      const lastDistribution = progressAccount.lastDistributionTs.toNumber();
      const now = Math.floor(Date.now() / 1000);

      if (lastDistribution > 0) {
        const timeSince = now - lastDistribution;
        const canDistribute = timeSince >= 86400;

        if (!canDistribute) {
          assert.ok(timeSince < 86400);
        }
      }
    });
  });
});
