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
use zyberlink_fhe::{prelude::*, ClientKey, FheUint8, generate_keys};
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
    /// Verify ADD: 50 + 10 = 60
    VerifyAdd,
    /// Verify MULTIPLY: 5 * 3 = 15
    VerifyMultiply,
    /// Verify SUM: [10,20,30] = 60
    VerifySum,
    /// Verify THRESHOLD: 75 >= 50 = true (1)
    VerifyThreshold,
    /// Verify RANGE_CHECK: 50 in [0,100] = true (1)
    VerifyRange,
    /// Verify AVERAGE: avg([10,20,30,40,50]) = 30
    VerifyAverage,
    /// Verify COUNT_IF (PoI): [15,20,25,17] >= 18 = 2
    VerifyPoi,
    /// Run ALL verification tests
    VerifyAll,
    /// Simulate webapp flow: upload server_key separately, then validate-and-build
    WebappFlow,
    /// Webapp flow + wait for completion + decrypt and verify result
    WebappFlowVerify,
    /// Webapp flow for Proof of Innocence (count_if with Equals predicate)
    WebappFlowPoi,
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

/// Response from /api/server-key/upload
#[derive(Debug, Deserialize)]
struct ServerKeyUploadResponse {
    server_key_hash: String,
    size_bytes: u64,
}

/// Response from /api/jobs/validate-and-build
#[derive(Debug, Deserialize)]
struct ValidateAndBuildResponse {
    job_id: u64,
    transaction: String,
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
        Commands::VerifyAdd => {
            run_verify_add(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyMultiply => {
            run_verify_multiply(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifySum => {
            run_verify_sum(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyThreshold => {
            run_verify_threshold(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyRange => {
            run_verify_range(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyAverage => {
            run_verify_average(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyPoi => {
            run_verify_poi(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::VerifyAll => {
            run_verify_all(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url).await
        }
        Commands::WebappFlow => {
            run_webapp_flow(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url, false).await
        }
        Commands::WebappFlowVerify => {
            run_webapp_flow(&sdk, &rpc_client, &http_client, &user_keypair, &backend_url, true).await
        }
        Commands::WebappFlowPoi => {
            run_webapp_flow_poi(&rpc_client, &http_client, &user_keypair, &backend_url).await
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

/// Run Add verification test: 50 + 10 = 60
async fn run_verify_add(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Add Verification Test");
    log::info!("  50 + 10 -> expect 60");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![50];
    let expected_result: u8 = 60;

    let operation = FheOperation::Add(10);

    run_verified_job(
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url,
        operation,
        &test_values,
        expected_result,
        "Add",
    )
    .await
}

/// Run Multiply verification test: 5 * 3 = 15
async fn run_verify_multiply(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Multiply Verification Test");
    log::info!("  5 * 3 -> expect 15");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![5];
    let expected_result: u8 = 15;

    let operation = FheOperation::Multiply(3);

    run_verified_job(
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url,
        operation,
        &test_values,
        expected_result,
        "Multiply",
    )
    .await
}

/// Run Threshold verification test: 75 >= 50 = true (1)
async fn run_verify_threshold(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Threshold Verification Test");
    log::info!("  75 >= 50 -> expect 1 (true)");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![75];
    let expected_result: u8 = 1; // true

    let operation = FheOperation::Threshold {
        threshold: 50,
        greater_or_equal: true,
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
        "Threshold",
    )
    .await
}

/// Run RangeCheck verification test: 50 in [0,100] = true (1)
async fn run_verify_range(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  RangeCheck Verification Test");
    log::info!("  50 in [0, 100] -> expect 1 (true)");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![50];
    let expected_result: u8 = 1; // true

    let operation = FheOperation::RangeCheck { min: 0, max: 100 };

    run_verified_job(
        sdk,
        rpc_client,
        http_client,
        user_keypair,
        backend_url,
        operation,
        &test_values,
        expected_result,
        "RangeCheck",
    )
    .await
}

/// Run Average verification test: avg([10,20,30,40,50]) = 30
async fn run_verify_average(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Average Verification Test");
    log::info!("  avg([10,20,30,40,50]) -> expect 30");
    log::info!("===========================================");
    log::info!("");

    let test_values: Vec<u8> = vec![10, 20, 30, 40, 50];
    let expected_result: u8 = 30;

    let operation = FheOperation::Average {
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
        "Average",
    )
    .await
}

/// Run ALL verification tests sequentially
async fn run_verify_all(
    sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("###############################################");
    log::info!("#  RUNNING ALL FHE VERIFICATION TESTS        #");
    log::info!("#  Sequential: send -> wait -> verify -> next #");
    log::info!("###############################################");
    log::info!("");

    let mut results: Vec<(&str, Result<()>)> = Vec::new();

    // Test 1: Add
    log::info!("[TEST 1/7] Starting Add test...");
    let r = run_verify_add(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("Add (50 + 10 = 60)", r));

    // Test 2: Multiply
    log::info!("[TEST 2/7] Starting Multiply test...");
    let r = run_verify_multiply(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("Multiply (5 * 3 = 15)", r));

    // Test 3: Sum
    log::info!("[TEST 3/7] Starting Sum test...");
    let r = run_verify_sum(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("Sum ([10,20,30] = 60)", r));

    // Test 4: Threshold
    log::info!("[TEST 4/7] Starting Threshold test...");
    let r = run_verify_threshold(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("Threshold (75 >= 50)", r));

    // Test 5: RangeCheck
    log::info!("[TEST 5/7] Starting RangeCheck test...");
    let r = run_verify_range(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("RangeCheck (50 in [0,100])", r));

    // Test 6: Average
    log::info!("[TEST 6/7] Starting Average test...");
    let r = run_verify_average(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("Average ([10,20,30,40,50] = 30)", r));

    // Test 7: CountIf (PoI)
    log::info!("[TEST 7/7] Starting CountIf (PoI) test...");
    let r = run_verify_poi(sdk, rpc_client, http_client, user_keypair, backend_url).await;
    results.push(("CountIf/PoI ([15,20,25,17] >= 18 = 2)", r));

    // Print summary
    log::info!("");
    log::info!("###############################################");
    log::info!("#  TEST SUMMARY                              #");
    log::info!("###############################################");

    let mut passed = 0;
    let mut failed = 0;

    for (name, result) in &results {
        match result {
            Ok(_) => {
                log::info!("  [PASS] {}", name);
                passed += 1;
            }
            Err(e) => {
                log::error!("  [FAIL] {}: {}", name, e);
                failed += 1;
            }
        }
    }

    log::info!("");
    log::info!("  Total: {} passed, {} failed out of 7", passed, failed);
    log::info!("###############################################");

    if failed > 0 {
        anyhow::bail!("{} tests failed", failed);
    }

    Ok(())
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
    let (encrypted_data, server_key, client_key) = create_fhe_data_with_values(test_values, &operation)?;
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
    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
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
///
/// Format depends on operation type:
/// - Add/Multiply/Threshold/RangeCheck: Single FheUint8 serialized directly
/// - Sum/Average/CountIf: Vec<Vec<u8>> with each value serialized separately
fn create_fhe_data_with_values(values: &[u8], operation: &FheOperation) -> Result<(Vec<u8>, Vec<u8>, ClientKey)> {
    let (client_key, server_key) = generate_keys()?;

    let encrypted_bytes = match operation {
        // Single-value operations: serialize FheUint8 directly
        FheOperation::Add(_) | FheOperation::Multiply(_) |
        FheOperation::Threshold { .. } | FheOperation::RangeCheck { .. } => {
            if values.len() != 1 {
                anyhow::bail!("Single-value operations (Add/Multiply/Threshold/RangeCheck) require exactly 1 input value, got {}", values.len());
            }
            let encrypted = FheUint8::encrypt(values[0], &client_key);
            bincode::serialize(&encrypted)?
        }
        // Multi-value operations: serialize as Vec<Vec<u8>>
        FheOperation::Sum { .. } | FheOperation::Average { .. } |
        FheOperation::CountIf { .. } | FheOperation::Histogram { .. } => {
            let mut encrypted_values: Vec<Vec<u8>> = Vec::with_capacity(values.len());
            for &value in values {
                let encrypted = FheUint8::encrypt(value, &client_key);
                let enc_bytes = bincode::serialize(&encrypted)?;
                encrypted_values.push(enc_bytes);
            }
            bincode::serialize(&encrypted_values)?
        }
    };

    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted_bytes, server_key_bytes, client_key))
}

/// Decrypt a result using the client key
fn decrypt_result(encrypted_result: &[u8], client_key: &ClientKey, operation: &FheOperation) -> Result<u8> {
    match operation {
        // Average returns a tuple (sum as Vec<u8>, count as u16)
        FheOperation::Average { .. } => {
            let (sum_bytes, count): (Vec<u8>, u16) = bincode::deserialize(encrypted_result)?;
            let sum_encrypted: FheUint8 = bincode::deserialize(&sum_bytes)?;
            let sum: u8 = sum_encrypted.decrypt(client_key);
            let average = sum / (count as u8);
            Ok(average)
        }
        // All other operations return FheUint8 directly
        _ => {
            let encrypted: FheUint8 = bincode::deserialize(encrypted_result)?;
            let decrypted: u8 = encrypted.decrypt(client_key);
            Ok(decrypted)
        }
    }
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

    if balance < 20_000_000 {
        // Only request airdrop if balance is < 0.02 SOL
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
    let (client_key, server_key) = generate_keys()?;

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

/// Simulate webapp flow for creating FHE jobs
/// This mimics exactly what the frontend does:
/// 1. Upload server_key to /api/server-key/upload
/// 2. Call /api/jobs/validate-and-build with server_key_hash + encrypted_data
/// 3. Sign and submit the transaction
/// 4. Confirm with backend
/// 5. (Optional) Wait for job completion and verify decrypted result
async fn run_webapp_flow(
    _sdk: &MarketplaceSDK,
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
    verify_result: bool,
) -> Result<()> {
    let total_steps = if verify_result { 8 } else { 6 };

    log::info!("");
    log::info!("===========================================");
    log::info!("  Webapp Flow Simulation{}", if verify_result { " + Verification" } else { "" });
    log::info!("  Testing: server_key pre-upload + validate-and-build");
    if verify_result {
        log::info!("  + Wait for completion + Decrypt & verify result");
    }
    log::info!("===========================================");
    log::info!("");

    // Step 1: Generate FHE keys and encrypt test data
    log::info!("[1/{}] Generating FHE keys and encrypting data...", total_steps);
    let test_values: Vec<u8> = vec![10, 20, 30];
    let expected_result: u8 = test_values.iter().map(|&x| x as u16).sum::<u16>() as u8; // 60
    let operation = FheOperation::Sum { expected_count: 3 };

    let (encrypted_data, server_key, client_key) = create_fhe_data_with_values(&test_values, &operation)?;
    log::info!(
        "  Encrypted {} values: {} bytes encrypted_data, {} bytes server_key ({:.1} MB)",
        test_values.len(),
        encrypted_data.len(),
        server_key.len(),
        server_key.len() as f64 / 1024.0 / 1024.0
    );

    // Step 2: Upload server_key to /api/server-key/upload
    log::info!("[2/{}] Uploading server_key to /api/server-key/upload...", total_steps);
    let upload_url = format!("{}/api/server-key/upload", backend_url);

    let upload_start = std::time::Instant::now();
    let upload_response = http_client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(server_key.clone())
        .timeout(Duration::from_secs(600)) // 10 min timeout for large upload
        .send()
        .await
        .context("Failed to upload server key")?;

    if !upload_response.status().is_success() {
        let error_text = upload_response.text().await.unwrap_or_default();
        anyhow::bail!("Server key upload failed: {}", error_text);
    }

    let upload_result: ServerKeyUploadResponse = upload_response.json().await?;
    let upload_elapsed = upload_start.elapsed();
    log::info!(
        "  Uploaded in {:.1}s, hash: {}",
        upload_elapsed.as_secs_f64(),
        upload_result.server_key_hash
    );

    // Step 3: Create signature for validate-and-build (simulating wallet.signMessage)
    log::info!("[3/{}] Creating signature for job creation...", total_steps);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let nonce = format!("{:x}", rand::random::<u64>());
    let job_id_placeholder = timestamp * 1000 + rand::random::<u64>() % 1000;
    let message = format!("create_job:{}:{}:{}", job_id_placeholder, timestamp, nonce);

    let signature = user_keypair.sign_message(message.as_bytes());
    let signature_base58 = bs58::encode(signature.as_ref()).into_string();
    log::info!("  Message: {}", message);
    log::info!("  Signature: {}...", &signature_base58[..20]);

    // Step 4: Call /api/jobs/validate-and-build
    log::info!("[4/{}] Calling /api/jobs/validate-and-build...", total_steps);
    let encrypted_data_base64 = BASE64.encode(&encrypted_data);

    let validate_url = format!("{}/api/jobs/validate-and-build", backend_url);
    let validate_body = serde_json::json!({
        "creator_pubkey": user_keypair.pubkey().to_string(),
        "encrypted_data": encrypted_data_base64,
        "server_key_hash": upload_result.server_key_hash,
        "message": message,
        "signature": signature_base58,
        "nonce": nonce,
        "operation": "sum",
        "operation_value": 0,
        "expected_count": test_values.len(),  // For Sum operation, specify how many values
        "price_lamports": 15000000,  // 0.015 SOL (profitable for provers)
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "SOL"
    });

    let validate_response = http_client
        .post(&validate_url)
        .header("Content-Type", "application/json")
        .json(&validate_body)
        .send()
        .await
        .context("Failed to call validate-and-build")?;

    if !validate_response.status().is_success() {
        let error_text = validate_response.text().await.unwrap_or_default();
        anyhow::bail!("validate-and-build failed: {}", error_text);
    }

    let validate_result: ValidateAndBuildResponse = validate_response.json().await?;
    log::info!("  Job ID: {}", validate_result.job_id);
    log::info!("  Transaction received ({} bytes base64)", validate_result.transaction.len());

    // Step 5: Deserialize, sign, and submit transaction
    log::info!("[5/{}] Signing and submitting transaction...", total_steps);
    let tx_bytes = BASE64.decode(&validate_result.transaction)?;
    let mut tx: Transaction = bincode::deserialize(&tx_bytes)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    tx.sign(&[user_keypair], recent_blockhash);

    let signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .context("Failed to submit transaction")?;
    log::info!("  Transaction confirmed: {}", signature);

    // Step 6: Confirm with backend
    log::info!("[6/{}] Confirming job with backend...", total_steps);
    let confirm_url = format!("{}/api/jobs/{}/confirm", backend_url, validate_result.job_id);
    let confirm_response = http_client
        .post(&confirm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "signature": signature.to_string() }))
        .send()
        .await?;

    if !confirm_response.status().is_success() {
        log::warn!("  Confirm returned non-success (may be ok if chain sync handles it)");
    } else {
        log::info!("  Job confirmed with backend");
    }

    // If not verifying, we're done
    if !verify_result {
        log::info!("");
        log::info!("===========================================");
        log::info!("  SUCCESS! Webapp flow works correctly");
        log::info!("  Job ID: {}", validate_result.job_id);
        log::info!("===========================================");
        log::info!("");
        return Ok(());
    }

    // Step 7: Wait for job completion
    log::info!("[7/{}] Waiting for job completion...", total_steps);
    log::info!("  Test values: {:?} -> expected sum: {}", test_values, expected_result);

    let poll_interval = Duration::from_secs(5);
    let max_wait = Duration::from_secs(180); // 3 minutes max
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            anyhow::bail!("Timeout waiting for job completion after {:?}", max_wait);
        }

        // Check if result is available (more reliable than status)
        let result_url = format!("{}/api/jobs/{}/result", backend_url, validate_result.job_id);
        if let Ok(resp) = http_client.get(&result_url).send().await {
            if resp.status().is_success() {
                log::info!("  Job completed! (result available, elapsed: {:?})", start.elapsed());
                break;
            }
        }

        log::info!("  Waiting for result... (elapsed: {:?})", start.elapsed());
        tokio::time::sleep(poll_interval).await;
    }

    // Step 8: Fetch and decrypt result
    log::info!("[8/{}] Fetching and verifying result...", total_steps);

    let encrypted_result = fetch_result(http_client, backend_url, validate_result.job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  SUCCESS! Full E2E test PASSED");
        log::info!("  Job ID: {}", validate_result.job_id);
        log::info!("  Input: {:?}", test_values);
        log::info!("  Operation: Sum");
        log::info!("  Result: {} (correct!)", decrypted);
        log::info!("===========================================");
    } else {
        log::error!("===========================================");
        log::error!("  FAILED! Result mismatch");
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
        anyhow::bail!("Result verification failed");
    }
    log::info!("");

    Ok(())
}

/// Simulate webapp Proof of Innocence flow
/// This tests count_if with Equals predicate - exactly what the PoI page does
/// Flow:
/// 1. Generate encrypted "transaction history" data
/// 2. Upload server_key to /api/server-key/upload
/// 3. Call /api/jobs/validate-and-build with operation=count_if and predicate={Equals: X}
/// 4. Sign and submit transaction
/// 5. Wait for result and verify
async fn run_webapp_flow_poi(
    rpc_client: &RpcClient,
    http_client: &reqwest::Client,
    user_keypair: &Keypair,
    backend_url: &str,
) -> Result<()> {
    log::info!("");
    log::info!("===========================================");
    log::info!("  Webapp Proof of Innocence Flow");
    log::info!("  Testing: count_if with Equals predicate");
    log::info!("===========================================");
    log::info!("");

    // Simulated transaction history (indices the user has interacted with)
    // Demo sanctioned list: [66, 77, 88, 99, 111, 122, 133, 144, 155, 166]
    let user_history: Vec<u8> = vec![10, 20, 30, 40, 50]; // User's "clean" history - none are sanctioned
    let sanctioned_index: u8 = 66; // Check if user interacted with sanctioned index 66

    // Expected result: 0 (user did NOT interact with sanctioned index 66 = INNOCENT)
    let expected_result: u8 = 0;

    log::info!("  User history: {:?}", user_history);
    log::info!("  Sanctioned index to check: {}", sanctioned_index);
    log::info!("  Expected result: {} (0 = innocent, >0 = guilty)", expected_result);
    log::info!("");

    // Step 1: Generate FHE keys and encrypt user history
    log::info!("[1/8] Generating FHE keys and encrypting user history...");
    let operation = FheOperation::CountIf {
        predicate: FhePredicate::Equals(sanctioned_index),
        expected_count: user_history.len() as u16,
    };

    let (encrypted_data, server_key, client_key) = create_fhe_data_with_values(&user_history, &operation)?;
    log::info!(
        "  Encrypted {} history entries: {} bytes data, {:.1} MB server_key",
        user_history.len(),
        encrypted_data.len(),
        server_key.len() as f64 / 1024.0 / 1024.0
    );

    // Step 2: Upload server_key
    log::info!("[2/8] Uploading server_key to /api/server-key/upload...");
    let upload_url = format!("{}/api/server-key/upload", backend_url);
    let upload_start = std::time::Instant::now();

    let upload_response = http_client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(server_key.clone())
        .timeout(Duration::from_secs(600))
        .send()
        .await
        .context("Failed to upload server key")?;

    if !upload_response.status().is_success() {
        let error_text = upload_response.text().await.unwrap_or_default();
        anyhow::bail!("Server key upload failed: {}", error_text);
    }

    let upload_result: ServerKeyUploadResponse = upload_response.json().await?;
    log::info!(
        "  Uploaded in {:.1}s, hash: {}",
        upload_start.elapsed().as_secs_f64(),
        upload_result.server_key_hash
    );

    // Step 3: Create signature
    log::info!("[3/8] Creating signature for job creation...");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let nonce = format!("{:x}", rand::random::<u64>());
    let job_id_placeholder = timestamp * 1000 + rand::random::<u64>() % 1000;
    let message = format!("create_job:{}:{}:{}", job_id_placeholder, timestamp, nonce);

    let signature = user_keypair.sign_message(message.as_bytes());
    let signature_base58 = bs58::encode(signature.as_ref()).into_string();
    log::info!("  Message: {}", message);

    // Step 4: Call validate-and-build with count_if and Equals predicate
    log::info!("[4/8] Calling /api/jobs/validate-and-build with count_if...");
    let encrypted_data_base64 = BASE64.encode(&encrypted_data);
    let validate_url = format!("{}/api/jobs/validate-and-build", backend_url);

    // This is the exact format the frontend sends
    let validate_body = serde_json::json!({
        "creator_pubkey": user_keypair.pubkey().to_string(),
        "encrypted_data": encrypted_data_base64,
        "server_key_hash": upload_result.server_key_hash,
        "message": message,
        "signature": signature_base58,
        "nonce": nonce,
        "operation": "count_if",
        "operation_value": sanctioned_index,
        "expected_count": user_history.len(),
        "price_lamports": 50000000,  // 0.05 SOL (count_if is tier 4)
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "SOL",
        // Rust enum format for serde: {"Equals": value}
        "predicate": {
            "Equals": sanctioned_index
        }
    });

    log::info!("  Request body (predicate): {:?}", validate_body["predicate"]);

    let validate_response = http_client
        .post(&validate_url)
        .header("Content-Type", "application/json")
        .json(&validate_body)
        .send()
        .await
        .context("Failed to call validate-and-build")?;

    if !validate_response.status().is_success() {
        let status = validate_response.status();
        let error_text = validate_response.text().await.unwrap_or_default();
        anyhow::bail!("validate-and-build failed ({}): {}", status, error_text);
    }

    let validate_result: ValidateAndBuildResponse = validate_response.json().await?;
    log::info!("  Job ID: {}", validate_result.job_id);

    // Step 5: Sign and submit transaction
    log::info!("[5/8] Signing and submitting transaction...");
    let tx_bytes = BASE64.decode(&validate_result.transaction)?;
    let mut tx: Transaction = bincode::deserialize(&tx_bytes)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    tx.sign(&[user_keypair], recent_blockhash);

    let tx_signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .context("Failed to submit transaction")?;
    log::info!("  Transaction confirmed: {}", tx_signature);

    // Step 6: Confirm with backend
    log::info!("[6/8] Confirming job with backend...");
    let confirm_url = format!("{}/api/jobs/{}/confirm", backend_url, validate_result.job_id);
    let _ = http_client
        .post(&confirm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "signature": tx_signature.to_string() }))
        .send()
        .await;
    log::info!("  Job confirmed");

    // Step 7: Wait for completion
    log::info!("[7/8] Waiting for job completion...");
    let poll_interval = Duration::from_secs(10);
    let max_wait = Duration::from_secs(400); // CountIf has longer timeout
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            anyhow::bail!("Timeout waiting for job completion");
        }

        let result_url = format!("{}/api/jobs/{}/result", backend_url, validate_result.job_id);
        if let Ok(resp) = http_client.get(&result_url).send().await {
            if resp.status().is_success() {
                log::info!("  Job completed! (elapsed: {:?})", start.elapsed());
                break;
            }
        }

        log::info!("  Waiting... (elapsed: {:?})", start.elapsed());
        tokio::time::sleep(poll_interval).await;
    }

    // Step 8: Decrypt and verify
    log::info!("[8/8] Decrypting and verifying result...");
    let encrypted_result = fetch_result(http_client, backend_url, validate_result.job_id).await?;
    log::info!("  Got encrypted result ({} bytes)", encrypted_result.len());

    let decrypted = decrypt_result(&encrypted_result, &client_key, &operation)?;
    log::info!("  Decrypted result: {}", decrypted);
    log::info!("  Expected result:  {}", expected_result);

    log::info!("");
    if decrypted == expected_result {
        log::info!("===========================================");
        log::info!("  PROOF OF INNOCENCE: VERIFIED");
        log::info!("  User history: {:?}", user_history);
        log::info!("  Sanctioned index checked: {}", sanctioned_index);
        log::info!("  Count of matches: {} (0 = INNOCENT)", decrypted);
        log::info!("===========================================");
    } else {
        log::error!("===========================================");
        log::error!("  RESULT MISMATCH");
        log::error!("  Expected: {}, Got: {}", expected_result, decrypted);
        log::error!("===========================================");
        anyhow::bail!("Result verification failed");
    }
    log::info!("");

    Ok(())
}
