//! Example: Register a prover on the Bedrock program
//!
//! This example demonstrates how to:
//! 1. Derive the prover PDA
//! 2. Build the RegisterProver instruction
//! 3. Send the transaction
//!
//! Run with: cargo run --example register_prover

use bedrock_sdk::{derive_prover_pda, instructions};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

fn main() {
    // In production, load these from config or CLI args
    let bedrock_program_id = solana_sdk::pubkey!("BeDrockXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

    // Generate a new prover keypair (in production, load from file)
    let prover = Keypair::new();

    println!("Prover wallet: {}", prover.pubkey());

    // Derive the prover PDA
    let (prover_pda, bump) = derive_prover_pda(&bedrock_program_id, &prover.pubkey());

    println!("Prover PDA: {} (bump: {})", prover_pda, bump);

    // Build the instruction
    let stake_amount = 100_000_000; // 0.1 SOL
    let ix = instructions::register_prover(
        &bedrock_program_id,
        &prover.pubkey(),
        &prover_pda,
        stake_amount,
    );

    println!("\nInstruction built:");
    println!("  Program: {}", ix.program_id);
    println!("  Accounts: {}", ix.accounts.len());
    println!("  Data length: {} bytes", ix.data.len());

    // In a real application, you would:
    // 1. Get a recent blockhash
    // 2. Create and sign a transaction
    // 3. Send it to the cluster
    //
    // Example:
    // let recent_blockhash = rpc_client.get_latest_blockhash()?;
    // let mut transaction = Transaction::new_with_payer(&[ix], Some(&prover.pubkey()));
    // transaction.sign(&[&prover], recent_blockhash);
    // let signature = rpc_client.send_and_confirm_transaction(&transaction)?;

    println!("\nReady to send transaction!");
    println!("Note: This is a demonstration. Add RPC client to actually send.");
}
