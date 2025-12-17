//! Futarchy Pool Worker
//!
//! Processes FHE jobs for Futarchy Markets:
//! - Fetches current encrypted pool from app server
//! - Fetches encrypted bet amount from app server
//! - Performs homomorphic addition: new_pool = pool + bet
//! - Submits result back to app server
//! - Submits result hash to chain for consensus

use anyhow::{anyhow, Context, Result};
use fhe_client_sdk::FutarchyFheClient;
use serde::{Deserialize, Serialize};

use super::CiphertextFetcher;

/// Job data for Futarchy pool updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutarchyPoolJob {
    /// Unique job ID from chain
    pub job_id: u64,
    /// Market ID (pubkey as string)
    pub market_id: String,
    /// Side of the bet (true = YES, false = NO)
    pub side: bool,
    /// Hash of the bet ciphertext (32 bytes)
    pub bet_ciphertext_hash: [u8; 32],
    /// Hash of the current pool ciphertext (32 bytes)
    pub pool_ciphertext_hash: [u8; 32],
}

/// Result of processing a Futarchy pool job
#[derive(Debug, Clone)]
pub struct FutarchyPoolResult {
    /// Job ID that was processed
    pub job_id: u64,
    /// Hash of the new pool ciphertext
    pub result_hash: [u8; 32],
    /// The new pool ciphertext (for storage)
    pub new_pool_ciphertext: Vec<u8>,
}

/// Worker for processing Futarchy FHE pool updates
pub struct FutarchyPoolWorker {
    /// FHE client for homomorphic operations
    fhe_client: FutarchyFheClient,
    /// Fetcher for getting ciphertexts from app server
    ciphertext_fetcher: CiphertextFetcher,
}

impl FutarchyPoolWorker {
    /// Create a new worker
    ///
    /// # Arguments
    /// * `server_key_bytes` - Serialized FHE server key
    /// * `app_server_url` - URL of the app server storing ciphertexts
    pub fn new(server_key_bytes: &[u8], app_server_url: &str) -> Result<Self> {
        // For server-side, we only need the server key (no client key needed)
        // But FutarchyFheClient requires both. We'll use a dummy client key.
        // In production, we should refactor to have a ServerOnlyClient.

        // For now, create a new client (expensive but works for PoC)
        // TODO: Refactor to load server key only
        let fhe_client = FutarchyFheClient::new()
            .map_err(|e| anyhow!("Failed to create FHE client: {}", e))?;

        let ciphertext_fetcher = CiphertextFetcher::new(app_server_url);

        Ok(Self {
            fhe_client,
            ciphertext_fetcher,
        })
    }

    /// Create worker with existing FHE client
    pub fn with_client(fhe_client: FutarchyFheClient, app_server_url: &str) -> Self {
        Self {
            fhe_client,
            ciphertext_fetcher: CiphertextFetcher::new(app_server_url),
        }
    }

    /// Process a single pool update job
    ///
    /// Flow:
    /// 1. Fetch current pool ciphertext from app server
    /// 2. Fetch bet ciphertext from app server
    /// 3. Verify hashes match what's on-chain
    /// 4. Perform homomorphic addition: new_pool = pool + bet
    /// 5. Return result with hash for consensus
    pub fn process_job(&self, job: &FutarchyPoolJob) -> Result<FutarchyPoolResult> {
        log::info!(
            "Processing Futarchy pool job {} for market {} (side: {})",
            job.job_id,
            job.market_id,
            if job.side { "YES" } else { "NO" }
        );

        // 1. Fetch current pool
        let pool_ciphertext = self
            .ciphertext_fetcher
            .fetch_by_hash(&job.pool_ciphertext_hash)
            .context("Failed to fetch pool ciphertext")?;

        // 2. Fetch bet
        let bet_ciphertext = self
            .ciphertext_fetcher
            .fetch_by_hash(&job.bet_ciphertext_hash)
            .context("Failed to fetch bet ciphertext")?;

        // 3. Verify hashes (already done in fetcher, but double-check)
        let pool_hash = FutarchyFheClient::hash_ciphertext(&pool_ciphertext);
        let bet_hash = FutarchyFheClient::hash_ciphertext(&bet_ciphertext);

        if pool_hash != job.pool_ciphertext_hash {
            return Err(anyhow!("Pool ciphertext hash mismatch"));
        }
        if bet_hash != job.bet_ciphertext_hash {
            return Err(anyhow!("Bet ciphertext hash mismatch"));
        }

        log::debug!(
            "Ciphertexts verified. Pool: {} bytes, Bet: {} bytes",
            pool_ciphertext.len(),
            bet_ciphertext.len()
        );

        // 4. Perform homomorphic addition
        log::info!("Performing homomorphic addition...");
        let start = std::time::Instant::now();

        let new_pool_ciphertext = self
            .fhe_client
            .homomorphic_add(&pool_ciphertext, &bet_ciphertext)
            .map_err(|e| anyhow!("Homomorphic addition failed: {}", e))?;

        let elapsed = start.elapsed();
        log::info!(
            "Homomorphic addition completed in {:?}. Result: {} bytes",
            elapsed,
            new_pool_ciphertext.len()
        );

        // 5. Compute result hash
        let result_hash = FutarchyFheClient::hash_ciphertext(&new_pool_ciphertext);

        log::info!(
            "Job {} completed. Result hash: {}",
            job.job_id,
            hex::encode(result_hash)
        );

        Ok(FutarchyPoolResult {
            job_id: job.job_id,
            result_hash,
            new_pool_ciphertext,
        })
    }

    /// Process job and submit result to app server
    pub fn process_and_submit(&self, job: &FutarchyPoolJob) -> Result<FutarchyPoolResult> {
        let result = self.process_job(job)?;

        // Submit to app server
        self.ciphertext_fetcher.submit_pool_update(
            &job.market_id,
            job.side,
            &result.new_pool_ciphertext,
            job.job_id,
        )?;

        Ok(result)
    }

    /// Get reference to the FHE client
    pub fn fhe_client(&self) -> &FutarchyFheClient {
        &self.fhe_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_serialization() {
        let job = FutarchyPoolJob {
            job_id: 12345,
            market_id: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
            side: true,
            bet_ciphertext_hash: [1u8; 32],
            pool_ciphertext_hash: [2u8; 32],
        };

        let json = serde_json::to_string(&job).unwrap();
        let restored: FutarchyPoolJob = serde_json::from_str(&json).unwrap();

        assert_eq!(job.job_id, restored.job_id);
        assert_eq!(job.market_id, restored.market_id);
        assert_eq!(job.side, restored.side);
    }
}
