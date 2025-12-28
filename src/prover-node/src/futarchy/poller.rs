//! Futarchy FHE Jobs Poller
//!
//! Polls the Solana blockchain for pending Futarchy FHE jobs and processes them.
//! This reads directly from on-chain state (Position and Market accounts) instead
//! of relying on an API server.

use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use std::sync::Arc;
use std::time::Duration;
use futarchy_sdk::{query_position, query_market, FutarchyFheJob, find_pending_fhe_jobs};

use super::{FutarchyPoolJob, FutarchyPoolWorker};

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
#[derive(Debug, Clone)]
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
}

impl Default for FutarchyPollerConfig {
    fn default() -> Self {
        Self {
            solana_rpc_url: "http://localhost:8899".to_string(),
            futarchy_program_id: solana_program::pubkey::Pubkey::default(),
            fhe_program_id: solana_program::pubkey::Pubkey::default(),
            poll_interval: Duration::from_secs(5),
            max_jobs_per_poll: 10,
        }
    }
}

/// Poller for Futarchy FHE jobs
pub struct FutarchyPoller {
    config: FutarchyPollerConfig,
    rpc_client: RpcClient,
    worker: Arc<FutarchyPoolWorker>,
}

impl FutarchyPoller {
    /// Create a new poller
    pub fn new(config: FutarchyPollerConfig, worker: Arc<FutarchyPoolWorker>) -> Result<Self> {
        let rpc_client = RpcClient::new(config.solana_rpc_url.clone());

        Ok(Self {
            config,
            rpc_client,
            worker,
        })
    }

    /// Poll for pending jobs and process them
    ///
    /// Returns the number of jobs processed
    pub fn poll_and_process(&self) -> Result<usize> {
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

        Ok(())
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

