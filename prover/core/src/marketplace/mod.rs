//! Multi-chain marketplace abstraction layer
//!
//! This module provides a chain-agnostic interface for marketplace operations,
//! allowing prover-node to work with any blockchain that implements the
//! `MarketplaceOperations` trait.
//!
//! # Architecture
//!
//! ```text
//! ProverNode
//!     └── Arc<dyn MarketplaceOperations>
//!             ├── SolanaMarketplace (wraps existing MarketplaceClient)
//!             ├── AptosMarketplace (future)
//!             └── StarknetMarketplace (future)
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use prover_node::marketplace::{MarketplaceOperations, JobData};
//!
//! async fn process_jobs(marketplace: &dyn MarketplaceOperations) -> Result<()> {
//!     let jobs = marketplace.find_pending_jobs().await?;
//!     for job in jobs {
//!         let result = marketplace.claim_job(job.id).await?;
//!         println!("Claimed job {} with tx: {}", job.id, result.signature);
//!     }
//!     Ok(())
//! }
//! ```

mod types;
pub mod factory;
pub mod solana;
pub mod aptos;
pub mod starknet;

#[cfg(test)]
pub mod mock;

pub use types::*;
pub use factory::{ChainType, MarketplaceConfig, MarketplaceFactory};
pub use solana::SolanaMarketplace;
pub use aptos::AptosMarketplace;
pub use starknet::StarknetMarketplace;

use async_trait::async_trait;

/// Chain-agnostic marketplace operations trait
///
/// This trait abstracts all marketplace-specific operations (register, claim, submit)
/// independently of the underlying blockchain. Each chain implements this trait
/// using its specific SDK.
///
/// # Design Principles
///
/// 1. **Minimal Surface Area:** Only marketplace-specific operations, not generic RPC
/// 2. **Chain-Agnostic Types:** Uses `JobData` instead of Solana `Job` struct
/// 3. **Async by Default:** All I/O operations are async
/// 4. **Error Transparency:** Chain-specific errors wrapped in generic `MarketplaceError`
///
/// # Implementors
///
/// - `SolanaMarketplace`: Wraps existing `MarketplaceClient` from zyberlink-sdk
/// - `AptosMarketplace`: (Future) Wraps Aptos Move SDK
/// - `StarknetMarketplace`: (Future) Wraps Starknet Cairo SDK
#[async_trait]
pub trait MarketplaceOperations: Send + Sync {
    // ========== Prover Lifecycle ==========

    /// Register a new prover with the marketplace
    ///
    /// # Arguments
    /// * `stake` - Amount to stake in base units (lamports, octas, etc.)
    /// * `encryption_pubkey` - Public key for FHE job encryption (optional)
    ///
    /// # Returns
    /// Transaction result with registration signature
    async fn register_prover(
        &self,
        stake: u64,
        encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult>;

    /// Get prover data by authority address
    ///
    /// # Arguments
    /// * `authority` - Prover authority address as string
    ///
    /// # Returns
    /// Prover data if found, None otherwise
    async fn get_prover(&self, authority: &str) -> Result<Option<ProverData>>;

    /// Check if the current prover is registered
    async fn is_registered(&self) -> Result<bool>;

    // ========== Job Discovery ==========

    /// Find all pending ZK jobs available for claiming
    ///
    /// Returns jobs that:
    /// - Have status `Pending`
    /// - Have not expired
    /// - Match prover's supported circuit types (if filtered)
    async fn find_pending_jobs(&self) -> Result<Vec<JobData>>;

    /// Find all FHE jobs that need additional provers
    ///
    /// Returns jobs that:
    /// - Are FHE type
    /// - Have status `Pending` or need more provers for consensus
    /// - Have not expired
    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>>;

    /// Get a specific job by ID
    ///
    /// # Arguments
    /// * `job_id` - Unique job identifier
    /// * `creator` - Job creator address (needed for PDA derivation on some chains)
    async fn get_job(&self, job_id: u64, creator: &str) -> Result<Option<JobData>>;

    // ========== Job Execution ==========

    /// Claim a ZK job for processing
    ///
    /// # Arguments
    /// * `job_id` - Job to claim
    /// * `creator` - Job creator address
    ///
    /// # Returns
    /// Transaction result with claim signature
    ///
    /// # Errors
    /// - `JobAlreadyClaimed` if another prover claimed first
    /// - `JobExpired` if job timeout has passed
    async fn claim_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult>;

    /// Claim an FHE job for processing (legacy signature)
    ///
    /// # Arguments
    /// * `job_id` - Job to claim
    /// * `creator` - Job creator address
    ///
    /// # Returns
    /// Transaction result with claim signature
    ///
    /// # Note
    /// Prefer using `claim_fhe_job_v2` when you have the full JobData
    async fn claim_fhe_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult>;

    /// Claim an FHE job for processing (v2 - uses full JobData for new generators)
    ///
    /// This method supports both legacy zyberlink and new FHE-Generator programs.
    /// The job's `source` and `program_id` fields determine which program to use.
    ///
    /// # Arguments
    /// * `job` - Full job data including source and program_id
    ///
    /// # Returns
    /// Transaction result with claim signature
    async fn claim_fhe_job_v2(&self, job: &JobData) -> Result<TransactionResult> {
        // Default implementation falls back to legacy method
        self.claim_fhe_job(job.id, &job.creator).await
    }

    /// Submit a ZK proof result
    ///
    /// # Arguments
    /// * `job_id` - Job being completed
    /// * `creator` - Job creator address
    /// * `proof_commitment` - Commitment hash of the proof
    /// * `proof_size` - Size of the proof in bytes
    /// * `fee_recipient` - Optional address to receive fee (defaults to prover)
    ///
    /// # Returns
    /// Transaction result with submission signature
    async fn submit_proof(
        &self,
        job_id: u64,
        creator: &str,
        proof_commitment: [u8; 32],
        proof_size: u32,
        fee_recipient: Option<&str>,
    ) -> Result<TransactionResult>;

    /// Submit an FHE computation result (legacy signature)
    ///
    /// # Arguments
    /// * `job_id` - Job being completed
    /// * `creator` - Job creator address
    /// * `result_hash` - Hash of the FHE computation result
    ///
    /// # Returns
    /// Transaction result with submission signature
    ///
    /// # Note
    /// Prefer using `submit_fhe_result_v2` when you have the full JobData
    async fn submit_fhe_result(
        &self,
        job_id: u64,
        creator: &str,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult>;

    /// Submit an FHE computation result (v2 - uses full JobData for new generators)
    ///
    /// This method supports both legacy zyberlink and new FHE-Generator programs.
    ///
    /// # Arguments
    /// * `job` - Full job data including source and program_id
    /// * `result_hash` - Hash of the FHE computation result
    ///
    /// # Returns
    /// Transaction result with submission signature
    async fn submit_fhe_result_v2(
        &self,
        job: &JobData,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        // Default implementation falls back to legacy method
        self.submit_fhe_result(job.id, &job.creator, result_hash).await
    }

    // ========== FHE Consensus ==========

    /// Get FHE consensus configuration for a job
    ///
    /// # Arguments
    /// * `job_id` - Job ID to query
    ///
    /// # Returns
    /// Consensus config if job is FHE type, None otherwise
    async fn get_fhe_consensus_config(&self, job_id: u64) -> Result<Option<FheConsensusConfig>>;

    // ========== Metadata ==========

    /// Get the chain identifier (e.g., "solana", "aptos", "starknet")
    fn chain_id(&self) -> &str;

    /// Get the network name (e.g., "mainnet", "devnet", "testnet")
    fn network(&self) -> &str;

    /// Get the program/contract address for the marketplace
    fn program_address(&self) -> &str;

    /// Get the prover's public key/address
    fn prover_address(&self) -> &str;
}

/// Extension trait for marketplace operations with default implementations
#[async_trait]
pub trait MarketplaceOperationsExt: MarketplaceOperations {
    /// Find and filter jobs by minimum price
    async fn find_profitable_jobs(&self, min_price: u64) -> Result<Vec<JobData>> {
        let jobs = self.find_pending_jobs().await?;
        Ok(jobs.into_iter().filter(|j| j.price >= min_price).collect())
    }

    /// Claim a job with automatic retry on transient failures
    async fn claim_job_with_retry(
        &self,
        job_id: u64,
        creator: &str,
        max_retries: u32,
    ) -> Result<TransactionResult> {
        let mut last_error = None;
        for attempt in 0..max_retries {
            match self.claim_job(job_id, creator).await {
                Ok(result) => return Ok(result),
                Err(MarketplaceError::Network(e)) => {
                    log::warn!(
                        "Claim attempt {} failed with network error: {}",
                        attempt + 1,
                        e
                    );
                    last_error = Some(MarketplaceError::Network(e));
                    tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt as u64 + 1)))
                        .await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_error.unwrap_or(MarketplaceError::Other("Max retries exceeded".to_string())))
    }
}

// Blanket implementation for all MarketplaceOperations
impl<T: MarketplaceOperations + ?Sized> MarketplaceOperationsExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify trait is object-safe
    fn _assert_object_safe(_: &dyn MarketplaceOperations) {}

    #[test]
    fn test_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn MarketplaceOperations>>();
    }
}
