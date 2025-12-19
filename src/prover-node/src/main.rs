#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::too_many_arguments,
    clippy::manual_range_contains,
    clippy::manual_contains,
    clippy::unnecessary_cast,
    clippy::needless_borrows_for_generic_args,
    deprecated
)]

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info, warn};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use zyberlink_sdk::MarketplaceClient;
use zyberlink_types::CircuitType;

// FHE engine from shared crate
use zyberlink_fhe::{deserialize_server_key, FheEngine};

// Internal modules
mod circuits;
mod cli;
mod config;
mod core;
mod engines;
mod futarchy;
mod gateway;
mod halo2_prover;
mod marketplace;
mod roi_calculator;
mod services;
mod tui;
mod witness_encryption;
mod witness_fetcher;
mod wizard;

// Re-exports for convenience
use cli::{ProverArgs, ProverCommand};
use config::ProverConfig;
use marketplace::{MarketplaceFactory, MarketplaceOperations, SolanaMarketplace};
use core::{CircuitRegistry, JobProcessor};
use gateway::GatewayClient;
use halo2_prover::Halo2Prover;
use roi_calculator::ROICalculator;
use witness_encryption::WitnessEncryption;
use witness_fetcher::WitnessFetcher;

// Futarchy FHE imports
use futarchy::{FutarchyPoller, FutarchyPollerConfig, FutarchyPoolWorker};
use fhe_client_sdk::FutarchyFheClient;

/// Helper to create ProverConfig from args
fn config_from_args(args: &ProverArgs) -> Result<ProverConfig> {
    let program_id = args
        .program_id
        .as_ref()
        .context("Program ID is required")?
        .parse()
        .context("Invalid program ID format")?;

    Ok(ProverConfig::new(
        args.rpc_url.clone(),
        program_id,
        args.keypair
            .replace("~", &std::env::var("HOME").unwrap_or_default()),
        Duration::from_secs(args.poll_interval),
        args.min_price,
        args.min_roi,
        args.cost_multiplier,
        Duration::from_secs(args.mock_proving_time),
        args.max_concurrent_jobs,
        args.gateway_url.clone(),
        args.blink_backend_url.clone(),
        args.zk_circuits_path.clone(),
        args.fhe_server_key_path
            .clone()
            .map(|p| p.replace("~", &std::env::var("HOME").unwrap_or_default())),
    ))
}


/// Main prover node that manages job polling and proof generation
struct ProverNode {
    marketplace: Arc<SolanaMarketplace>,
    keypair: Arc<Keypair>,
    config: ProverConfig,
    roi_calculator: Arc<ROICalculator>,
    active_jobs: Arc<tokio::sync::Mutex<Vec<Pubkey>>>,
    job_processor: Arc<JobProcessor>,
    tui_state: Option<Arc<tui::TUIState>>,
    start_time: std::time::Instant,
}

impl ProverNode {
    fn new(config: ProverConfig) -> Result<Self> {
        Self::new_with_tui(config, None)
    }

    fn new_with_tui(config: ProverConfig, tui_state: Option<Arc<tui::TUIState>>) -> Result<Self> {
        let keypair = read_keypair_file(&config.keypair_path)
            .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;
        let keypair_arc = Arc::new(keypair);

        // Create SolanaMarketplace using factory
        let marketplace = MarketplaceFactory::create_solana(
            &config.rpc_url,
            config.program_id,
            keypair_arc.clone(),
        )?;

        // Initialize Halo2 prover
        info!("Initializing Halo2 proving system...");
        let mut halo2_prover = Halo2Prover::new()?;
        halo2_prover.setup()?;
        info!("Halo2 prover ready");

        // Initialize witness encryption (derived from Solana keypair for determinism)
        info!("Initializing witness encryption system...");
        let encryption_seed = derive_encryption_seed(&*keypair_arc);
        let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
        let pubkey = witness_encryption.public_key();
        info!("Witness encryption ready (pubkey: {})", hex::encode(pubkey));

        // Initialize witness fetcher
        info!("Initializing witness fetcher...");
        let witness_fetcher = WitnessFetcher::new(config.gateway_url.clone());
        info!(
            "Witness fetcher ready (backend: {})",
            config.gateway_url
        );

        // Initialize GatewayClient
        info!("Initializing gateway client...");
        let gateway_client = GatewayClient::new(
            config.gateway_url.clone(),
            Arc::new(keypair_arc.insecure_clone()),
        );
        info!(
            "Gateway client ready (gateway: {})",
            config.gateway_url
        );

        // Initialize FHE engine if server key is provided
        let fhe_engine = if let Some(ref key_path) = config.fhe_server_key_path {
            info!("Initializing FHE engine with server key from: {}", key_path);
            let server_key_bytes =
                std::fs::read(key_path).context("Failed to read FHE server key file")?;
            let server_key = deserialize_server_key(&server_key_bytes)
                .context("Failed to deserialize FHE server key")?;
            let engine = FheEngine::new(server_key);
            info!("FHE engine ready");
            Some(Arc::new(engine))
        } else {
            info!("FHE engine not initialized (no server key provided)");
            None
        };

        // Initialize ROI calculator
        let roi_calculator = ROICalculator::new(config.min_roi, config.cost_multiplier);
        info!(
            "ROI calculator initialized - Min ROI: {:.1}%, Cost multiplier: {:.1}x",
            config.min_roi, config.cost_multiplier
        );

        // Create shared references
        let halo2_prover_arc = Arc::new(halo2_prover);
        let witness_encryption_arc = Arc::new(witness_encryption);
        let witness_fetcher_arc = Arc::new(witness_fetcher);
        let gateway_client_arc = Arc::new(gateway_client);

        // Initialize JobProcessor with all dependencies
        let job_processor = Arc::new(JobProcessor::new(
            marketplace.clone(),
            keypair_arc.clone(),
            halo2_prover_arc,
            witness_encryption_arc,
            witness_fetcher_arc,
            gateway_client_arc,
            fhe_engine,
        ));

        Ok(Self {
            marketplace,
            keypair: keypair_arc,
            config,
            roi_calculator: Arc::new(roi_calculator),
            active_jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            job_processor,
            tui_state,
            start_time: std::time::Instant::now(),
        })
    }

    /// Start the prover node main loop
    async fn run(&self) -> Result<()> {
        info!("Starting ZyberLink Prover Node");
        info!("Prover Authority: {}", self.keypair.pubkey());
        info!("Program ID: {}", self.config.program_id);
        info!("RPC URL: {}", self.config.rpc_url);
        info!("Poll Interval: {:?}", self.config.poll_interval);
        info!("Min Price: {} lamports", self.config.min_price);
        info!("Max Concurrent Jobs: {}", self.config.max_concurrent_jobs);

        loop {
            if let Err(e) = self.poll_and_process_jobs().await {
                error!("Error in job processing loop: {:#}", e);
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

        // Find pending ZK jobs
        let pending_jobs = self.marketplace.find_pending_jobs().await
            .context("Failed to find pending jobs")?;

        // Also find FHE jobs that need more provers (for consensus)
        let fhe_jobs = self.marketplace.find_fhe_jobs_needing_provers().await
            .context("Failed to find FHE jobs")?;

        let total_pending = pending_jobs.len() + fhe_jobs.len();
        info!(
            "Found {} pending jobs ({} ZK + {} FHE needing provers)",
            total_pending,
            pending_jobs.len(),
            fhe_jobs.len()
        );

        // Filter jobs using ROI calculator - only accept profitable jobs
        let mut suitable_jobs = Vec::new();
        let mut rejected_count = 0;

        // Process regular pending jobs (mostly ZK jobs)
        for job in pending_jobs {
            // Skip FHE jobs here - we handle them separately below
            if CircuitRegistry::is_fhe_circuit_from_enum(&job.circuit_type) {
                continue;
            }

            let circuit_type = CircuitRegistry::get_circuit_type_from_enum(&job.circuit_type);
            let required_provers = 1u8; // ZK jobs use single prover

            // Evaluate job profitability
            let roi = self.roi_calculator.evaluate_job(
                &circuit_type,
                job.price,
                required_provers,
            );

            if roi.is_profitable {
                suitable_jobs.push((job, circuit_type, roi));
            } else {
                rejected_count += 1;
                debug!(
                    "Rejected job {} - Tier {}, ROI {:.1}%, Profit: {} lamports",
                    job.id, roi.complexity_tier, roi.roi_percentage, roi.profit
                );
            }
        }

        // Process FHE jobs that need more provers
        for job in fhe_jobs {
            let circuit_type = job.circuit_type.clone();

            // Get FHE consensus config to determine required provers
            let fhe_config = self.marketplace.get_fhe_consensus_config(job.id).await
                .context("Failed to get FHE consensus config")?;

            let required_provers = fhe_config.as_ref()
                .map(|c| c.required_provers)
                .unwrap_or(1);

            // Evaluate job profitability
            let roi = self.roi_calculator.evaluate_job(
                &circuit_type,
                job.price,
                required_provers,
            );

            if roi.is_profitable {
                if let Some(config) = &fhe_config {
                    info!(
                        "FHE job {} needs provers: required {}",
                        job.id, config.required_provers
                    );
                }
                suitable_jobs.push((job, circuit_type, roi));
            } else {
                rejected_count += 1;
                debug!(
                    "Rejected FHE job {} - Tier {}, ROI {:.1}%, Profit: {} lamports",
                    job.id, roi.complexity_tier, roi.roi_percentage, roi.profit
                );
            }
        }

        if rejected_count > 0 {
            info!(
                "Rejected {} unprofitable jobs (ROI < {:.1}%)",
                rejected_count, self.config.min_roi
            );
        }

        // Update TUI stats with pending jobs count
        if let Some(ref tui) = self.tui_state {
            tui.update_stats(|stats| {
                stats.jobs_pending = suitable_jobs.len() as u32;
                stats.jobs_claimed = active_count as u32;
            });
        }

        if suitable_jobs.is_empty() {
            debug!("No profitable jobs available");
            return Ok(());
        }

        info!(
            "Found {} profitable jobs (ROI >= {:.1}%)",
            suitable_jobs.len(),
            self.config.min_roi
        );

        // Process jobs up to max concurrent limit
        let slots_available = self.config.max_concurrent_jobs - active_count;
        for (job, circuit_type, roi) in suitable_jobs.into_iter().take(slots_available) {
            info!(
                "Processing job {} - Price: {} lamports, Circuit: {:?}, ROI: {:.1}%, Profit: {} lamports",
                job.id, job.price, circuit_type, roi.roi_percentage, roi.profit
            );

            // Parse job_pda from address string
            let job_pda: Pubkey = job.address.parse()
                .context("Failed to parse job address")?;

            // Spawn job processing task
            let active_jobs = self.active_jobs.clone();
            let job_processor = self.job_processor.clone();
            let tui_state = self.tui_state.clone();
            let job_price = job.price;
            let job_id = job.id;
            let witness_hash = job.witness_hash;
            let creator = job.creator.clone();

            tokio::spawn(async move {
                if let Err(e) = job_processor
                    .process_job(job_pda, job_id, creator, circuit_type, witness_hash, job_price, tui_state.clone())
                    .await
                {
                    error!("Failed to process job {}: {}", job_id, e);

                    // Update failed job stats
                    if let Some(ref tui) = tui_state {
                        tui.update_stats(|stats| {
                            stats.jobs_failed += 1;
                        });
                    }
                }

                // Remove from active jobs
                active_jobs.lock().await.retain(|&pda| pda != job_pda);
            });

            // Add to active jobs
            self.active_jobs.lock().await.push(job_pda);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let args = ProverArgs::parse();

    match args.command.as_ref().unwrap_or(&ProverCommand::Run) {
        ProverCommand::Run => {
            let config = config_from_args(&args)?;

            // Start Futarchy poller in background thread if enabled
            let futarchy_handle = if args.enable_futarchy {
                Some(start_futarchy_poller(&args)?)
            } else {
                None
            };

            if args.tui_mode {
                // Run with TUI
                run_with_tui(config).await?;
            } else {
                // Run headless
                let prover = ProverNode::new(config)?;
                prover.run().await?;
            }

            // Futarchy poller runs in its own thread and will be cleaned up on exit
            drop(futarchy_handle);
        }
        ProverCommand::Register { stake_amount } => {
            register_prover(&args, *stake_amount).await?;
        }
        ProverCommand::ShowPubkey => {
            show_pubkey(&args)?;
        }
        ProverCommand::Setup { stake_amount } => {
            run_setup_wizard(*stake_amount).await?;
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
    let prover_tui_state = Arc::clone(&tui_state);
    let prover_handle = tokio::spawn(async move {
        let prover = ProverNode::new_with_tui(config, Some(prover_tui_state))?;
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
async fn register_prover(args: &ProverArgs, stake_amount: u64) -> Result<()> {
    let program_id = args
        .program_id
        .as_ref()
        .context("Program ID is required (--program-id)")?
        .parse()
        .context("Invalid program ID format")?;

    let keypair_path = args
        .keypair
        .replace("~", &std::env::var("HOME").unwrap_or_default());
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
    info!(
        "  Stake: {} lamports ({} SOL)",
        stake_amount,
        stake_amount as f64 / 1_000_000_000.0
    );
    info!("  Encryption pubkey: {}", hex::encode(encryption_pubkey));

    // Create register instruction
    let ix =
        client.register_prover_instruction(&keypair.pubkey(), stake_amount, encryption_pubkey)?;

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
    let mut seed_material = b"ZYBERLINK_WITNESS_ENCRYPTION_V1:".to_vec();
    seed_material.extend_from_slice(&keypair.to_bytes());

    let hash_result = hash(&seed_material);
    hash_result.to_bytes()
}

/// Show the encryption public key for this prover
fn show_pubkey(args: &ProverArgs) -> Result<()> {
    let keypair_path = args
        .keypair
        .replace("~", &std::env::var("HOME").unwrap_or_default());
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    // Derive encryption seed from Solana keypair (deterministic)
    let encryption_seed = derive_encryption_seed(&keypair);
    let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
    let encryption_pubkey = witness_encryption.public_key();

    println!("Prover Encryption Public Key");
    println!("=============================");
    println!("Authority:       {}", keypair.pubkey());
    println!("Encryption Key:  {}", hex::encode(encryption_pubkey));
    println!();
    println!("Clients should use this key to encrypt witness data before uploading.");

    Ok(())
}

/// Run the interactive setup wizard
async fn run_setup_wizard(stake_amount: u64) -> Result<()> {
    info!("Starting setup wizard...");

    let mut wizard = wizard::SetupWizard::new(stake_amount);
    let _config = wizard.run().await?;

    info!("Setup wizard completed successfully!");
    info!(
        "Configuration saved to: {:?}",
        config::ProverConfiguration::default_path()?
    );

    Ok(())
}

/// Start the Futarchy FHE job poller in a background thread
///
/// Returns a JoinHandle that can be dropped to stop the poller
fn start_futarchy_poller(args: &ProverArgs) -> Result<std::thread::JoinHandle<()>> {
    info!("Starting Futarchy FHE poller...");
    info!("  Server URL: {}", args.futarchy_server_url);
    info!("  Poll Interval: {} seconds", args.poll_interval);

    // Create FHE client (this takes a few seconds for key generation)
    info!("Initializing Futarchy FHE client (this may take 10-30 seconds)...");
    let fhe_client = FutarchyFheClient::new()
        .map_err(|e| anyhow::anyhow!("Failed to create Futarchy FHE client: {}", e))?;
    info!("Futarchy FHE client initialized");

    // Create worker with the FHE client
    let worker = Arc::new(FutarchyPoolWorker::with_client(
        fhe_client,
        &args.futarchy_server_url,
    ));

    // Configure poller
    let config = FutarchyPollerConfig {
        solana_rpc_url: args.rpc_url.clone(),
        // TODO: Make these configurable via CLI args
        futarchy_program_id: Pubkey::default(), // Placeholder
        fhe_program_id: Pubkey::default(),      // Placeholder
        poll_interval: Duration::from_secs(args.poll_interval),
        max_jobs_per_poll: 5,
    };

    // Create poller
    let poller = FutarchyPoller::new(config, worker)?;

    // Start in background thread
    let handle = std::thread::Builder::new()
        .name("futarchy-poller".to_string())
        .spawn(move || {
            poller.run_loop();
        })
        .context("Failed to spawn Futarchy poller thread")?;

    info!("Futarchy FHE poller started in background thread");

    Ok(handle)
}
