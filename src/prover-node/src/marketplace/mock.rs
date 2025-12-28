//! Mock implementation of MarketplaceOperations for testing
//!
//! Provides a configurable mock that can simulate various marketplace behaviors
//! including success cases, failures, and edge conditions.

use super::{
    FheConsensusConfig, JobData, JobSource, MarketplaceError, MarketplaceOperations, ProverData,
    Result, TransactionResult,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use zyberlink_types::{CircuitType, JobStatus};

/// Mock marketplace for testing
///
/// Provides full control over marketplace behavior for unit tests.
/// All state is stored in memory and can be configured before tests.
///
/// # Example
///
/// ```rust,ignore
/// use prover_node::marketplace::mock::MockMarketplace;
///
/// #[tokio::test]
/// async fn test_claim_job() {
///     let mut mock = MockMarketplace::new();
///     mock.add_pending_job(JobData { id: 1, ... });
///
///     let result = mock.claim_job(1, "creator").await.unwrap();
///     assert!(result.success);
/// }
/// ```
pub struct MockMarketplace {
    chain_id: String,
    network: String,
    program_address: String,
    prover_address: String,

    // Internal state
    jobs: Arc<RwLock<HashMap<u64, JobData>>>,
    provers: Arc<RwLock<HashMap<String, ProverData>>>,
    is_registered: Arc<RwLock<bool>>,

    // Behavior configuration
    should_fail_claims: Arc<RwLock<bool>>,
    should_fail_submissions: Arc<RwLock<bool>>,
    claim_delay_ms: Arc<RwLock<u64>>,
    tx_counter: Arc<RwLock<u64>>,
}

impl MockMarketplace {
    /// Create a new mock marketplace with default configuration
    pub fn new() -> Self {
        Self {
            chain_id: "mock".to_string(),
            network: "testnet".to_string(),
            program_address: "MockProgram111111111111111111111111111111111".to_string(),
            prover_address: "MockProver1111111111111111111111111111111111".to_string(),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            provers: Arc::new(RwLock::new(HashMap::new())),
            is_registered: Arc::new(RwLock::new(false)),
            should_fail_claims: Arc::new(RwLock::new(false)),
            should_fail_submissions: Arc::new(RwLock::new(false)),
            claim_delay_ms: Arc::new(RwLock::new(0)),
            tx_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Create mock configured for a specific chain
    pub fn for_chain(chain_id: &str, network: &str) -> Self {
        let mut mock = Self::new();
        mock.chain_id = chain_id.to_string();
        mock.network = network.to_string();
        mock
    }

    /// Set the prover address
    pub fn with_prover_address(mut self, address: &str) -> Self {
        self.prover_address = address.to_string();
        self
    }

    /// Add a job to the mock marketplace
    pub fn add_job(&self, job: JobData) {
        let mut jobs = self.jobs.write().unwrap();
        jobs.insert(job.id, job);
    }

    /// Add a pending job with minimal configuration
    pub fn add_pending_job(&self, id: u64, creator: &str, price: u64) {
        self.add_job(JobData {
            id,
            creator: creator.to_string(),
            circuit_type: CircuitType::AnonymousVote,
            witness_hash: [0u8; 32],
            price,
            status: JobStatus::Pending,
            prover: None,
            created_at: chrono::Utc::now().timestamp(),
            timeout_at: chrono::Utc::now().timestamp() + 3600,
            is_fhe: false,
            address: format!("JobPDA{}", id),
            source: JobSource::Legacy,
            program_id: None,
            starknet_job_type: None,
            encrypted_c1: None,
            encrypted_c2: None,
            payload_hash: None,
        });
    }

    /// Add a pending FHE job
    pub fn add_pending_fhe_job(&self, id: u64, creator: &str, price: u64) {
        self.add_job(JobData {
            id,
            creator: creator.to_string(),
            circuit_type: CircuitType::AnonymousVote,
            witness_hash: [0u8; 32],
            price,
            status: JobStatus::Pending,
            prover: None,
            created_at: chrono::Utc::now().timestamp(),
            timeout_at: chrono::Utc::now().timestamp() + 3600,
            is_fhe: true,
            address: format!("FheJobPDA{}", id),
            source: JobSource::Legacy,
            program_id: None,
            starknet_job_type: None,
            encrypted_c1: None,
            encrypted_c2: None,
            payload_hash: None,
        });
    }

    /// Register a prover in the mock
    pub fn add_prover(&self, prover: ProverData) {
        let mut provers = self.provers.write().unwrap();
        provers.insert(prover.authority.clone(), prover);
    }

    /// Set whether the prover is registered
    pub fn set_registered(&self, registered: bool) {
        let mut is_reg = self.is_registered.write().unwrap();
        *is_reg = registered;
    }

    /// Configure claims to fail
    pub fn set_claims_should_fail(&self, should_fail: bool) {
        let mut fail = self.should_fail_claims.write().unwrap();
        *fail = should_fail;
    }

    /// Configure submissions to fail
    pub fn set_submissions_should_fail(&self, should_fail: bool) {
        let mut fail = self.should_fail_submissions.write().unwrap();
        *fail = should_fail;
    }

    /// Add artificial delay to claims (simulates network latency)
    pub fn set_claim_delay(&self, delay_ms: u64) {
        let mut delay = self.claim_delay_ms.write().unwrap();
        *delay = delay_ms;
    }

    /// Get a job by ID (for test assertions)
    pub fn get_job(&self, id: u64) -> Option<JobData> {
        let jobs = self.jobs.read().unwrap();
        jobs.get(&id).cloned()
    }

    /// Get all jobs (for test assertions)
    pub fn get_all_jobs(&self) -> Vec<JobData> {
        let jobs = self.jobs.read().unwrap();
        jobs.values().cloned().collect()
    }

    /// Generate a mock transaction signature
    fn next_tx_signature(&self) -> String {
        let mut counter = self.tx_counter.write().unwrap();
        *counter += 1;
        format!("MockTx{:016x}", *counter)
    }
}

impl Default for MockMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceOperations for MockMarketplace {
    async fn register_prover(
        &self,
        stake: u64,
        encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult> {
        let prover = ProverData {
            authority: self.prover_address.clone(),
            stake,
            encryption_pubkey,
            is_active: true,
            jobs_completed: 0,
            jobs_failed: 0,
            reputation: 100,
            registered_at: chrono::Utc::now().timestamp(),
        };

        self.add_prover(prover);
        self.set_registered(true);

        Ok(TransactionResult::success(self.next_tx_signature()))
    }

    async fn get_prover(&self, authority: &str) -> Result<Option<ProverData>> {
        let provers = self.provers.read().unwrap();
        Ok(provers.get(authority).cloned())
    }

    async fn is_registered(&self) -> Result<bool> {
        let is_reg = self.is_registered.read().unwrap();
        Ok(*is_reg)
    }

    async fn find_pending_jobs(&self) -> Result<Vec<JobData>> {
        let jobs = self.jobs.read().unwrap();
        Ok(jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending && !j.is_fhe)
            .cloned()
            .collect())
    }

    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>> {
        let jobs = self.jobs.read().unwrap();
        Ok(jobs
            .values()
            .filter(|j| j.is_fhe && j.status == JobStatus::Pending)
            .cloned()
            .collect())
    }

    async fn get_job(&self, job_id: u64, _creator: &str) -> Result<Option<JobData>> {
        let jobs = self.jobs.read().unwrap();
        Ok(jobs.get(&job_id).cloned())
    }

    async fn claim_job(&self, job_id: u64, _creator: &str) -> Result<TransactionResult> {
        // Check if should fail
        if *self.should_fail_claims.read().unwrap() {
            return Err(MarketplaceError::TransactionFailed("Mock failure".to_string()));
        }

        // Simulate delay
        let delay = *self.claim_delay_ms.read().unwrap();
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        // Update job status
        let mut jobs = self.jobs.write().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            if job.status != JobStatus::Pending {
                return Err(MarketplaceError::JobAlreadyClaimed);
            }
            job.status = JobStatus::Claimed;
            job.prover = Some(self.prover_address.clone());
            Ok(TransactionResult::success(self.next_tx_signature()))
        } else {
            Err(MarketplaceError::JobNotFound(job_id))
        }
    }

    async fn claim_fhe_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult> {
        // Same logic as claim_job for mock
        self.claim_job(job_id, creator).await
    }

    async fn submit_proof(
        &self,
        job_id: u64,
        _creator: &str,
        _proof_commitment: [u8; 32],
        _proof_size: u32,
        _fee_recipient: Option<&str>,
    ) -> Result<TransactionResult> {
        if *self.should_fail_submissions.read().unwrap() {
            return Err(MarketplaceError::TransactionFailed("Mock failure".to_string()));
        }

        let mut jobs = self.jobs.write().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Completed;
            Ok(TransactionResult::success(self.next_tx_signature()))
        } else {
            Err(MarketplaceError::JobNotFound(job_id))
        }
    }

    async fn submit_fhe_result(
        &self,
        job_id: u64,
        _creator: &str,
        _result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        if *self.should_fail_submissions.read().unwrap() {
            return Err(MarketplaceError::TransactionFailed("Mock failure".to_string()));
        }

        let mut jobs = self.jobs.write().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            // FHE jobs stay claimed until consensus
            Ok(TransactionResult::success(self.next_tx_signature()))
        } else {
            Err(MarketplaceError::JobNotFound(job_id))
        }
    }

    async fn get_fhe_consensus_config(&self, _job_id: u64) -> Result<Option<FheConsensusConfig>> {
        Ok(Some(FheConsensusConfig {
            required_provers: 3,
            consensus_threshold: 2,
        }))
    }

    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    fn network(&self) -> &str {
        &self.network
    }

    fn program_address(&self) -> &str {
        &self.program_address
    }

    fn prover_address(&self) -> &str {
        &self.prover_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_find_pending_jobs() {
        let mock = MockMarketplace::new();
        mock.add_pending_job(1, "creator1", 1000);
        mock.add_pending_job(2, "creator2", 2000);
        mock.add_pending_fhe_job(3, "creator3", 3000);

        let pending = mock.find_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 2); // Only non-FHE jobs

        let fhe_jobs = mock.find_fhe_jobs_needing_provers().await.unwrap();
        assert_eq!(fhe_jobs.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_claim_job() {
        let mock = MockMarketplace::new();
        mock.add_pending_job(1, "creator", 1000);

        let result = mock.claim_job(1, "creator").await.unwrap();
        assert!(result.success);

        // Job should now be claimed
        let job = mock.get_job(1).unwrap();
        assert_eq!(job.status, JobStatus::Claimed);
        assert!(job.prover.is_some());
    }

    #[tokio::test]
    async fn test_mock_claim_already_claimed() {
        let mock = MockMarketplace::new();
        mock.add_pending_job(1, "creator", 1000);

        // First claim succeeds
        mock.claim_job(1, "creator").await.unwrap();

        // Second claim fails
        let result = mock.claim_job(1, "creator").await;
        assert!(matches!(result, Err(MarketplaceError::JobAlreadyClaimed)));
    }

    #[tokio::test]
    async fn test_mock_configurable_failures() {
        let mock = MockMarketplace::new();
        mock.add_pending_job(1, "creator", 1000);
        mock.set_claims_should_fail(true);

        let result = mock.claim_job(1, "creator").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_register_prover() {
        let mock = MockMarketplace::new();
        assert!(!mock.is_registered().await.unwrap());

        let result = mock.register_prover(1_000_000_000, None).await.unwrap();
        assert!(result.success);
        assert!(mock.is_registered().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_submit_proof() {
        let mock = MockMarketplace::new();
        mock.add_pending_job(1, "creator", 1000);
        mock.claim_job(1, "creator").await.unwrap();

        let result = mock
            .submit_proof(1, "creator", [0u8; 32], 1024, None)
            .await
            .unwrap();
        assert!(result.success);

        let job = mock.get_job(1).unwrap();
        assert_eq!(job.status, JobStatus::Completed);
    }
}
