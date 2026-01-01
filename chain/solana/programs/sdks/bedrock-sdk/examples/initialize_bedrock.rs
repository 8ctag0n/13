//! Initialize the Bedrock program using bedrock-sdk
//!
//! This example initializes the marketplace with the ZK and FHE generator programs.
//!
//! Run with: cargo run --example initialize_bedrock
//!
//! Required environment variables:
//! - SOLANA_RPC_URL (default: http://localhost:8899)
//! - PROGRAM_ID or BEDROCK_PROGRAM_ID
//! - ZK_GENERATOR_PROGRAM_ID
//! - FHE_GENERATOR_PROGRAM_ID
//! - KEYPAIR_PATH (default: ~/.config/solana/id.json)

use anyhow::Result;
use bedrock_sdk::{derive_config_pda, instructions};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::env;

fn main() -> Result<()> {
    // Get configuration from environment
    let rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());

    let program_id_str = env::var("BEDROCK_PROGRAM_ID")
        .or_else(|_| env::var("PROGRAM_ID"))
        .expect("BEDROCK_PROGRAM_ID or PROGRAM_ID required");
    let program_id = program_id_str.trim().parse()
        .expect("Invalid program ID format");

    let zk_generator_str = env::var("ZK_GENERATOR_PROGRAM_ID")
        .expect("ZK_GENERATOR_PROGRAM_ID required");
    let zk_generator = zk_generator_str.trim().parse()
        .expect("Invalid ZK generator program ID");

    let fhe_generator_str = env::var("FHE_GENERATOR_PROGRAM_ID")
        .expect("FHE_GENERATOR_PROGRAM_ID required");
    let fhe_generator = fhe_generator_str.trim().parse()
        .expect("Invalid FHE generator program ID");

    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| format!("{}/.config/solana/id.json", env::var("HOME").unwrap()));

    println!("Initializing Bedrock Program");
    println!("  RPC URL: {}", rpc_url);
    println!("  Program ID: {}", program_id);
    println!("  ZK Generator: {}", zk_generator);
    println!("  FHE Generator: {}", fhe_generator);
    println!();

    // Load admin keypair
    let admin = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair from {}: {}", keypair_path, e))?;
    println!("  Admin: {}", admin.pubkey());

    // Derive config PDA
    let (config_pda, bump) = derive_config_pda(&program_id);
    println!("  Config PDA: {} (bump: {})", config_pda, bump);
    println!();

    // Create RPC client
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Check if already initialized
    match client.get_account(&config_pda) {
        Ok(account) => {
            if !account.data.is_empty() {
                println!("Config account already exists with {} bytes", account.data.len());
                println!("Skipping initialization (already initialized)");
                return Ok(());
            }
        }
        Err(_) => {
            println!("Config account does not exist, proceeding with initialization...");
        }
    }

    // Build initialize instruction
    let ix = instructions::initialize(
        &program_id,
        &admin.pubkey(),
        &config_pda,
        &zk_generator,
        &fhe_generator,
    );

    println!("Sending initialization transaction...");

    // Get recent blockhash and create transaction
    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        recent_blockhash,
    );

    // Send and confirm
    let signature = client.send_and_confirm_transaction_with_spinner(&tx)?;

    println!("Program initialized successfully!");
    println!("  Signature: {}", signature);
    println!();

    Ok(())
}
