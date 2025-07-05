import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CyrusSolana } from "../target/types/cyrus_solana";
import { expect } from "chai";

describe("cyrus-solana", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.CyrusSolana as Program<CyrusSolana>;
  const provider = anchor.getProvider();

  it("Emits settlement request", async () => {
    console.log("🧪 Testing Cyrus Solana Settlement Emitter");
    
    const aptosRecipient = "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd";
    const amountUsdc = 1_000_000; // 1 USDC in micro units
    
    console.log(`Requesting settlement:`);
    console.log(`Amount: ${amountUsdc / 1_000_000} USDC`);
    console.log(`Aptos Recipient: ${aptosRecipient}`);
    
    // Call the settlement request function
    const tx = await program.methods
      .emitSettlement(aptosRecipient, new anchor.BN(amountUsdc))
      .accounts({
        user: provider.wallet.publicKey,
        instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log(`✅ Transaction signature: ${tx}`);
    console.log(`🔗 Explorer: https://explorer.solana.com/tx/${tx}?cluster=devnet`);
    
    // Wait for confirmation
    await provider.connection.confirmTransaction(tx);
    
    console.log("Settlement request emitted successfully!");
    console.log("");
    console.log("For relayer integration:");
    console.log(`   Source TX Hash: ${tx}`);
    console.log(`   Receiver: ${aptosRecipient}`);
    console.log(`   Amount: ${amountUsdc}`);
    console.log(`   Next: Configure relayer to listen for this transaction`);
  });

  it("Emits multiple settlements", async () => {
    console.log("\nTesting multiple settlements");
    
    const settlements = [
      {
        recipient: "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd",
        amount: 500_000 // 0.05 USDC
      },
      {
        recipient: "7LGeDHShvvvmT9sDWfjf5XEMbMYS888QQ4RK1E815yGp",
        amount: 1_000_000 // 0.1 USDC
      }
    ];
    
    for (let i = 0; i < settlements.length; i++) {
      const settlement = settlements[i];
      
      console.log(`\nSettlement ${i + 1}:`);
      console.log(`   Amount: ${settlement.amount / 1_000_000} USDC`);
      console.log(`   Recipient: ${settlement.recipient}`);
      
      const tx = await program.methods
        .emitSettlement(settlement.recipient, new anchor.BN(settlement.amount))
        .accounts({
          user: provider.wallet.publicKey,
          instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      console.log(`Transaction: ${tx}`);
      
      console.log(`Explorer: https://explorer.solana.com/tx/${tx}?cluster=devnet`);
      console.log(`Waiting for settlement ${i + 1} to be confirmed...`);
      await provider.connection.confirmTransaction(tx);
      console.log(`Settlement ${i + 1} emitted successfully!`);
      console.log(`Next: Configure relayer to listen for this transaction`);
      console.log("Waiting 1 second before next settlement...");

      // Waiting before next settlement to avoid rate limits
      console.log(`Waiting for settlement ${i + 1} to be confirmed...`);
      await provider.connection.confirmTransaction(tx);
      console.log(`Settlement ${i + 1} emitted successfully!`);
      console.log(`Next: Configure relayer to listen for this transaction`);
      console.log("Waiting 1 second before next settlement...");
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
    
    console.log("\nAll settlements emitted!");
  });
});