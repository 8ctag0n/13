use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use blake2::{Blake2s256, Digest};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::time::Duration;
use tfhe::{prelude::*, ClientKey, ConfigBuilder, FheUint8, generate_keys};
use zyberlink_sdk::MarketplaceSDK;
use zyberlink_types::fhe::{FheConsensusConfig, FheOperation, FhePredicate};

#[derive(Parser)]
#[command(name = "job-creator")]
#[command(about = "ZyberLink Job Creator - Create and verify FHE jobs")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run continuous job creation (default behavior)
    Continuous,
    /// Run a single verified test: PoI CountIf with [15,20,25,17] >= 18 -> expect 2
    VerifyPoi,
    /// Run a single verified test: Sum [10,20,30] -> expect 60
    VerifySum,
}

#[derive(Debug, Deserialize)]
struct WitnessUploadResponse {
    commitment: String,
}

#[derive(Debug, Deserialize)]
struct JobStatusResponse {
    status: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FheResultResponse {
    encrypted_result: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Load configuration
    let rpc_url =
        std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id: solana_sdk::pubkey::Pubkey = std::env::var("PROGRAM_ID")
        .context("PROGRAM_ID env var required")?
        .parse()
        .context("Invalid PROGRAM_ID")?;
    let backend_url =
        std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Load or create user keypair
    let keypair_path =
        std::env::var("USER_KEYPAIR").unwrap_or_else(|_| "/tmp/job-creator-keypair.json".to_string());

    let user_keypair = if std::path::Path::new(&keypair_path).exists() {
        log::info!("Loading keypair from {}", keypair_path);
        read_keypair_file(&keypair_path).context("Failed to read keypair")?
    } else {
        log::info!("Creating new keypair at {}", keypair_path);
        let keypair = Keypair::new();
        write_keypair_file(&keypair, &keypair_path).context("Failed to write keypair")?;
        keypair
    };

    log::info!("Job Creator starting...");
    log::info!("RPC URL: {}", rpc_url);
    log::info!("Program ID: {}", program_id);
    log::info!("Backend URL: {}", backend_url);
    log::info!("User: {}", user_keypair.pubkey());

    let http_client = reqwest::Client::new();
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());

    // Check and fund balance
    ensure_balance(&rpc_client, &user_keypair).await?;

    let sdk = MarketplaceSDK::new(program_id);

    match cli.command.unwrap_or(Commands::Continuous) {
        Commands::Continuous => {
            run_continuous(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyPoi => {
            run_verify_poi(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifySum => {
            run_verify_sum(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
    }
}

/// Run PoI verification test: CountIf with [15,20,25,17] >= 18 -> expect 2
async fn run_verify_poi(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  PoI Verification Test");
    log::info!("  CountIf([15,20,25,17], >= 18) -> expect 2");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![15, 20, 25, 17];
    let expected_result: u8 = 2; // 20 and 25 are >= 18

    let operation = FheOperation::CountIf {
        predicate: FhePredicate::GreaterThan(17), // >= 18 means > 17
        expected_count: test_values.len() as u16,
    };

    run_verified_job(
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url,
        operation,
        &test_values,
        expected_result,
        "PoI CountIf",
    )
    .await
}

/// Run Sum verification test: Sum [10,20,30] -> expect 60
async fn run_verify_sum(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Sum Verification Test");
    log::info!("  Sum([10,20,30]) -> expect 60");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![10, 20, 30];
    let expected_result: u8 = 60;

    let operation = FheOperation::Sum {
        expected_count: test_values.len() as u16,
    };

    run_verified_job(
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url,
        operation,
        &test_values,
        expected_result,
        "Sum",
    )
    .await
}

/// Run a verified job end-to-end
async fn run_verified_job(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
    operation: FheOperation,
    test_values: &[u8],
    expected_result: u8,
    test_name: &str,
) -> Result<()> {
    // Step 1: Generate FHE keys and encrypt data
    log::info!("[1/6] Generating FHE keys and encrypting data...");
    let (encrypted_data, server_key, client_key) = create_fhe_data_with_values(test_values)?;
    log::info!(
        "  Encrypted {} values ({} bytes data, {} bytes server key)",
        test_values.len(),
        encrypted_data.len(),
        server_key.len()
    );

    // Step 2: Prepare and upload witness
    log::info!("[2/6] Uploading witness to backend...");
    let mut encrypted_input = Vec::new();
    encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
    encrypted_input.extend_from_slice(&encrypted_data);
    encrypted_input.extend_from_slice(&server_key);

    let commitment = upload_witness(http_client, backend_url, &encrypted_input).await?;
    log::info!("  Commitment: {}", commitment);

    // Step 3: Create job on-chain
    log::info!("[3/6] Creating job on-chain...");
    let cost_config = operation.get_cost_config();
    let required_provers = 3u8;
    let consensus_threshold = 2u8;

    let fhe_config = FheConsensusConfig {
        required_provers,
        consensus_threshold,
        submission_timeout_secs: cost_config.timeout_seconds,
        operation: operation.clone(),
    };

    let total_price = cost_config.min_payment_lamports * 2 * (required_provers as u64);
    let job_id = sdk.get_next_job_id(rpc_client)?;

    let create_job_ix = sdk.create_fhe_job(
        user_keypair.pubkey(),
        job_id,
        &encrypted_input,
        fhe_config,
        total_price,
        cost_config.timeout_seconds,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&user_keypair.pubkey()));
    tx.sign(&[user_keypair], recent_blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    log::info!("  Job {} created, signature: {}", job_id, signature);

    // Step 4: Poll for completion
    log::info!("[4/6] Waiting for job completion (timeout: {}s)...", cost_config.timeout_seconds);
    let final_status = poll_job_status(http_client, backend_url, job_id, cost_config.timeout_seconds as u64).await?;

    if final_status != "completed" {
        log::error!("  Job failed with status: {}", final_status);
        log::error!("");
        log::error!("===========================================");
        log::error!("  {} Test: FAILED (job status: {})", test_name, final_status);
        log::error!("===========================================");
        return Ok(());
    }
    log::info!("  Job completed!");

    // Step 5: Fetch encrypted result
    log::info!("[5/6] Fetching encrypted result...");
    let encrypted_result = fetch_result(http_client, backend_url, job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    // Step 6: Decrypt and verify
    log::info!("[6/6] Decrypting and verifying result...");
    let decrypted = decrypt_result(&encrypted_result, &client_key)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  {} Test: PASSED", test_name);
        log::info!("===========================================");
    } else {
        log::error!("===========================================");
        log::error!("  {} Test: FAILED", test_name);
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
    }

    Ok(())
}

/// Create FHE encrypted data with specific values, returning client_key for later decryption
fn create_fhe_data_with_values(values: &[u8]) -> Result<(Vec<u8>, Vec<u8>, ClientKey)> {
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    let mut encrypted_values: Vec<Vec<u8>> = Vec::with_capacity(values.len());
    for &value in values {
        let encrypted = FheUint8::encrypt(value, &client_key);
        let enc_bytes = bincode::serialize(&encrypted)?;
        encrypted_values.push(enc_bytes);
    }

    let encrypted_bytes = bincode::serialize(&encrypted_values)?;
    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted_bytes, server_key_bytes, client_key))
}

/// Decrypt a result using the client key
fn decrypt_result(encrypted_result: &[u8], client_key: &ClientKey) -> Result<u8> {
    let encrypted: FheUint8 = bincode::deserialize(encrypted_result)?;
    let decrypted: u8 = encrypted.decrypt(client_key);
    Ok(decrypted)
}

/// Upload witness to backend, return commitment
async fn upload_witness(
    http_client: &reqwest::Client,
    backend_url: &str,
    data: &[u8],
) -> Result<String> {
    let mut hasher = Blake2s256::new();
    hasher.update(data);
    let local_commitment = hex::encode(hasher.finalize());

    let url = format!("{}/witness", backend_url);
    let response = http_client
        .post(&url)
        .body(data.to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Backend returned error: {}", response.status());
    }

    let resp: WitnessUploadResponse = response.json().await?;

    if resp.commitment != local_commitment {
        anyhow::bail!(
            "Commitment mismatch! Local: {}, Backend: {}",
            local_commitment,
            resp.commitment
        );
    }

    Ok(resp.commitment)
}

/// Poll job status until completed or failed
async fn poll_job_status(
    http_client: &reqwest::Client,
    backend_url: &str,
    job_id: u64,
    timeout_secs: u64,
) -> Result<String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs + 60); // Extra margin
    let poll_interval = Duration::from_secs(10);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Job polling timeout after {}s", timeout_secs);
        }

        let url = format!("{}/api/jobs/{}", backend_url, job_id);
        let response = http_client.get(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let status: JobStatusResponse = resp.json().await?;
                log::info!("  Status: {} (elapsed: {:?})", status.status, start.elapsed());

                match status.status.as_str() {
                    "completed" => return Ok("completed".to_string()),
                    "failed" => {
                        return Ok(format!("failed: {}", status.error.unwrap_or_default()))
                    }
                    "expired" => return Ok("expired".to_string()),
                    _ => {}
                }
            }
            Ok(resp) => {
                log::warn!("  Status check returned: {}", resp.status());
            }
            Err(e) => {
                log::warn!("  Status check failed: {}", e);
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Fetch encrypted result from backend
async fn fetch_result(
    http_client: &reqwest::Client,
    backend_url: &str,
    job_id: u64,
) -> Result<Vec<u8>> {
    // Use the new endpoint that fetches result by job_id
    let url = format!("{}/api/jobs/{}/result", backend_url, job_id);
    let response = http_client.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch result: {}", response.status());
    }

    let result: FheResultResponse = response.json().await?;
    let bytes = BASE64.decode(&result.encrypted_result)?;
    Ok(bytes)
}

/// Ensure user has enough balance
async fn ensure_balance(rpc_client: &RpcClient, keypair: &Keypair) -> Result<()> {
    let balance = rpc_client.get_balance(&keypair.pubkey())?;
    log::info!("User balance: {} SOL", balance as f64 / 1_000_000_000.0);

    if balance < 100_000_000 {
        log::warn!("Low balance! Requesting airdrop...");
        let signature = rpc_client.request_airdrop(&keypair.pubkey(), 1_000_000_000)?;
        log::info!("Airdrop signature: {}", signature);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

/// Run continuous job creation (original behavior)
async fn run_continuous(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
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

        let operation = match job_counter % 6 {
            0 => {
                log::info!("Creating Tier 1 job: Add(42)");
                FheOperation::Add(42)
            }
            1 => {
                log::info!("Creating Tier 1 job: Multiply(7)");
                FheOperation::Multiply(7)
            }
            2 => {
                log::info!("Creating Tier 2 job: Sum (5 items)");
                FheOperation::Sum { expected_count: 5 }
            }
            3 => {
                log::info!("Creating Tier 3 job: Threshold(50)");
                FheOperation::Threshold {
                    threshold: 50,
                    greater_or_equal: true,
                }
            }
            4 => {
                log::info!("Creating Tier 4 job: Average (5 items)");
                FheOperation::Average { expected_count: 5 }
            }
            5 => {
                log::info!("Creating Tier 4 job: CountIf >= 18 (PoI)");
                FheOperation::CountIf {
                    predicate: FhePredicate::GreaterThan(17),
                    expected_count: 4,
                }
            }
            _ => unreachable!(),
        };

        let (encrypted_data, server_key) = match create_fhe_data(&operation) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to create FHE data: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        log::info!(
            "FHE data created ({} bytes encrypted, {} bytes server key)",
            encrypted_data.len(),
            server_key.len()
        );

        let cost_config = operation.get_cost_config();
        log::info!(
            "Tier {}: {} lamports/prover ({} SOL)",
            cost_config.complexity_tier,
            cost_config.min_payment_lamports,
            cost_config.min_payment_lamports as f64 / 1_000_000_000.0
        );

        let required_provers = 3u8;
        let consensus_threshold = 2u8;

        let mut encrypted_input = Vec::new();
        encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
        encrypted_input.extend_from_slice(&encrypted_data);
        encrypted_input.extend_from_slice(&server_key);

        log::info!(
            "Uploading witness to backend ({} bytes)...",
            encrypted_input.len()
        );

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

        if backend_commitment != local_commitment {
            log::error!(
                "Commitment mismatch! Local: {}, Backend: {}",
                local_commitment,
                backend_commitment
            );
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

        let total_price_lamports = cost_config.min_payment_lamports * 2 * (required_provers as u64);

        let job_id = match sdk.get_next_job_id(rpc_client) {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to get next job ID: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        log::info!("Using job ID: {}", job_id);

        let create_job_ix = sdk
            .create_fhe_job(
                user_keypair.pubkey(),
                job_id,
                &encrypted_input,
                fhe_config,
                total_price_lamports,
                cost_config.timeout_seconds,
            )
            .context("Failed to create instruction")?;

        let recent_blockhash = rpc_client.get_latest_blockhash()?;
        let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&user_keypair.pubkey()));
        tx.sign(&[user_keypair], recent_blockhash);

        match rpc_client.send_and_confirm_transaction(&tx) {
            Ok(signature) => {
                log::info!("Job created! Signature: {}", signature);
            }
            Err(e) => {
                log::error!("Failed to create job: {}", e);
            }
        }

        log::info!("");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

/// Create FHE encrypted data for testing (continuous mode)
fn create_fhe_data(operation: &FheOperation) -> Result<(Vec<u8>, Vec<u8>)> {
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    let encrypted_bytes = match operation {
        FheOperation::Sum { expected_count }
        | FheOperation::Average { expected_count }
        | FheOperation::CountIf { expected_count, .. } => {
            let count = *expected_count as usize;
            let mut encrypted_values: Vec<Vec<u8>> = Vec::with_capacity(count);

            for i in 0..count {
                let value = ((i % 10) + 1) as u8;
                let encrypted = FheUint8::encrypt(value, &client_key);
                let enc_bytes = bincode::serialize(&encrypted)?;
                encrypted_values.push(enc_bytes);
            }

            bincode::serialize(&encrypted_values)?
        }
        _ => {
            let value = 10u8;
            let encrypted = FheUint8::encrypt(value, &client_key);
            bincode::serialize(&encrypted)?
        }
    };

    let server_key_bytes = bincode::serialize(&server_key)?;
    Ok((encrypted_bytes, server_key_bytes))
}

fn read_keypair_file(path: &str) -> Result<Keypair> {
    let contents = std::fs::read_to_string(path)?;
    let bytes: Vec<u8> = serde_json::from_str(&contents)?;
    Keypair::try_from(bytes.as_slice()).context("Invalid keypair bytes")
}

fn write_keypair_file(keypair: &Keypair, path: &str) -> Result<()> {
    let bytes = keypair.to_bytes();
    let json = serde_json::to_string(&bytes.to_vec())?;
    std::fs::write(path, json)?;
    Ok(())
}
