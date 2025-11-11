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

mod halo2_prover;
mod witness_encryption;

use halo2_prover::{Halo2Prover, OrchardWitness};
use witness_encryption::WitnessEncryption;

/// CypherLink Prover Node - Autonomous ZK proof generation daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Solana RPC URL
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// CypherLink program ID
    #[arg(short, long)]
    program_id: String,

    /// Path to prover keypair file
    #[arg(short, long, default_value = "~/.config/solana/id.json")]
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
}

impl ProverConfig {
    fn from_args(args: Args) -> Result<Self> {
        let program_id = args
            .program_id
            .parse()
            .context("Invalid program ID format")?;

        Ok(Self {
            rpc_url: args.rpc_url,
            program_id,
            keypair_path: args.keypair.replace("~", &std::env::var("HOME").unwrap_or_default()),
            poll_interval: Duration::from_secs(args.poll_interval),
            min_price: args.min_price,
            mock_proving_time: Duration::from_secs(args.mock_proving_time),
            max_concurrent_jobs: args.max_concurrent_jobs,
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

        // Initialize witness encryption
        info!("Initializing witness encryption system...");
        let witness_encryption = WitnessEncryption::new()?;
        let pubkey = witness_encryption.public_key();
        info!("Witness encryption ready (pubkey: {})", hex::encode(&pubkey));

        Ok(Self {
            client: Arc::new(client),
            keypair: Arc::new(keypair),
            config,
            active_jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            halo2_prover: Arc::new(halo2_prover),
            witness_encryption: Arc::new(witness_encryption),
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

            tokio::spawn(async move {
                if let Err(e) =
                    Self::process_job(
                        client,
                        keypair,
                        job_pda,
                        job.id,
                        job.circuit_type,
                        mock_proving_time,
                        halo2_prover,
                        witness_encryption,
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
        _mock_proving_time: Duration,
        halo2_prover: Arc<Halo2Prover>,
        witness_encryption: Arc<WitnessEncryption>,
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

        if job.status != JobStatus::Claimed || job.prover != Some(keypair.pubkey()) {
            warn!("[Job {}] Job not claimed by us, aborting", job_id);
            return Ok(());
        }

        // Step 2: Generate proof (REAL Halo2)
        info!(
            "[Job {}] Generating proof (circuit: {:?})...",
            job_id, circuit_type
        );

        // Decrypt witness from job data
        // In production, job.witness_data would contain the encrypted witness
        // For now, we'll simulate this by encrypting a dummy witness for demonstration
        info!("[Job {}] Decrypting witness data...", job_id);

        // TODO: Replace this with actual encrypted witness from job.witness_data
        // For now, simulate encryption/decryption with dummy data
        let dummy_witness = OrchardWitness::dummy();
        let simulated_encrypted = WitnessEncryption::encrypt_witness(
            &dummy_witness,
            &witness_encryption.public_key()
        ).context("Failed to simulate witness encryption")?;

        let witness = witness_encryption
            .decrypt_witness(&simulated_encrypted)
            .context("Failed to decrypt witness data")?;

        info!("[Job {}] Witness decrypted successfully", job_id);

        // Validate witness
        witness
            .validate()
            .context("Invalid witness data")?;

        // Generate real Halo2 proof
        let proof_bytes = Self::real_generate_proof(halo2_prover.clone(), witness).await?;

        info!(
            "[Job {}] Proof generated successfully ({} bytes)",
            job_id,
            proof_bytes.len()
        );

        // Step 3: Submit proof
        info!("[Job {}] Submitting proof...", job_id);

        // Generate proof commitment (hash of actual proof)
        let proof_commitment = Self::generate_proof_commitment(&proof_bytes);
        let proof_size = proof_bytes.len() as u32;

        // Fetch config to get protocol fee recipient
        let (config_pda, _) = client.get_config_pda();
        let config_account = client.rpc_client.get_account(&config_pda)
            .context("Failed to fetch config account")?;

        // Extract protocol_fee_recipient from config
        // Offset: authority(32) + fee_basis_points(2) + min_stake(8) + min_reputation(4) + timeout(8) = 54
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
                info!("[Job {}] Proof submitted successfully (sig: {})", job_id, sig);
                info!("[Job {}] Completed! 🎉", job_id);
            }
            Err(e) => {
                error!("[Job {}] Failed to submit proof: {}", job_id, e);
                return Err(e.into());
            }
        }

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
    let config = ProverConfig::from_args(args)?;

    // Create and run prover node
    let prover = ProverNode::new(config)?;
    prover.run().await?;

    Ok(())
}
