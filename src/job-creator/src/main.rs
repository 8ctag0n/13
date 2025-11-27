use anyhow::{Context, Result};
use zyberlink_sdk::MarketplaceSDK;
use zyberlink_types::fhe::{FheConsensusConfig, FheOperation};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::time::Duration;
use tfhe::{prelude::*, ConfigBuilder, FheUint8, generate_keys};
use blake2::{Blake2s256, Digest};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WitnessUploadResponse {
    commitment: String,
}

/// Job Creator - Continuously creates FHE jobs to test the full system
///
/// This program:
/// 1. Creates FHE encrypted data
/// 2. Submits jobs to the Solana program on localnet
/// 3. Monitors job status
/// 4. Reports on prover claims and completions
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Load configuration
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id: solana_sdk::pubkey::Pubkey = std::env::var("PROGRAM_ID")
        .context("PROGRAM_ID env var required")?
        .parse()
        .context("Invalid PROGRAM_ID")?;
    let backend_url = std::env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Load or create user keypair
    let keypair_path = std::env::var("USER_KEYPAIR")
        .unwrap_or_else(|_| "/tmp/job-creator-keypair.json".to_string());

    let user_keypair = if std::path::Path::new(&keypair_path).exists() {
        log::info!("Loading keypair from {}", keypair_path);
        read_keypair_file(&keypair_path)
            .context("Failed to read keypair")?
    } else {
        log::info!("Creating new keypair at {}", keypair_path);
        let keypair = Keypair::new();
        write_keypair_file(&keypair, &keypair_path)
            .context("Failed to write keypair")?;
        keypair
    };

    log::info!("Job Creator starting...");
    log::info!("RPC URL: {}", rpc_url);
    log::info!("Program ID: {}", program_id);
    log::info!("Backend URL: {}", backend_url);
    log::info!("User: {}", user_keypair.pubkey());

    // Create HTTP client for backend
    let http_client = reqwest::Client::new();

    // Connect to RPC
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Check balance
    let balance = rpc_client.get_balance(&user_keypair.pubkey())?;
    log::info!("User balance: {} SOL", balance as f64 / 1_000_000_000.0);

    if balance < 100_000_000 {
        log::warn!("Low balance! Requesting airdrop...");
        let signature = rpc_client.request_airdrop(&user_keypair.pubkey(), 1_000_000_000)?;
        log::info!("Airdrop signature: {}", signature);

        // Wait for confirmation
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Initialize SDK
    let sdk = MarketplaceSDK::new(program_id);

    log::info!("");
    log::info!("===========================================");
    log::info!("  Job Creator - Continuous Mode");
    log::info!("===========================================");
    log::info!("");
    log::info!("Creating FHE jobs every 10 seconds...");
    log::info!("Press Ctrl+C to stop");
    log::info!("");

    let mut job_counter = 0u64;

    loop {
        job_counter += 1;

        log::info!("--- Job #{} ---", job_counter);

        // Rotate through different operations for variety
        let operation = match job_counter % 5 {
            0 => {
                log::info!("Creating Tier 1 job: Add(42)");
                FheOperation::Add(42)
            }
            1 => {
                log::info!("Creating Tier 1 job: Multiply(7)");
                FheOperation::Multiply(7)
            }
            2 => {
                log::info!("Creating Tier 2 job: Sum (100 items)");
                FheOperation::Sum { expected_count: 100 }
            }
            3 => {
                log::info!("Creating Tier 3 job: Threshold(50)");
                FheOperation::Threshold {
                    threshold: 50,
                    greater_or_equal: true,
                }
            }
            4 => {
                log::info!("Creating Tier 4 job: Average (100 items)");
                FheOperation::Average { expected_count: 100 }
            }
            _ => unreachable!(),
        };

        // Create FHE encrypted data
        let (encrypted_data, server_key) = match create_fhe_data(&operation) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to create FHE data: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        log::info!("FHE data created ({} bytes encrypted, {} bytes server key)",
            encrypted_data.len(), server_key.len());

        // Get cost config for this operation
        let cost_config = operation.get_cost_config();
        log::info!("Tier {}: {} lamports/prover ({} SOL)",
            cost_config.complexity_tier,
            cost_config.min_payment_lamports,
            cost_config.min_payment_lamports as f64 / 1_000_000_000.0);

        // Create job instruction
        let required_provers = 3u8;
        let consensus_threshold = 2u8; // 2 out of 3

        // Combine encrypted data and server key into single input
        // Format: [encrypted_data_len (4 bytes)] [encrypted_data] [server_key]
        let mut encrypted_input = Vec::new();
        encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
        encrypted_input.extend_from_slice(&encrypted_data);
        encrypted_input.extend_from_slice(&server_key);

        // Upload witness to backend
        log::info!("Uploading witness to backend ({} bytes)...", encrypted_input.len());

        // Compute local commitment for verification (must match SDK's Blake2s256)
        let mut hasher = Blake2s256::new();
        hasher.update(&encrypted_input);
        let local_commitment = hex::encode(hasher.finalize());

        let upload_url = format!("{}/witness", backend_url);
        let upload_result = http_client
            .post(&upload_url)
            .body(encrypted_input.clone())
            .header("Content-Type", "application/octet-stream")
            .send()
            .await;

        let backend_commitment = match upload_result {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<WitnessUploadResponse>().await {
                        Ok(resp) => {
                            log::info!("Witness uploaded, commitment: {}", resp.commitment);
                            resp.commitment
                        }
                        Err(e) => {
                            log::error!("Failed to parse upload response: {}", e);
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                } else {
                    log::error!("Backend returned error: {}", response.status());
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            }
            Err(e) => {
                log::error!("Failed to upload witness: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        // Verify commitment matches
        if backend_commitment != local_commitment {
            log::error!("Commitment mismatch! Local: {}, Backend: {}",
                local_commitment, backend_commitment);
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }

        log::info!("Commitment verified: {}", local_commitment);

        let fhe_config = FheConsensusConfig {
            required_provers,
            consensus_threshold,
            submission_timeout_secs: cost_config.timeout_seconds,
            operation: operation.clone(),
        };

        // Calculate total price (payment per prover * number of provers)
        // Use 2x minimum to ensure profitability for provers (covers 1.5x cost multiplier + 20% ROI)
        let total_price_lamports = cost_config.min_payment_lamports * 2 * (required_provers as u64);

        // Get the next job ID from on-chain config
        let job_id = match sdk.get_next_job_id(&rpc_client) {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to get next job ID: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        log::info!("Using job ID: {}", job_id);

        // Build create job instruction using new SDK
        let create_job_ix = sdk.create_fhe_job(
                user_keypair.pubkey(),
                job_id, // Use ID from on-chain config
                &encrypted_input,
                fhe_config,
                total_price_lamports,
                cost_config.timeout_seconds,
            )
            .context("Failed to create instruction")?;

        // Submit transaction
        let recent_blockhash = rpc_client.get_latest_blockhash()?;
        let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&user_keypair.pubkey()));
        tx.sign(&[&user_keypair], recent_blockhash);

        match rpc_client.send_and_confirm_transaction(&tx) {
            Ok(signature) => {
                log::info!("✓ Job created! Signature: {}", signature);
                log::info!("  View: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http://localhost:8899", signature);
            }
            Err(e) => {
                log::error!("✗ Failed to create job: {}", e);
            }
        }

        log::info!("");

        // Wait before creating next job
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

/// Create FHE encrypted data for testing
fn create_fhe_data(_operation: &FheOperation) -> Result<(Vec<u8>, Vec<u8>)> {
    // Generate FHE keys (in production, these would be cached/reused)
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    // Create sample encrypted input
    // For testing, we'll encrypt a simple value
    let value = 10u8;
    let encrypted = FheUint8::encrypt(value, &client_key);

    // Serialize encrypted data
    let encrypted_bytes = bincode::serialize(&encrypted)
        .context("Failed to serialize encrypted data")?;

    // Serialize server key
    let server_key_bytes = bincode::serialize(&server_key)
        .context("Failed to serialize server key")?;

    Ok((encrypted_bytes, server_key_bytes))
}

/// Read keypair from file
fn read_keypair_file(path: &str) -> Result<Keypair> {
    let contents = std::fs::read_to_string(path)
        .context("Failed to read keypair file")?;

    let bytes: Vec<u8> = serde_json::from_str(&contents)
        .context("Failed to parse keypair JSON")?;

    Keypair::try_from(bytes.as_slice())
        .context("Invalid keypair bytes")
}

/// Write keypair to file
fn write_keypair_file(keypair: &Keypair, path: &str) -> Result<()> {
    let bytes = keypair.to_bytes();
    let json = serde_json::to_string(&bytes.to_vec())
        .context("Failed to serialize keypair")?;

    std::fs::write(path, json)
        .context("Failed to write keypair file")?;

    Ok(())
}
