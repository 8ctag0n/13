use anyhow::Result;
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::{CircuitType, FheOperation};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::env;
use reqwest::Client;
use serde_json;

/// Simple example to create a test job
/// Usage: cargo run --example create_test_job -- [zk|fhe] [price_in_lamports]
#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let job_type = args.get(1).map(|s| s.as_str()).unwrap_or("zk");
    let price = args
        .get(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15_000_000); // Default 0.015 SOL

    // Get configuration from environment or defaults
    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id_str = env::var("PROGRAM_ID")
        .or_else(|_| std::fs::read_to_string("../logs/zyberlink_program_id.txt"))
        .or_else(|_| std::fs::read_to_string("/tmp/zyberlink_program_id.txt"))
        .expect("Program ID not found. Set PROGRAM_ID env var or run demo first.");
    let program_id = program_id_str
        .trim()
        .parse()
        .expect("Invalid program ID format");

    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| format!("{}/.config/solana/id.json", env::var("HOME").unwrap()));

    println!("🔧 Configuration:");
    println!("  RPC URL: {}", rpc_url);
    println!("  Program ID: {}", program_id);
    println!("  Keypair: {}", keypair_path);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    println!("👤 User: {}", keypair.pubkey());
    println!();

    // Create client
    let client = MarketplaceClient::new_with_commitment(
        rpc_url,
        program_id,
        CommitmentConfig::confirmed(),
    );

    // Determine circuit type and create appropriate job
    let (circuit_type, job_name) = match job_type {
        "fhe" => {
            println!("📊 Creating FHE Computation job...");
            (
                CircuitType::FheComputation(FheOperation::Add(10)),
                "FHE Add Operation"
            )
        }
        "zk" | _ => {
            println!("🔐 Creating ZK Zcash Orchard job...");
            (CircuitType::ZcashOrchard, "ZK Zcash Orchard Proof")
        }
    };

    println!("  Type: {}", job_name);
    println!("  Price: {} lamports ({:.4} SOL)", price, price as f64 / 1e9);
    println!();

    // Generate dummy witness data
    let witness_data = vec![42u8; 128];

    // Upload witness to storage backend
    println!("📤 Uploading witness to storage...");
    let http_client = Client::new();
    let witness_url = env::var("WITNESS_BACKEND_URL")
        .unwrap_or_else(|_| "http://localhost:3030".to_string());

    let upload_response = http_client
        .post(format!("{}/witness", witness_url))
        .body(witness_data.clone())
        .send()
        .await?;

    if !upload_response.status().is_success() {
        anyhow::bail!("Failed to upload witness: {}", upload_response.status());
    }

    let upload_result: serde_json::Value = upload_response.json().await?;
    let commitment_hex = upload_result["commitment"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No commitment in response"))?;

    // Convert hex commitment to bytes
    let commitment_bytes = hex::decode(commitment_hex)?;
    let mut witness_commitment = [0u8; 32];
    witness_commitment.copy_from_slice(&commitment_bytes[..32]);

    println!("  Witness uploaded: {}", commitment_hex);
    println!();

    println!("📝 Creating job...");

    // Get next job ID
    let job_id = client.fetch_next_job_id()?;
    println!("  Job ID: {}", job_id);

    // Create job instruction
    let ix = client.create_job_instruction(
        &keypair.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        witness_data.len() as u32,
        price,
        600, // 10 minutes timeout
        None, // No FHE consensus config for now
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

    println!("📤 Sending transaction...");

    // Send transaction
    let signature = client
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)?;

    println!("✅ Transaction confirmed!");
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Job created successfully!");
    println!("  Signature: {}", signature);
    println!("  Job ID: {}", job_id);
    println!("  Type: {}", job_name);
    println!("  Price: {:.4} SOL", price as f64 / 1e9);
    println!();
    println!("  👀 Watch the Prover logs to see it being processed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    Ok(())
}
