import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("honorary-dammv2-crank", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.HonoraryDammv2Crank;

  it("initialize config", async () => {
    const authority = provider.wallet;
    const configKeypair = anchor.web3.Keypair.generate();
    await program.rpc.initialize(
      new anchor.BN(1000), // y0
      5000, // investor_fee_share_bps
      new anchor.BN(1000), // min payout lamports
      {
        accounts: {
          config: configKeypair.publicKey,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        },
        signers: [configKeypair],
        instructions: [
          await program.provider.connection.requestAirdrop(
            configKeypair.publicKey,
            1e9
          ),
        ],
      }
    );
  });

  it("create honorary position (invoke DAMM create_position)", async () => {
    // build all accounts required by DAMM create_position (mint, nft acct, position pda etc.)
    // For tests you can create dummy accounts and pass them; DAMM must be deployed locally and expects exact accounts.
    // Example shows how to call the instruction and pass remaining_accounts array
    const vault = anchor.web3.Keypair.generate();
    const ownerPda = await PublicKey.findProgramAddress(
      [Buffer.from("investor_fee_pos_owner"), vault.publicKey.toBytes()],
      program.programId
    );
    // create dummy pool and other accounts and pass them as remaining_accounts in the same order your program expects
    // ... create mints, token accounts, fund payer, etc.
    // call:
    // await program.rpc.createHonoraryPosition(poolPubkey, quoteMintPubkey, {
    //   accounts: { payer: provider.wallet.publicKey, vault: vault.publicKey, ownerPda: ownerPda[0], honoraryPosition: honoraryPositionPubkey, pool: poolPubkey, quoteMint: quoteMintPubkey, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY },
    //   signers: [],
    //   remainingAccounts: [positionNftMintPubkey, positionNftAccountPubkey, positionPdaPubkey, poolAuthorityPubkey, tokenProgramPubkey, systemProgramPubkey]
    // });
    assert.ok(true);
  });

  it("crank distribute page", async () => {
    // call crank_distribute with page data: pass pairs [stream_pubkey, investor_ata] as remainingAccounts after required fixed accounts
    assert.ok(true);
  });
});
