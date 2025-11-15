use anyhow::{Context, Result};
use clap::Parser;
use cypherlink_sdk::{fetch_job, find_pending_jobs, MarketplaceClient};
use cypherlink_types::{CircuitType, JobStatus};
use log::{debug, error, info, warn};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair, Signer},
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

mod fhe_engine;
mod halo2_prover;
mod witness_encryption;
mod witness_fetcher;
mod tui;

use fhe_engine::FheEngine;
use halo2_prover::{Halo2Prover, OrchardWitness};
use witness_encryption::WitnessEncryption;
use witness_fetcher::WitnessFetcher;

/// CypherLink Prover Node - Autonomous ZK proof generation daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Solana RPC URL
    #[arg(short, long, default_value = "http://localhost:8899", global = true)]
    rpc_url: String,

    /// CypherLink program ID
    #[arg(short, long, global = true)]
    program_id: Option<String>,

    /// Path to prover keypair file
    #[arg(short, long, default_value = "~/.config/solana/id.json", global = true)]
    keypair: String,

    /// Polling interval in seconds
    #[arg(long, default_value = "5")]
    poll_interval: u64,

    /// Minimum job price in lamports to accept
    #[arg(long, default_value = "1000000")]
    min_price: u64,

    /// Mock proving time in seconds (simulates proof generation)
    #[arg(long, default_value = "10")]
    mock_proving_time: u64,

    /// Maximum concurrent jobs
    #[arg(long, default_value = "3")]
    max_concurrent_jobs: usize,

    /// Witness storage backend URL
    #[arg(long, default_value = "http://localhost:3030")]
    witness_backend_url: String,

    /// FHE server key file path (required for FHE jobs)
    #[arg(long)]
    fhe_server_key_path: Option<String>,

    /// Enable TUI (Terminal User Interface) mode
    #[arg(long)]
    tui_mode: bool,
}

#[derive(Parser, Debug)]
enum Command {
    /// Run the prover daemon (default)
    Run,

    /// Register as a prover on-chain
    Register {
        /// Stake amount in lamports
        #[arg(long, default_value = "10000000000")]
        stake_amount: u64,
    },

    /// Show encryption public key
    ShowPubkey,
}

/// Prover node configuration
#[derive(Debug, Clone)]
struct ProverConfig {
    rpc_url: String,
    program_id: solana_sdk::pubkey::Pubkey,
    keypair_path: String,
    poll_interval: Duration,
    min_price: u64,
    mock_proving_time: Duration,
    max_concurrent_jobs: usize,
    witness_backend_url: String,
    fhe_server_key_path: Option<String>,
}

impl ProverConfig {
    fn from_args(args: &Args) -> Result<Self> {
        let program_id = args
            .program_id
            .as_ref()
            .context("Program ID is required")?
            .parse()
            .context("Invalid program ID format")?;

        Ok(Self {
            rpc_url: args.rpc_url.clone(),
            program_id,
            keypair_path: args.keypair.replace("~", &std::env::var("HOME").unwrap_or_default()),
            poll_interval: Duration::from_secs(args.poll_interval),
            min_price: args.min_price,
            mock_proving_time: Duration::from_secs(args.mock_proving_time),
            max_concurrent_jobs: args.max_concurrent_jobs,
            witness_backend_url: args.witness_backend_url.clone(),
            fhe_server_key_path: args.fhe_server_key_path.clone()
                .map(|p| p.replace("~", &std::env::var("HOME").unwrap_or_default())),
        })
    }
}

/// Main prover node that manages job polling and proof generation
struct ProverNode {
    client: Arc<MarketplaceClient>,
    keypair: Arc<Keypair>,
    config: ProverConfig,
    active_jobs: Arc<tokio::sync::Mutex<Vec<solana_sdk::pubkey::Pubkey>>>,
    halo2_prover: Arc<Halo2Prover>,
    witness_encryption: Arc<WitnessEncryption>,
    witness_fetcher: Arc<WitnessFetcher>,
    fhe_engine: Option<Arc<FheEngine>>,
}

impl ProverNode {
    fn new(config: ProverConfig) -> Result<Self> {
        let keypair = read_keypair_file(&config.keypair_path)
            .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

        let client = MarketplaceClient::new_with_commitment(
            config.rpc_url.clone(),
            config.program_id,
            CommitmentConfig::confirmed(),
        );

        // Initialize Halo2 prover
        info!("Initializing Halo2 proving system...");
        let mut halo2_prover = Halo2Prover::new()?;
        halo2_prover.setup()?;
        info!("Halo2 prover ready");

        // Initialize witness encryption (derived from Solana keypair for determinism)
        info!("Initializing witness encryption system...");
        let encryption_seed = derive_encryption_seed(&keypair);
        let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
        let pubkey = witness_encryption.public_key();
        info!("Witness encryption ready (pubkey: {})", hex::encode(&pubkey));

        // Initialize witness fetcher
        info!("Initializing witness fetcher...");
        let witness_fetcher = WitnessFetcher::new(config.witness_backend_url.clone());
        info!("Witness fetcher ready (backend: {})", config.witness_backend_url);

        // Initialize FHE engine if server key is provided
        let fhe_engine = if let Some(ref key_path) = config.fhe_server_key_path {
            info!("Initializing FHE engine with server key from: {}", key_path);
            let server_key_bytes = std::fs::read(key_path)
                .context("Failed to read FHE server key file")?;
            let server_key = fhe_engine::deserialize_server_key(&server_key_bytes)
                .context("Failed to deserialize FHE server key")?;
            let engine = FheEngine::new(server_key);
            info!("FHE engine ready");
            Some(Arc::new(engine))
        } else {
            info!("FHE engine not initialized (no server key provided)");
            None
        };

        Ok(Self {
            client: Arc::new(client),
            keypair: Arc::new(keypair),
            config,
            active_jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            halo2_prover: Arc::new(halo2_prover),
            witness_encryption: Arc::new(witness_encryption),
            witness_fetcher: Arc::new(witness_fetcher),
            fhe_engine,
        })
    }

    /// Start the prover node main loop
    async fn run(&self) -> Result<()> {
        info!("Starting CypherLink Prover Node");
        info!("Prover Authority: {}", self.keypair.pubkey());
        info!("Program ID: {}", self.config.program_id);
        info!("RPC URL: {}", self.config.rpc_url);
        info!("Poll Interval: {:?}", self.config.poll_interval);
        info!("Min Price: {} lamports", self.config.min_price);
        info!("Max Concurrent Jobs: {}", self.config.max_concurrent_jobs);

        loop {
            if let Err(e) = self.poll_and_process_jobs().await {
                error!("Error in job processing loop: {}", e);
            }

            sleep(self.config.poll_interval).await;
        }
    }

    /// Poll for available jobs and process them
    async fn poll_and_process_jobs(&self) -> Result<()> {
        debug!("Polling for available jobs...");

        // Get current active job count
        let active_count = self.active_jobs.lock().await.len();
        if active_count >= self.config.max_concurrent_jobs {
            debug!(
                "Max concurrent jobs reached ({}/{}), skipping poll",
                active_count, self.config.max_concurrent_jobs
            );
            return Ok(());
        }

        // Find pending jobs
        let pending_jobs = find_pending_jobs(&self.client.rpc_client, &self.config.program_id)
            .context("Failed to query pending jobs")?;

        info!("Found {} pending jobs", pending_jobs.len());

        // Filter jobs by minimum price
        let suitable_jobs: Vec<_> = pending_jobs
            .into_iter()
            .filter(|(_, job)| job.price_lamports >= self.config.min_price)
            .collect();

        if suitable_jobs.is_empty() {
            debug!("No suitable jobs available");
            return Ok(());
        }

        info!(
            "Found {} suitable jobs (>= {} lamports)",
            suitable_jobs.len(),
            self.config.min_price
        );

        // Process jobs up to max concurrent limit
        let slots_available = self.config.max_concurrent_jobs - active_count;
        for (job_pda, job) in suitable_jobs.into_iter().take(slots_available) {
            info!(
                "Processing job {} (price: {} lamports, circuit: {:?})",
                job.id, job.price_lamports, job.circuit_type
            );

            // Spawn job processing task
            let client = self.client.clone();
            let keypair = self.keypair.clone();
            let active_jobs = self.active_jobs.clone();
            let mock_proving_time = self.config.mock_proving_time;
            let halo2_prover = self.halo2_prover.clone();
            let witness_encryption = self.witness_encryption.clone();

            let witness_fetcher = self.witness_fetcher.clone();
            let fhe_engine = self.fhe_engine.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::process_job(
                        client,
                        keypair,
                        job_pda,
                        job.id,
                        job.circuit_type,
                        job.witness_commitment,
                        mock_proving_time,
                        halo2_prover,
                        witness_encryption,
                        witness_fetcher,
                        fhe_engine,
                    )
                    .await
                {
                    error!("Failed to process job {}: {}", job.id, e);
                }

                // Remove from active jobs
                active_jobs.lock().await.retain(|&pda| pda != job_pda);
            });

            // Add to active jobs
            self.active_jobs.lock().await.push(job_pda);
        }

        Ok(())
    }

    /// Process a single job: claim -> prove -> submit
    async fn process_job(
        client: Arc<MarketplaceClient>,
        keypair: Arc<Keypair>,
        job_pda: solana_sdk::pubkey::Pubkey,
        job_id: u64,
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        _mock_proving_time: Duration,
        halo2_prover: Arc<Halo2Prover>,
        witness_encryption: Arc<WitnessEncryption>,
        witness_fetcher: Arc<WitnessFetcher>,
        fhe_engine: Option<Arc<FheEngine>>,
    ) -> Result<()> {
        info!("[Job {}] Starting processing", job_id);

        // Step 1: Claim the job
        info!("[Job {}] Claiming job...", job_id);
        let claim_ix = client
            .claim_job_instruction(&keypair.pubkey(), &job_pda)
            .context("Failed to build claim instruction")?;

        match client.send_and_confirm_transaction(&[claim_ix], &[&*keypair]) {
            Ok(sig) => {
                info!("[Job {}] Claimed successfully (sig: {})", job_id, sig);
            }
            Err(e) => {
                warn!("[Job {}] Failed to claim (already claimed?): {}", job_id, e);
                return Err(e.into());
            }
        }

        // Verify claim succeeded
        let job = fetch_job(&client.rpc_client, &job_pda)
            .context("Failed to fetch job after claim")?;

        // For ZK jobs: verify we claimed it exclusively
        if let CircuitType::ZcashOrchard = circuit_type {
            if job.status != JobStatus::Claimed || job.prover != Some(keypair.pubkey()) {
                warn!("[Job {}] ZK job not claimed by us, aborting", job_id);
                return Ok(());
            }
        }

        // For FHE jobs: verify we're in the claimed_provers list
        if let CircuitType::FheComputation(_) = circuit_type {
            if job.status != JobStatus::Claimed {
                warn!("[Job {}] FHE job not in Claimed status, aborting", job_id);
                return Ok(());
            }

            if !job.claimed_provers.contains(&keypair.pubkey()) {
                warn!("[Job {}] We are not in the claimed_provers list, aborting", job_id);
                return Ok(());
            }

            // Check if we already submitted a result
            if job.fhe_results.iter().any(|r| r.prover == keypair.pubkey()) {
                info!("[Job {}] Already submitted FHE result, skipping", job_id);
                return Ok(());
            }
        }

        // Step 2: Download and decrypt witness
        info!("[Job {}] Downloading encrypted witness from backend...", job_id);

        let encrypted_witness = witness_fetcher
            .download_witness(&witness_commitment)
            .await
            .context("Failed to download witness from backend")?;

        info!(
            "[Job {}] Downloaded encrypted witness ({} bytes)",
            job_id,
            encrypted_witness.len()
        );

        // Decrypt witness
        info!("[Job {}] Decrypting witness data...", job_id);

        let witness = witness_encryption
            .decrypt_witness(&encrypted_witness)
            .context("Failed to decrypt witness data")?;

        info!("[Job {}] Witness decrypted successfully", job_id);

        // Step 3: Generate proof based on circuit type
        info!(
            "[Job {}] Generating proof (circuit: {:?})...",
            job_id, circuit_type
        );

        let proof_bytes = match circuit_type {
            CircuitType::ZcashOrchard => {
                // Validate witness
                witness
                    .validate()
                    .context("Invalid witness data")?;

                // Generate real Halo2 proof
                let proof = Self::real_generate_proof(halo2_prover.clone(), witness).await?;
                info!(
                    "[Job {}] Halo2 proof generated successfully ({} bytes)",
                    job_id,
                    proof.len()
                );
                proof
            }
            CircuitType::FheComputation(ref operation) => {
                // Handle FHE computation
                let engine = fhe_engine
                    .as_ref()
                    .context("FHE engine not initialized - server key required for FHE jobs")?;

                info!("[Job {}] Executing FHE operation: {:?}", job_id, operation);

                // For FHE jobs, the encrypted witness should be used directly as input
                // The witness for FHE is the serialized encrypted FheUint8 bytes
                // For now, we'll use the encrypted witness bytes directly
                let encrypted_input_bytes = &encrypted_witness;

                // Perform FHE computation
                let result_bytes = Self::execute_fhe_computation(
                    engine.clone(),
                    encrypted_input_bytes,
                    operation,
                )
                .await?;

                // Hash result for consensus
                let result_hash = FheEngine::hash_result(&result_bytes);

                info!(
                    "[Job {}] FHE computation complete ({} bytes, hash: {})",
                    job_id,
                    result_bytes.len(),
                    hex::encode(&result_hash[..8])
                );

                // For FHE, the "proof" is the encrypted result
                result_bytes
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported circuit type: {:?}",
                    circuit_type
                ));
            }
        };

        info!(
            "[Job {}] Result generated successfully ({} bytes)",
            job_id,
            proof_bytes.len()
        );

        // Step 4: Submit result based on job type
        match circuit_type {
            CircuitType::ZcashOrchard => {
                info!("[Job {}] Submitting ZK proof...", job_id);

                // Generate proof commitment (hash of actual proof)
                let proof_commitment = Self::generate_proof_commitment(&proof_bytes);
                let proof_size = proof_bytes.len() as u32;

                // Fetch config to get protocol fee recipient
                let (config_pda, _) = client.get_config_pda();
                let config_account = client.rpc_client.get_account(&config_pda)
                    .context("Failed to fetch config account")?;

                // Extract protocol_fee_recipient from config
                let protocol_fee_recipient = if config_account.data.len() >= 86 {
                    solana_sdk::pubkey::Pubkey::try_from(&config_account.data[54..86])?
                } else {
                    return Err(anyhow::anyhow!("Invalid config account"));
                };

                let submit_ix = client
                    .submit_proof_instruction_with_recipient(
                        &keypair.pubkey(),
                        &job_pda,
                        &job.creator,
                        &protocol_fee_recipient,
                        proof_commitment,
                        proof_size,
                    )
                    .context("Failed to build submit proof instruction")?;

                match client.send_and_confirm_transaction(&[submit_ix], &[&*keypair]) {
                    Ok(sig) => {
                        info!("[Job {}] ZK proof submitted successfully (sig: {})", job_id, sig);
                    }
                    Err(e) => {
                        error!("[Job {}] Failed to submit ZK proof: {}", job_id, e);
                        return Err(e.into());
                    }
                }
            }

            CircuitType::FheComputation(_) => {
                info!("[Job {}] Submitting FHE result...", job_id);

                // Hash result for consensus
                let result_hash = FheEngine::hash_result(&proof_bytes);

                info!(
                    "[Job {}] FHE result hash: {}",
                    job_id,
                    hex::encode(&result_hash[..8])
                );

                // Store encrypted result in witness backend
                info!("[Job {}] Uploading encrypted result to witness backend...", job_id);

                let witness_backend_url = std::env::var("WITNESS_BACKEND_URL")
                    .unwrap_or_else(|_| "http://localhost:3030".to_string());

                let upload_url = format!("{}/fhe-result", witness_backend_url);

                let response = reqwest::blocking::Client::new()
                    .post(&upload_url)
                    .body(proof_bytes.clone())
                    .send()
                    .context("Failed to upload FHE result to witness backend")?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to upload FHE result: HTTP {}",
                        response.status()
                    ));
                }

                let upload_response: serde_json::Value = response.json()
                    .context("Failed to parse upload response")?;

                let stored_commitment = upload_response["commitment"]
                    .as_str()
                    .context("Missing commitment in response")?;

                info!(
                    "[Job {}] FHE result stored with commitment: {}",
                    job_id, stored_commitment
                );

                // Build SubmitFheResult instruction
                let submit_ix = client
                    .submit_fhe_result_instruction(&keypair.pubkey(), &job_pda, result_hash)
                    .context("Failed to build submit FHE result instruction")?;

                match client.send_and_confirm_transaction(&[submit_ix], &[&*keypair]) {
                    Ok(sig) => {
                        info!("[Job {}] FHE result submitted successfully (sig: {})", job_id, sig);
                    }
                    Err(e) => {
                        error!("[Job {}] Failed to submit FHE result: {}", job_id, e);
                        return Err(e.into());
                    }
                }
            }

            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported circuit type: {:?}",
                    circuit_type
                ));
            }
        }

        info!("[Job {}] Completed!", job_id);

        Ok(())
    }

    /// Generate real Halo2 proof (CPU-intensive, runs in blocking thread)
    async fn real_generate_proof(
        prover: Arc<Halo2Prover>,
        witness: OrchardWitness,
    ) -> Result<Vec<u8>> {
        // Run in blocking thread since Halo2 is CPU-intensive
        tokio::task::spawn_blocking(move || prover.generate_orchard_proof(witness))
            .await
            .context("Proof generation task panicked")?
    }

    /// Execute FHE computation (runs in blocking thread since it's CPU-intensive)
    async fn execute_fhe_computation(
        engine: Arc<FheEngine>,
        encrypted_input: &[u8],
        operation: &cypherlink_types::FheOperation,
    ) -> Result<Vec<u8>> {
        use cypherlink_types::FheOperation;

        // encrypted_input contains serialized FheUint8 ciphertext
        let input_bytes = encrypted_input.to_vec();
        let operation = operation.clone();

        // Run in blocking thread since FHE computation is CPU-intensive
        tokio::task::spawn_blocking(move || {
            match operation {
                FheOperation::Add(constant) => engine.compute_add(&input_bytes, constant),
                FheOperation::Multiply(constant) => engine.compute_multiply(&input_bytes, constant),
            }
        })
        .await
        .context("FHE computation task panicked")?
    }

    /// Generate proof commitment (hash of actual proof)
    fn generate_proof_commitment(proof: &[u8]) -> [u8; 32] {
        use solana_sdk::hash::hash;
        let hash_result = hash(proof);
        hash_result.to_bytes()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let args = Args::parse();

    match args.command.as_ref().unwrap_or(&Command::Run) {
        Command::Run => {
            let config = ProverConfig::from_args(&args)?;

            if args.tui_mode {
                // Run with TUI
                run_with_tui(config).await?;
            } else {
                // Run headless
                let prover = ProverNode::new(config)?;
                prover.run().await?;
            }
        }
        Command::Register { stake_amount } => {
            register_prover(&args, *stake_amount).await?;
        }
        Command::ShowPubkey => {
            show_pubkey(&args)?;
        }
    }

    Ok(())
}

/// Run prover node with TUI interface
async fn run_with_tui(config: ProverConfig) -> Result<()> {
    use std::sync::Arc;

    // Create shared TUI state
    let tui_state = Arc::new(tui::TUIState::new());

    // Setup terminal
    let mut terminal = tui::setup_terminal()?;

    // Create TUI app
    let mut tui_app = tui::TUIApp::new(Arc::clone(&tui_state));

    // Start prover node in background task
    let prover_state = Arc::clone(&tui_state);
    let prover_handle = tokio::spawn(async move {
        let prover = ProverNode::new(config)?;

        // TODO: Pass tui_state to prover to update stats
        // For now, just run the prover
        prover.run().await
    });

    // Run TUI in main thread (needs to be on main thread for terminal control)
    let tui_result = tui_app.run(&mut terminal);

    // Cleanup terminal
    tui::restore_terminal(&mut terminal)?;

    // Signal prover to quit
    tui_state.set_quit();

    // Wait for prover to finish
    match tokio::time::timeout(Duration::from_secs(5), prover_handle).await {
        Ok(Ok(Ok(_))) => info!("Prover shut down cleanly"),
        Ok(Ok(Err(e))) => error!("Prover error: {}", e),
        Ok(Err(e)) => error!("Prover task panicked: {}", e),
        Err(_) => warn!("Prover shutdown timeout"),
    }

    tui_result
}

/// Register this prover on-chain
async fn register_prover(args: &Args, stake_amount: u64) -> Result<()> {
    let program_id = args
        .program_id
        .as_ref()
        .context("Program ID is required (--program-id)")?
        .parse()
        .context("Invalid program ID format")?;

    let keypair_path = args.keypair.replace("~", &std::env::var("HOME").unwrap_or_default());
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    // Initialize witness encryption to get pubkey
    let witness_encryption = WitnessEncryption::new()?;
    let encryption_pubkey = witness_encryption.public_key();

    // Create client
    let client = MarketplaceClient::new_with_commitment(
        args.rpc_url.clone(),
        program_id,
        CommitmentConfig::confirmed(),
    );

    info!("Registering prover...");
    info!("  Authority: {}", keypair.pubkey());
    info!("  Stake: {} lamports ({} SOL)", stake_amount, stake_amount as f64 / 1_000_000_000.0);
    info!("  Encryption pubkey: {}", hex::encode(&encryption_pubkey));

    // Create register instruction
    let ix = client.register_prover_instruction(
        &keypair.pubkey(),
        stake_amount,
        encryption_pubkey,
    )?;

    // Send transaction
    let sig = client.send_and_confirm_transaction(&[ix], &[&keypair])?;

    info!("Prover registered successfully!");
    info!("  Signature: {}", sig);

    // Verify registration
    let (prover_pda, _) = client.get_prover_pda(&keypair.pubkey());
    info!("  Prover PDA: {}", prover_pda);

    Ok(())
}

/// Derive a deterministic encryption seed from the Solana keypair
/// This ensures the same keypair always generates the same encryption key
fn derive_encryption_seed(keypair: &Keypair) -> [u8; 32] {
    use solana_sdk::hash::hash;

    // Hash the secret key bytes to derive encryption seed
    // This provides domain separation from the signing key
    let mut seed_material = b"CYPHERLINK_WITNESS_ENCRYPTION_V1:".to_vec();
    seed_material.extend_from_slice(&keypair.to_bytes());

    let hash_result = hash(&seed_material);
    hash_result.to_bytes()
}

/// Show the encryption public key for this prover
fn show_pubkey(args: &Args) -> Result<()> {
    let keypair_path = args.keypair.replace("~", &std::env::var("HOME").unwrap_or_default());
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    // Derive encryption seed from Solana keypair (deterministic)
    let encryption_seed = derive_encryption_seed(&keypair);
    let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
    let encryption_pubkey = witness_encryption.public_key();

    println!("Prover Encryption Public Key");
    println!("=============================");
    println!("Authority:       {}", keypair.pubkey());
    println!("Encryption Key:  {}", hex::encode(&encryption_pubkey));
    println!();
    println!("Clients should use this key to encrypt witness data before uploading.");

    Ok(())
}
