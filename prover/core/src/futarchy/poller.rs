//! Futarchy FHE Jobs Poller
//!
//! Hybrid flow for Futarchy FHE jobs:
//! 1. Discover jobs on-chain (Markets with pending_pool_update_job set)
//! 2. Fetch ciphertexts via API (too large for on-chain storage)
//! 3. Process FHE computation
//! 4. Submit results via API and on-chain

use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::Duration;
use futarchy_sdk::{query_position, query_market, FutarchyFheJob, find_pending_fhe_jobs, build_update_pool_ix};
use fhe_generator_sdk::derive_consensus_pda;

use super::{FutarchyPoolJob, FutarchyPoolWorker, FutarchyPoolResult};
use super::api_client::{FutarchyApiClient, ApiFheJob, FheJobData};

/// On-chain FHE job representation
/// Combines data from Market and Position accounts
#[derive(Debug, Clone)]
struct OnChainFheJob {
    market_id: u64,
    market_pubkey: Pubkey,
    position_pubkey: Option<Pubkey>,
    user_pubkey: Option<Pubkey>,
    side: bool,
    encrypted_pool: Vec<u8>,
    encrypted_bet: Option<Vec<u8>>,
    job_id: Option<u64>,
}

/// Configuration for the Futarchy poller
#[derive(Clone)]
pub struct FutarchyPollerConfig {
    /// Solana RPC URL (e.g., http://localhost:8899)
    pub solana_rpc_url: String,
    /// Futarchy program ID
    pub futarchy_program_id: solana_program::pubkey::Pubkey,
    /// FHE Generator program ID
    pub fhe_program_id: solana_program::pubkey::Pubkey,
    /// How often to poll for new jobs
    pub poll_interval: Duration,
    /// Maximum number of jobs to fetch per poll
    pub max_jobs_per_poll: usize,
    /// Prover keypair for signing transactions (optional, if None won't submit on-chain)
    pub prover_keypair: Option<Arc<Keypair>>,
    /// Blink server API URL for fetching ciphertexts (hybrid flow)
    pub api_server_url: Option<String>,
}

impl Default for FutarchyPollerConfig {
    fn default() -> Self {
        Self {
            solana_rpc_url: "http://localhost:8899".to_string(),
            futarchy_program_id: solana_program::pubkey::Pubkey::default(),
            fhe_program_id: solana_program::pubkey::Pubkey::default(),
            poll_interval: Duration::from_secs(5),
            max_jobs_per_poll: 10,
            prover_keypair: None,
            api_server_url: None,
        }
    }
}

impl std::fmt::Debug for FutarchyPollerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FutarchyPollerConfig")
            .field("solana_rpc_url", &self.solana_rpc_url)
            .field("futarchy_program_id", &self.futarchy_program_id)
            .field("fhe_program_id", &self.fhe_program_id)
            .field("poll_interval", &self.poll_interval)
            .field("max_jobs_per_poll", &self.max_jobs_per_poll)
            .field("prover_keypair", &self.prover_keypair.as_ref().map(|k| k.pubkey()))
            .field("api_server_url", &self.api_server_url)
            .finish()
    }
}

/// Poller for Futarchy FHE jobs
pub struct FutarchyPoller {
    config: FutarchyPollerConfig,
    rpc_client: RpcClient,
    worker: Arc<FutarchyPoolWorker>,
    api_client: Option<FutarchyApiClient>,
}

impl FutarchyPoller {
    /// Create a new poller
    pub fn new(config: FutarchyPollerConfig, worker: Arc<FutarchyPoolWorker>) -> Result<Self> {
        let rpc_client = RpcClient::new(config.solana_rpc_url.clone());
        let api_client = config.api_server_url.as_ref().map(|url| FutarchyApiClient::new(url));

        Ok(Self {
            config,
            rpc_client,
            worker,
            api_client,
        })
    }

    /// Poll for pending jobs and process them
    ///
    /// Uses hybrid flow when api_client is available:
    /// 1. Fetch pending jobs from API (faster, includes DB jobs)
    /// 2. Fetch ciphertexts via API
    /// 3. Process and submit via API
    /// 4. Optionally submit on-chain
    ///
    /// Falls back to pure on-chain flow if no api_client.
    ///
    /// Returns the number of jobs processed
    pub fn poll_and_process(&self) -> Result<usize> {
        // Use hybrid flow if api_client is available
        if let Some(ref api_client) = self.api_client {
            return self.poll_and_process_hybrid(api_client);
        }

        // Fallback: pure on-chain flow
        self.poll_and_process_onchain()
    }

    /// Hybrid flow: API discovery + API ciphertext fetch + on-chain submit
    fn poll_and_process_hybrid(&self, api_client: &FutarchyApiClient) -> Result<usize> {
        // 1. Fetch pending jobs from API
        let api_jobs = api_client.fetch_pending_jobs()
            .context("Failed to fetch pending jobs from API")?;

        if api_jobs.is_empty() {
            debug!("No pending Futarchy FHE jobs in API");
            return Ok(0);
        }

        info!("Found {} pending Futarchy FHE jobs via API", api_jobs.len());

        let mut processed = 0;

        // 2. Process each job
        for api_job in api_jobs.into_iter().take(self.config.max_jobs_per_poll) {
            match self.process_api_job(api_client, &api_job) {
                Ok(_) => {
                    processed += 1;
                    info!(
                        "Successfully processed Futarchy FHE job {} (market {}, side {})",
                        api_job.id,
                        api_job.market_id,
                        &api_job.side
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to process Futarchy FHE job {} (market {}): {}",
                        api_job.id, api_job.market_id, e
                    );
                }
            }
        }

        Ok(processed)
    }

    /// Process a single job from API (hybrid flow)
    fn process_api_job(&self, api_client: &FutarchyApiClient, api_job: &ApiFheJob) -> Result<()> {
        info!(
            "Processing Futarchy FHE job {} (market {}, side {}, onchain_job_id {:?})",
            api_job.id,
            api_job.market_id,
            &api_job.side,
            api_job.job_id
        );

        // 1. Fetch ciphertexts from API
        let job_data = api_client.fetch_job_data(api_job.id)
            .with_context(|| format!("Failed to fetch ciphertexts for job {}", api_job.id))?;

        // 2. Decode ciphertexts
        let bet_ciphertext = job_data.bet_ciphertext
            .as_ref()
            .ok_or_else(|| anyhow!("Missing bet_ciphertext in job data"))?;
        let bet_bytes = FutarchyApiClient::decode_ciphertext(bet_ciphertext)
            .context("Failed to decode bet_ciphertext")?;

        // Pool ciphertext may be null (first bet)
        let pool_bytes = if let Some(ref pool_ct) = job_data.pool_ciphertext {
            FutarchyApiClient::decode_ciphertext(pool_ct)
                .context("Failed to decode pool_ciphertext")?
        } else {
            // First bet: empty pool
            Vec::new()
        };

        info!(
            "Fetched ciphertexts for job {}: pool={} bytes, bet={} bytes",
            api_job.id,
            pool_bytes.len(),
            bet_bytes.len()
        );

        // 3. Create internal job and process
        let job = FutarchyPoolJob {
            job_id: api_job.job_id.map(|id| id as u64).unwrap_or(api_job.id as u64),
            market_id: api_job.market_id.clone(),
            side: api_job.side_bool(),
            bet_ciphertext: bet_bytes.clone(),
            pool_ciphertext: pool_bytes.clone(),
            bet_ciphertext_hash: FutarchyApiClient::decode_hash(&api_job.bet_ciphertext_hash)
                .unwrap_or([0u8; 32]),
            pool_ciphertext_hash: FutarchyApiClient::decode_hash(&api_job.pool_ciphertext_hash)
                .unwrap_or([0u8; 32]),
        };

        let result = self.worker.process_job(&job)
            .context("FHE processing failed")?;

        info!(
            "FHE computation complete for job {}: result_hash={}",
            api_job.id,
            hex::encode(&result.result_hash[..8])
        );

        // 4. Submit result via API
        let prover_pubkey = self.config.prover_keypair.as_ref().map(|k| k.pubkey().to_string());
        api_client.submit_result(api_job.id, &result.new_pool_ciphertext, prover_pubkey)
            .with_context(|| format!("Failed to submit result for job {}", api_job.id))?;

        info!("Submitted result via API for job {}", api_job.id);

        // 5. Optionally submit UpdatePool on-chain
        if let (Some(ref keypair), Some(onchain_job_id)) = (&self.config.prover_keypair, api_job.job_id) {
            let market_id_u64: u64 = api_job.market_id.parse().unwrap_or(0);
            let onchain_job = OnChainFheJob {
                market_id: market_id_u64,
                market_pubkey: futarchy_sdk::find_market_pda(&self.config.futarchy_program_id, market_id_u64).address,
                position_pubkey: None,
                user_pubkey: None,
                side: api_job.side_bool(),
                encrypted_pool: pool_bytes,
                encrypted_bet: Some(bet_bytes),
                job_id: Some(onchain_job_id as u64),
            };

            match self.submit_update_pool_onchain(&onchain_job, &result, keypair) {
                Ok(signature) => {
                    info!(
                        "Submitted UpdatePool on-chain for job {}: {}",
                        api_job.id, signature
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to submit UpdatePool on-chain for job {} (API submit succeeded): {}",
                        api_job.id, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Pure on-chain flow (fallback when no API)
    fn poll_and_process_onchain(&self) -> Result<usize> {
        // 1. Fetch pending FHE jobs from blockchain using SDK
        let jobs = self.fetch_pending_jobs_onchain()?;

        if jobs.is_empty() {
            debug!("No pending Futarchy FHE jobs on-chain");
            return Ok(0);
        }

        info!("Found {} pending Futarchy FHE jobs on-chain", jobs.len());

        let mut processed = 0;

        // 2. Process each job
        for onchain_job in jobs {
            match self.process_onchain_job(&onchain_job) {
                Ok(_) => {
                    processed += 1;
                    info!(
                        "Successfully processed Futarchy FHE job for market {}",
                        onchain_job.market_id
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to process Futarchy FHE job for market {}: {}",
                        onchain_job.market_id, e
                    );
                }
            }
        }

        Ok(processed)
    }

    /// Fetch pending jobs from blockchain
    fn fetch_pending_jobs_onchain(&self) -> Result<Vec<OnChainFheJob>> {
        debug!("Fetching pending FHE jobs from Solana blockchain");

        // Use SDK to find pending FHE jobs
        let sdk_jobs = tokio::runtime::Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(find_pending_fhe_jobs(
                &self.rpc_client,
                &self.config.futarchy_program_id,
            ))
            .context("Failed to query pending FHE jobs from blockchain")?;

        // For each job, we need to fetch Position accounts to get encrypted_amount
        let mut jobs = Vec::new();

        for sdk_job in sdk_jobs.into_iter().take(self.config.max_jobs_per_poll) {
            // Fetch all Position accounts for this market to find the one with encrypted_amount
            let encrypted_bet = self.fetch_latest_position_encrypted_amount(
                &sdk_job.market_pubkey,
                sdk_job.side,
            )?;

            if encrypted_bet.is_none() {
                warn!(
                    "No Position with encrypted_amount found for market {} (side: {}), skipping",
                    sdk_job.market_id,
                    if sdk_job.side { "YES" } else { "NO" }
                );
                continue;
            }

            jobs.push(OnChainFheJob {
                market_id: sdk_job.market_id,
                market_pubkey: sdk_job.market_pubkey,
                position_pubkey: None,
                user_pubkey: None,
                side: sdk_job.side,
                encrypted_pool: sdk_job.encrypted_pool,
                encrypted_bet,
                job_id: sdk_job.job_id,
            });
        }

        debug!("Found {} pending FHE jobs with Position data on-chain", jobs.len());

        Ok(jobs)
    }

    /// Fetch the latest Position encrypted_amount for a market
    ///
    /// This queries all Position accounts and finds the most recent one with encrypted_amount
    fn fetch_latest_position_encrypted_amount(
        &self,
        market_pubkey: &Pubkey,
        _side: bool,
    ) -> Result<Option<Vec<u8>>> {
        use borsh::BorshDeserialize;
        use solana_client::rpc_config::RpcProgramAccountsConfig;
        use solana_account_decoder::UiAccountEncoding;
        use solana_sdk::commitment_config::CommitmentConfig;
        use futarchy_sdk::Position;

        // Get all program accounts for futarchy program
        let accounts = self.rpc_client.get_program_accounts_with_config(
            &self.config.futarchy_program_id,
            RpcProgramAccountsConfig {
                filters: None,
                account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
                with_context: None,
                sort_results: None,
            },
        ).context("Failed to get program accounts")?;

        // Find Position accounts for this market with encrypted_amount
        let mut latest_position: Option<(i64, Vec<u8>)> = None;

        for (_pubkey, account) in accounts {
            if let Ok(position) = Position::try_from_slice(&account.data) {
                // Check if this position is for the target market
                if position.market == *market_pubkey {
                    // Check if it has encrypted_amount
                    if let Some(encrypted_amount) = position.encrypted_amount {
                        // Keep the latest one (by placed_at timestamp)
                        if let Some((latest_timestamp, _)) = latest_position.as_ref() {
                            if position.placed_at > *latest_timestamp {
                                latest_position = Some((position.placed_at, encrypted_amount));
                            }
                        } else {
                            latest_position = Some((position.placed_at, encrypted_amount));
                        }
                    }
                }
            }
        }

        Ok(latest_position.map(|(_, encrypted)| encrypted))
    }

    /// Process a single on-chain job
    fn process_onchain_job(&self, onchain_job: &OnChainFheJob) -> Result<()> {
        info!(
            "Processing Futarchy FHE job for market {} (side: {})",
            onchain_job.market_id,
            if onchain_job.side { "YES" } else { "NO" }
        );

        // Convert on-chain job to internal job format
        let job = self.convert_onchain_job(onchain_job)?;

        // Process with worker
        let result = self.worker.process_job(&job)?;

        info!(
            "Futarchy FHE job for market {} completed. Result hash: {}",
            onchain_job.market_id,
            hex::encode(&result.result_hash[..8])
        );

        // Submit UpdatePool on-chain if we have a keypair
        if let Some(ref keypair) = self.config.prover_keypair {
            match self.submit_update_pool_onchain(onchain_job, &result, keypair) {
                Ok(signature) => {
                    info!(
                        "Successfully submitted UpdatePool TX for market {}: {}",
                        onchain_job.market_id, signature
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to submit UpdatePool TX for market {}: {}",
                        onchain_job.market_id, e
                    );
                    // Don't fail the whole job, just log the error
                    // The HTTP submission via worker may have succeeded
                }
            }
        } else {
            warn!(
                "No keypair configured, skipping on-chain UpdatePool for market {}",
                onchain_job.market_id
            );
        }

        Ok(())
    }

    /// Submit UpdatePool transaction on-chain
    fn submit_update_pool_onchain(
        &self,
        onchain_job: &OnChainFheJob,
        result: &FutarchyPoolResult,
        keypair: &Keypair,
    ) -> Result<String> {
        let fhe_job_id = onchain_job.job_id.ok_or_else(|| {
            anyhow!("Missing FHE job ID for on-chain submission")
        })?;

        // Derive FHE accounts
        // Note: For MVP, UpdatePool doesn't strictly verify these, but we include them for completeness
        let job_id_bytes = fhe_job_id.to_le_bytes();
        let (fhe_job_account, _) = Pubkey::find_program_address(
            &[b"fhe_job", onchain_job.market_pubkey.as_ref(), &job_id_bytes],
            &self.config.fhe_program_id,
        );
        let (fhe_consensus_account, _) = derive_consensus_pda(&self.config.fhe_program_id, fhe_job_id);

        info!(
            "Building UpdatePool instruction: market_id={}, fhe_job_id={}, side={}, result_size={}",
            onchain_job.market_id,
            fhe_job_id,
            if onchain_job.side { "YES" } else { "NO" },
            result.new_pool_ciphertext.len()
        );

        // Build UpdatePool instruction
        let instruction = build_update_pool_ix(
            &self.config.futarchy_program_id,
            &keypair.pubkey(),
            onchain_job.market_id,
            fhe_job_id,
            result.new_pool_ciphertext.clone(),
            onchain_job.side,
            &fhe_job_account,
            &fhe_consensus_account,
        ).map_err(|e| anyhow!("Failed to build UpdatePool instruction: {}", e))?;

        // Get recent blockhash
        let blockhash = self.rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .map_err(|e| anyhow!("Failed to get blockhash: {}", e))?
            .0;

        // Build and sign transaction
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&keypair.pubkey()));
        transaction.sign(&[keypair], blockhash);

        // Send transaction
        let signature = self.rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .map_err(|e| anyhow!("Failed to send UpdatePool transaction: {}", e))?;

        Ok(signature.to_string())
    }

    /// Convert on-chain job to internal job format
    fn convert_onchain_job(&self, onchain_job: &OnChainFheJob) -> Result<FutarchyPoolJob> {
        use fhe_client_sdk::FutarchyFheClient;

        let pool_ciphertext = onchain_job.encrypted_pool.clone();
        let pool_hash = FutarchyFheClient::hash_ciphertext(&pool_ciphertext);

        let bet_ciphertext = onchain_job.encrypted_bet.as_ref()
            .ok_or_else(|| anyhow!("Missing encrypted bet ciphertext"))?
            .clone();
        let bet_hash = FutarchyFheClient::hash_ciphertext(&bet_ciphertext);

        Ok(FutarchyPoolJob {
            job_id: onchain_job.job_id.unwrap_or(0),
            market_id: onchain_job.market_pubkey.to_string(),
            side: onchain_job.side,
            bet_ciphertext,
            pool_ciphertext,
            bet_ciphertext_hash: bet_hash,
            pool_ciphertext_hash: pool_hash,
        })
    }


    /// Run the poller loop (blocking)
    pub fn run_loop(&self) -> ! {
        info!(
            "Starting Futarchy FHE poller. RPC: {}, Interval: {:?}",
            self.config.solana_rpc_url, self.config.poll_interval
        );

        loop {
            match self.poll_and_process() {
                Ok(count) => {
                    if count > 0 {
                        info!("Processed {} Futarchy FHE jobs from blockchain", count);
                    }
                }
                Err(e) => {
                    error!("Error in Futarchy poller: {}", e);
                }
            }

            std::thread::sleep(self.config.poll_interval);
        }
    }
}

