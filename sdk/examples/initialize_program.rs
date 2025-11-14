use anyhow::Result;
use cypherlink_sdk::MarketplaceClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::env;

/// Initialize the CypherLink program
fn main() -> Result<()> {
    // Get configuration
    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id_str = env::var("PROGRAM_ID")
        .or_else(|_| std::fs::read_to_string("../logs/zyberlink_program_id.txt"))
        .or_else(|_| std::fs::read_to_string("/tmp/zyberlink_program_id.txt"))
        .expect("Program ID not found");
    let program_id = program_id_str.trim().parse().expect("Invalid program ID");

    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| format!("{}/.config/solana/id.json", env::var("HOME").unwrap()));

    println!("🔧 Initializing CypherLink Program");
    println!("  RPC URL: {}", rpc_url);
    println!("  Program ID: {}", program_id);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    println!("  Authority: {}", keypair.pubkey());

    // Create client
    let client = MarketplaceClient::new_with_commitment(
        rpc_url,
        program_id,
        CommitmentConfig::confirmed(),
    );

    // Create initialize instruction
    let ix = client.initialize_instruction(
        &keypair.pubkey(),
        250,        // 2.5% fee
        100_000_000, // 0.1 SOL min stake
        0,          // 0 min reputation
        3600,       // 1 hour default timeout
    )?;

    // Get recent blockhash
    let recent_blockhash = client.rpc_client.get_latest_blockhash()?;

    // Create and sign transaction
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash,
    );

    println!("📤 Sending initialization transaction...");

    // Send transaction
    let signature = client.rpc_client.send_and_confirm_transaction_with_spinner(&tx)?;

    println!("✅ Program initialized successfully!");
    println!("  Signature: {}", signature);
    println!();

    Ok(())
}
