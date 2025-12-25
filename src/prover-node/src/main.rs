#![allow(clippy::too_many_arguments, clippy::needless_borrows_for_generic_args)]

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
use config::{ProverConfig, ChainConfig};
use marketplace::{MarketplaceFactory, MarketplaceOperations, SolanaMarketplace};
use core::{CircuitRegistry, JobProcessor};
use gateway::GatewayClient;
use halo2_prover::Halo2Prover;
use roi_calculator::ROICalculator;
use services::{StarknetEventListener, StarknetEventListenerConfig};
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

    // Parse optional generator program IDs
    let zk_generator_program = args.zk_generator_program
        .as_ref()
        .map(|s| s.parse())
        .transpose()
        .context("Invalid ZK Generator program ID format")?;

    let fhe_generator_program = args.fhe_generator_program
        .as_ref()
        .map(|s| s.parse())
        .transpose()
        .context("Invalid FHE Generator program ID format")?;

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
        zk_generator_program,
        fhe_generator_program,
    ))
}


/// Main prover node that manages job polling and proof generation
struct ProverNode {
    /// Multi-chain marketplaces (chain_name -> marketplace)
    marketplaces: std::collections::HashMap<String, Arc<dyn MarketplaceOperations>>,
    keypair: Arc<Keypair>,
    config: ProverConfig,
    roi_calculator: Arc<ROICalculator>,
    active_jobs: Arc<tokio::sync::Mutex<Vec<String>>>, // Changed to String for multi-chain job IDs
    job_processor: Arc<JobProcessor>,
    tui_state: Option<Arc<tui::TUIState>>,
    start_time: std::time::Instant,
    starknet_event_configs: Vec<StarknetEventListenerConfig>,
}

impl ProverNode {
    fn new(config: ProverConfig) -> Result<Self> {
        Self::new_with_tui(config, None)
    }

    fn new_with_tui(config: ProverConfig, tui_state: Option<Arc<tui::TUIState>>) -> Result<Self> {
        let keypair = read_keypair_file(&config.keypair_path)
            .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;
        let keypair_arc = Arc::new(keypair);

        // Create a single Solana marketplace for legacy mode (single-chain)
        // This maintains backward compatibility when not using multi-chain config
        let mut marketplaces = std::collections::HashMap::new();

        let marketplace = if config.zk_generator_program.is_some() || config.fhe_generator_program.is_some() {
            info!(
                "Creating marketplace with generators: ZK={:?}, FHE={:?}",
                config.zk_generator_program, config.fhe_generator_program
            );
            Arc::new(SolanaMarketplace::new_with_generators(
                &config.rpc_url,
                config.program_id,
                keypair_arc.clone(),
                config.zk_generator_program,
                config.fhe_generator_program,
            ).map_err(|e| anyhow::anyhow!("Failed to create SolanaMarketplace: {}", e))?)
        } else {
            MarketplaceFactory::create_solana(
                &config.rpc_url,
                config.program_id,
                keypair_arc.clone(),
            )?
        };

        // Store in marketplaces map for compatibility with new multi-chain structure
        marketplaces.insert("solana".to_string(), marketplace.clone() as Arc<dyn MarketplaceOperations>);

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

        // Initialize ROI calculator (legacy mode uses default Profit mode)
        let roi_calculator = ROICalculator::new(
            config.min_roi,
            config.cost_multiplier,
            roi_calculator::OperationMode::default(),
        );
        info!(
            "ROI calculator initialized - Mode: profit, Min ROI: {:.1}%, Cost multiplier: {:.1}x",
            config.min_roi, config.cost_multiplier
        );

        // Create shared references
        let halo2_prover_arc = Arc::new(halo2_prover);
        let witness_encryption_arc = Arc::new(witness_encryption);
        let witness_fetcher_arc = Arc::new(witness_fetcher);
        let gateway_client_arc = Arc::new(gateway_client);

        // Initialize JobProcessor with all dependencies
        // Use the first marketplace for JobProcessor (it will be passed per-job in multi-chain mode)
        let default_marketplace = marketplace.clone();
        let job_processor = Arc::new(JobProcessor::new(
            default_marketplace.clone() as Arc<dyn MarketplaceOperations>,
            keypair_arc.clone(),
            halo2_prover_arc,
            witness_encryption_arc,
            witness_fetcher_arc,
            gateway_client_arc,
            fhe_engine,
        ));

        Ok(Self {
            marketplaces,
            keypair: keypair_arc,
            config,
            roi_calculator: Arc::new(roi_calculator),
            active_jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            job_processor,
            tui_state,
            start_time: std::time::Instant::now(),
            starknet_event_configs: Vec::new(),
        })
    }

    /// Create a ProverNode from multi-chain TOML configuration
    fn new_from_config_file(
        config_path: &str,
        tui_state: Option<Arc<tui::TUIState>>,
    ) -> Result<Self> {
        use config::{ProverConfigFile, ChainConfig};

        info!("Loading multi-chain configuration from: {}", config_path);
        let config_file = ProverConfigFile::load_from_file(config_path)?;

        // Get keypair from enabled chains (Solana preferred, Starknet fallback)
        // For multi-chain support, we try Solana first, then derive from Starknet if needed
        let (keypair_arc, prover_authority) = Self::get_keypair_from_config(&config_file)?;

        info!("Prover Authority: {}", prover_authority);

        // Create marketplaces for all enabled chains
        let mut marketplaces = std::collections::HashMap::new();
        let mut starknet_event_configs = Vec::new();

        for (chain_name, chain_config) in config_file.enabled_chains() {
            info!("Initializing marketplace for chain: {}", chain_name);

            let marketplace = Self::create_marketplace_from_config(
                &chain_name,
                chain_config,
                keypair_arc.clone(),
            )?;

            marketplaces.insert(chain_name.clone(), marketplace);
            info!("Marketplace ready for chain: {}", chain_name);

            if let ChainConfig::Starknet(starknet) = chain_config {
                if starknet.enabled {
                    starknet_event_configs.push(StarknetEventListenerConfig {
                        rpc_url: starknet.rpc_url.clone(),
                        contract_address: starknet.contract_address.clone(),
                        poll_interval: Duration::from_secs(config_file.prover.poll_interval_secs),
                    });
                }
            }
        }

        if marketplaces.is_empty() {
            anyhow::bail!("No marketplaces initialized - at least one chain must be enabled");
        }

        info!("Initialized {} marketplace(s)", marketplaces.len());

        // Initialize Halo2 prover
        info!("Initializing Halo2 proving system...");
        let mut halo2_prover = Halo2Prover::new()?;
        halo2_prover.setup()?;
        info!("Halo2 prover ready");

        // Initialize witness encryption
        info!("Initializing witness encryption system...");
        let encryption_seed = derive_encryption_seed(&*keypair_arc);
        let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
        let pubkey = witness_encryption.public_key();
        info!("Witness encryption ready (pubkey: {})", hex::encode(pubkey));

        // Initialize witness fetcher
        info!("Initializing witness fetcher...");
        let witness_fetcher = WitnessFetcher::new(config_file.witness.base_url.clone());
        info!("Witness fetcher ready (backend: {})", config_file.witness.base_url);

        // Initialize GatewayClient
        info!("Initializing gateway client...");
        let gateway_client = GatewayClient::new(
            config_file.witness.base_url.clone(),
            Arc::new(keypair_arc.insecure_clone()),
        );
        info!("Gateway client ready (gateway: {})", config_file.witness.base_url);

        // Initialize FHE engine if server key is provided
        let fhe_engine = if let Some(ref key_path) = config_file.prover.fhe_server_key_path {
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

        // Initialize ROI calculator with operation mode
        let roi_calculator = ROICalculator::new(
            config_file.prover.min_roi_threshold,
            config_file.prover.cost_multiplier,
            config_file.prover.mode,
        );
        info!(
            "ROI calculator initialized - Mode: {} ({}), Min ROI: {:.1}%, Cost multiplier: {:.1}x",
            config_file.prover.mode,
            config_file.prover.mode.description(),
            config_file.prover.min_roi_threshold,
            config_file.prover.cost_multiplier
        );

        // Create shared references
        let halo2_prover_arc = Arc::new(halo2_prover);
        let witness_encryption_arc = Arc::new(witness_encryption);
        let witness_fetcher_arc = Arc::new(witness_fetcher);
        let gateway_client_arc = Arc::new(gateway_client);

        // Initialize JobProcessor with first marketplace as default
        let default_marketplace = marketplaces.values().next().unwrap().clone();
        let job_processor = Arc::new(JobProcessor::new(
            default_marketplace,
            keypair_arc.clone(),
            halo2_prover_arc,
            witness_encryption_arc,
            witness_fetcher_arc,
            gateway_client_arc,
            fhe_engine,
        ));

        // Create legacy ProverConfig for compatibility
        // Use prover_authority as keypair path placeholder in multi-chain mode
        let legacy_config = ProverConfig::new(
            "multi-chain".to_string(),
            Pubkey::default(), // Not used in multi-chain mode
            prover_authority.clone(), // Use authority string as placeholder
            Duration::from_secs(config_file.prover.poll_interval_secs),
            0, // Deprecated
            config_file.prover.min_roi_threshold,
            config_file.prover.cost_multiplier,
            Duration::from_secs(config_file.prover.mock_proving_time_secs),
            config_file.prover.max_concurrent_jobs,
            config_file.witness.base_url.clone(),
            config_file.witness.blink_backend_url.clone(),
            config_file.prover.zk_circuits_path.clone(),
            config_file.prover.fhe_server_key_path.clone(),
            None,
            None,
        );

        Ok(Self {
            marketplaces,
            keypair: keypair_arc,
            config: legacy_config,
            roi_calculator: Arc::new(roi_calculator),
            active_jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            job_processor,
            tui_state,
            start_time: std::time::Instant::now(),
            starknet_event_configs,
        })
    }

    /// Get keypair from configuration - supports Solana keypair file or Starknet private key
    ///
    /// For multi-chain mode:
    /// - If Solana is enabled: use Solana keypair file
    /// - If only Starknet: derive keypair from STARKNET_PRIVATE_KEY env var
    /// - Returns (keypair, prover_authority_string) for logging
    fn get_keypair_from_config(
        config_file: &config::ProverConfigFile,
    ) -> Result<(Arc<Keypair>, String)> {
        use config::ChainConfig;

        // Try Solana keypair first
        let solana_keypair_path = config_file.chains.iter()
            .find_map(|(_, chain_config)| {
                if let ChainConfig::Solana(ref sol) = chain_config {
                    if sol.enabled {
                        return sol.keypair_path.clone();
                    }
                }
                None
            });

        if let Some(keypair_path) = solana_keypair_path {
            // Solana mode: read keypair from file
            let keypair = read_keypair_file(&keypair_path)
                .map_err(|e| anyhow::anyhow!("Failed to read Solana keypair file: {}", e))?;
            let pubkey = keypair.pubkey().to_string();
            return Ok((Arc::new(keypair), pubkey));
        }

        // Try Starknet private key
        let starknet_config = config_file.chains.iter()
            .find_map(|(_, chain_config)| {
                if let ChainConfig::Starknet(ref stark) = chain_config {
                    if stark.enabled {
                        return Some(stark);
                    }
                }
                None
            });

        if let Some(stark_config) = starknet_config {
            // Starknet-only mode: derive keypair from private key or generate placeholder
            let private_key = std::env::var("STARKNET_PRIVATE_KEY")
                .context("STARKNET_PRIVATE_KEY environment variable required for Starknet-only mode")?;

            // Use Starknet prover address as authority identifier
            let prover_authority = stark_config.prover_address.clone();

            // Derive a deterministic Solana keypair from Starknet private key
            // This is used for internal signing (witness encryption, gateway auth)
            let seed = derive_seed_from_hex(&private_key)?;

            // Create ed25519 keypair from seed (Solana uses ed25519)
            // Keypair::from_bytes expects 64 bytes: [secret_key (32) || public_key (32)]
            use ed25519_dalek::{SigningKey, VerifyingKey};
            let signing_key = SigningKey::from_bytes(&seed);
            let verifying_key = VerifyingKey::from(&signing_key);

            let mut keypair_bytes = [0u8; 64];
            keypair_bytes[..32].copy_from_slice(signing_key.as_bytes());
            keypair_bytes[32..].copy_from_slice(verifying_key.as_bytes());

            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| anyhow::anyhow!("Failed to create keypair from Starknet seed: {}", e))?;

            info!("Running in Starknet-only mode (no Solana chain configured)");
            return Ok((Arc::new(keypair), prover_authority));
        }

        // No valid chain configuration
        anyhow::bail!("No enabled chain with valid keypair configuration found. \
            Either configure a Solana chain with keypair_path, or a Starknet chain with STARKNET_PRIVATE_KEY env var.")
    }

    /// Create a marketplace client from chain configuration
    fn create_marketplace_from_config(
        chain_name: &str,
        chain_config: &ChainConfig,
        keypair: Arc<Keypair>,
    ) -> Result<Arc<dyn MarketplaceOperations>> {
        use marketplace::{MarketplaceConfig, MarketplaceFactory};

        match chain_config {
            ChainConfig::Solana(sol_config) => {
                let program_id = sol_config.program_id.as_ref()
                    .or(sol_config.zk_generator_program.as_ref())
                    .or(sol_config.fhe_generator_program.as_ref())
                    .context("At least one program ID required for Solana chain")?;

                let marketplace_config = MarketplaceConfig::solana(
                    &sol_config.rpc_url,
                    program_id,
                ).with_generators(
                    sol_config.zk_generator_program.as_deref(),
                    sol_config.fhe_generator_program.as_deref(),
                );

                MarketplaceFactory::create(&marketplace_config, keypair)
            }
            ChainConfig::Starknet(stark_config) => {
                // Load private key from environment
                let private_key = std::env::var("STARKNET_PRIVATE_KEY").ok();

                let mut marketplace_config = MarketplaceConfig::starknet(
                    &stark_config.rpc_url,
                    &stark_config.contract_address,
                    &stark_config.prover_address,
                );

                if let Some(ref pk) = private_key {
                    marketplace_config = marketplace_config.with_starknet_signer(pk);
                }

                MarketplaceFactory::create(&marketplace_config, keypair)
            }
            ChainConfig::Aptos(aptos_config) => {
                // Load private key from environment
                let _private_key = std::env::var("APTOS_PRIVATE_KEY").ok();

                let marketplace_config = MarketplaceConfig::aptos(
                    &aptos_config.rpc_url,
                    &aptos_config.module_address,
                    &aptos_config.prover_address,
                );

                MarketplaceFactory::create(&marketplace_config, keypair)
            }
        }
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

        for config in &self.starknet_event_configs {
            let config = config.clone();
            tokio::spawn(async move {
                match StarknetEventListener::new(config).await {
                    Ok(listener) => {
                        if let Err(e) = listener.run().await {
                            log::warn!("Starknet event listener stopped: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to start Starknet event listener: {}", e);
                    }
                }
            });
        }

        loop {
            if let Err(e) = self.poll_and_process_jobs().await {
                error!("Error in job processing loop: {:#}", e);
            }

            sleep(self.config.poll_interval).await;
        }
    }

    /// Poll for available jobs and process them (multi-chain)
    async fn poll_and_process_jobs(&self) -> Result<()> {
        debug!("Polling for available jobs across {} chain(s)...", self.marketplaces.len());

        // Get current active job count
        let active_count = self.active_jobs.lock().await.len();
        if active_count >= self.config.max_concurrent_jobs {
            debug!(
                "Max concurrent jobs reached ({}/{}), skipping poll",
                active_count, self.config.max_concurrent_jobs
            );
            return Ok(());
        }

        // Poll all marketplaces in parallel
        let mut all_pending_jobs = Vec::new();
        let mut all_fhe_jobs = Vec::new();

        for (chain_name, marketplace) in &self.marketplaces {
            debug!("Polling chain: {}", chain_name);

            // Find pending ZK jobs
            match marketplace.find_pending_jobs().await {
                Ok(jobs) => {
                    debug!("Found {} pending jobs on {}", jobs.len(), chain_name);
                    // Tag jobs with chain name for later processing
                    all_pending_jobs.extend(jobs.into_iter().map(|job| (chain_name.clone(), job)));
                }
                Err(e) => {
                    warn!("Failed to find pending jobs on {}: {:#}", chain_name, e);
                }
            }

            // Also find FHE jobs that need more provers (for consensus)
            match marketplace.find_fhe_jobs_needing_provers().await {
                Ok(jobs) => {
                    debug!("Found {} FHE jobs needing provers on {}", jobs.len(), chain_name);
                    all_fhe_jobs.extend(jobs.into_iter().map(|job| (chain_name.clone(), job)));
                }
                Err(e) => {
                    warn!("Failed to find FHE jobs on {}: {:#}", chain_name, e);
                }
            }
        }

        let total_pending = all_pending_jobs.len() + all_fhe_jobs.len();
        info!(
            "Found {} pending jobs across all chains ({} ZK + {} FHE needing provers)",
            total_pending,
            all_pending_jobs.len(),
            all_fhe_jobs.len()
        );

        // Filter jobs using ROI calculator - only accept profitable jobs
        let mut suitable_jobs = Vec::new();
        let mut rejected_count = 0;

        // Process regular pending jobs (mostly ZK jobs)
        for (chain_name, job) in all_pending_jobs {
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
                suitable_jobs.push((chain_name, job, circuit_type, roi));
            } else {
                rejected_count += 1;
                debug!(
                    "[{}] Rejected job {} - Tier {}, ROI {:.1}%, Profit: {} lamports",
                    chain_name, job.id, roi.complexity_tier, roi.roi_percentage, roi.profit
                );
            }
        }

        // Process FHE jobs that need more provers
        for (chain_name, job) in all_fhe_jobs {
            let circuit_type = job.circuit_type.clone();

            // Get marketplace for this chain
            let marketplace = self.marketplaces.get(&chain_name)
                .context("Marketplace not found for chain")?;

            // Get FHE consensus config to determine required provers
            let fhe_config = marketplace.get_fhe_consensus_config(job.id).await
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
                        "[{}] FHE job {} needs provers: required {}",
                        chain_name, job.id, config.required_provers
                    );
                }
                suitable_jobs.push((chain_name, job, circuit_type, roi));
            } else {
                rejected_count += 1;
                debug!(
                    "[{}] Rejected FHE job {} - Tier {}, ROI {:.1}%, Profit: {} lamports",
                    chain_name, job.id, roi.complexity_tier, roi.roi_percentage, roi.profit
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
        for (chain_name, job, circuit_type, roi) in suitable_jobs.into_iter().take(slots_available) {
            info!(
                "[{}] Processing job {} - Price: {} lamports, Circuit: {:?}, ROI: {:.1}%, Profit: {} lamports",
                chain_name, job.id, job.price, circuit_type, roi.roi_percentage, roi.profit
            );

            // Create unique job identifier for multi-chain (chain:job_id:creator)
            let job_identifier = format!("{}:{}:{}", chain_name, job.id, job.creator);

            // Spawn job processing task
            let active_jobs = self.active_jobs.clone();
            let job_processor = self.job_processor.clone();
            let tui_state = self.tui_state.clone();
            let job_id = job.id;
            let job_clone = job.clone();
            let chain_name_clone = chain_name.clone();
            let job_identifier_clone = job_identifier.clone();

            tokio::spawn(async move {
                if let Err(e) = job_processor
                    .process_job(&job_clone, tui_state.clone())
                    .await
                {
                    error!("[{}] Failed to process job {}: {}", chain_name_clone, job_id, e);

                    // Update failed job stats
                    if let Some(ref tui) = tui_state {
                        tui.update_stats(|stats| {
                            stats.jobs_failed += 1;
                        });
                    }
                }

                // Remove from active jobs
                active_jobs.lock().await.retain(|id| id != &job_identifier_clone);
            });

            // Add to active jobs
            self.active_jobs.lock().await.push(job_identifier);
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
            // Start Futarchy poller in background thread if enabled
            let futarchy_handle = if args.enable_futarchy {
                Some(start_futarchy_poller(&args)?)
            } else {
                None
            };

            // Check if using multi-chain TOML config or legacy CLI flags
            if let Some(ref config_path) = args.config {
                info!("Using multi-chain configuration from: {}", config_path);

                if args.tui_mode {
                    // Run with TUI (multi-chain mode)
                    run_with_tui_multichain(config_path).await?;
                } else {
                    // Run headless (multi-chain mode)
                    let prover = ProverNode::new_from_config_file(config_path, None)?;
                    prover.run().await?;
                }
            } else {
                // Legacy mode: single chain via CLI flags
                info!("Using legacy CLI flags (single-chain mode)");
                let config = config_from_args(&args)?;

                if args.tui_mode {
                    // Run with TUI (legacy mode)
                    run_with_tui(config).await?;
                } else {
                    // Run headless (legacy mode)
                    let prover = ProverNode::new(config)?;
                    prover.run().await?;
                }
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

/// Run prover node with TUI interface (legacy single-chain mode)
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

/// Run prover node with TUI interface (multi-chain mode)
async fn run_with_tui_multichain(config_path: &str) -> Result<()> {
    use std::sync::Arc;

    // Create shared TUI state
    let tui_state = Arc::new(tui::TUIState::new());

    // Setup terminal
    let mut terminal = tui::setup_terminal()?;

    // Create TUI app
    let mut tui_app = tui::TUIApp::new(Arc::clone(&tui_state));

    // Start prover node in background task
    let prover_tui_state = Arc::clone(&tui_state);
    let config_path_owned = config_path.to_string();
    let prover_handle = tokio::spawn(async move {
        let prover = ProverNode::new_from_config_file(&config_path_owned, Some(prover_tui_state))?;
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

/// Derive a 32-byte seed from a hex string (for Starknet private key)
/// This allows Starknet-only mode to derive deterministic keys
fn derive_seed_from_hex(hex_key: &str) -> Result<[u8; 32]> {
    use solana_sdk::hash::hash;

    // Strip 0x prefix if present
    let hex_clean = hex_key.strip_prefix("0x").unwrap_or(hex_key);

    // Parse hex to bytes
    let key_bytes = hex::decode(hex_clean)
        .context("Invalid hex format for private key")?;

    // Hash with domain separation to get 32 bytes
    let mut seed_material = b"ZYBERLINK_STARKNET_KEYPAIR_V1:".to_vec();
    seed_material.extend_from_slice(&key_bytes);

    let hash_result = hash(&seed_material);
    Ok(hash_result.to_bytes())
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
