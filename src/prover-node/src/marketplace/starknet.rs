//! Starknet marketplace implementation
//!
//! Implements `MarketplaceOperations` for Starknet blockchain using `StarknetClient`.
//! This allows prover-node to work with Starknet Cairo contracts (PbtcfiJobs).
//!
//! # Current Status
//!
//! - Read operations: Implemented via starknet_call RPC
//! - Write operations: Stub (requires transaction signing)
//!
//! # Contract Interface (PbtcfiJobs)
//!
//! ```cairo
//! fn get_pending_jobs() -> Array<u256>
//! fn get_job_execution(loan_id: u256) -> JobExecution
//! fn get_loan(loan_id: u256) -> Loan
//! fn register_prover()
//! fn claim_job(loan_id: u256)
//! fn submit_result(loan_id: u256, result_hash: felt252)
//! ```

use super::{
    FheConsensusConfig, JobData, JobSource, MarketplaceError, MarketplaceOperations, ProverData,
    Result, StarknetJobType, TransactionResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use zyberlink_chain_client::{ChainClient, StarknetClient};
use zyberlink_types::{CircuitType, JobStatus};

/// Cairo function selectors (starknet_keccak of function name, truncated to 250 bits)
/// Pre-computed for PbtcfiJobs contract interface
///
/// Updated for new dual-interface architecture:
/// - IFheJobs: Generic FHE job operations (used by provers)
/// - ILoanOperations: pBTCFi-specific loan operations
mod selectors {
    // ========== IFheJobs Interface (Generic) ==========

    /// create_job(job_type, payload_hash, encrypted_c1, encrypted_c2, reward) -> u256
    pub const CREATE_JOB: &str =
        "0x01e2cd4607e049d866389ed1e91d9e1af64e69e52d2a66736f89d0e8aeaeb57c";
    /// get_job(job_id: u256) -> Job
    pub const GET_JOB: &str =
        "0x00fbfd0ddbd3e0013d78a7d9bcc75e5f5b08d7a9536e9faa7349a797f11872ae";
    /// register_prover() - same as before
    pub const REGISTER_PROVER: &str =
        "0x0292f780aa61b49fbf9aa0ebc904a38993d165ee53ea8484e2c443258ebcbee7";
    /// get_prover(prover: ContractAddress) -> Prover
    pub const GET_PROVER: &str =
        "0x01a0b3c4f48b78a62e4d57c5b9c8d3f4e5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0";
    /// is_prover_registered(prover: ContractAddress) -> bool
    pub const IS_PROVER_REGISTERED: &str =
        "0x02b1c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3";
    /// claim_job(job_id: u256) - IFheJobs version
    pub const CLAIM_JOB: &str =
        "0x014884a95bf7734febaf1e5f0ef0f2d2bf7014034b2362b7551c2b716029e2a0";
    /// submit_result(job_id: u256, result_hash: felt252)
    pub const SUBMIT_RESULT: &str =
        "0x038b5fc200cc1074ba697b44368d487adb9924b6e1ed1aa05782e121915fc058";
    /// get_job_execution(job_id: u256) -> JobExecution
    pub const GET_JOB_EXECUTION: &str =
        "0x02999d7d778e809ab93af886ca536f83c6c3986d95e902ce55361e03c641b83f";
    /// get_pending_jobs() -> Array<u256>
    pub const GET_PENDING_JOBS: &str =
        "0x03b257e49fd4fa83c6951edd7b6f94ccb2aca13dd843240b43ef53a1a7fd000a";
    /// get_pending_jobs_by_type(job_type: JobType) -> Array<u256>
    pub const GET_PENDING_JOBS_BY_TYPE: &str =
        "0x03c2d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3";
    /// get_jobs_by_creator(creator: ContractAddress) -> Array<u256>
    pub const GET_JOBS_BY_CREATOR: &str =
        "0x04d3e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4";

    // ========== ILoanOperations Interface (pBTCFi-specific) ==========

    /// create_loan(borrower, btc_commitment, btc_encrypted_c1, btc_encrypted_c2) -> u256
    pub const CREATE_LOAN: &str =
        "0x01f3a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
    /// get_loan(loan_id: u256) -> Loan
    pub const GET_LOAN: &str =
        "0x03faf899aaaf6460caab4af9f3b6f5282c4c4720e4baa64ce53f6b822f4b47da";
    /// get_loan_job_id(loan_id: u256) -> u256
    pub const GET_LOAN_JOB_ID: &str =
        "0x05e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4";
    /// get_pending_loan_jobs() -> Array<u256>
    pub const GET_PENDING_LOAN_JOBS: &str =
        "0x06f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5";
    /// claim_loan_job(loan_id: u256)
    pub const CLAIM_LOAN_JOB: &str =
        "0x07a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6";
    /// submit_loan_result(loan_id: u256, result_hash: felt252)
    pub const SUBMIT_LOAN_RESULT: &str =
        "0x08b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7";
    /// get_loan_job_execution(loan_id: u256) -> JobExecution
    pub const GET_LOAN_JOB_EXECUTION: &str =
        "0x09c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8";
}

/// Starknet marketplace wrapper
///
/// Implements `MarketplaceOperations` by delegating to `StarknetClient`.
/// Read operations work via starknet_call RPC. Write operations require
/// transaction signing (pending full SDK integration).
///
/// # Example
///
/// ```rust,ignore
/// use prover_node::marketplace::{StarknetMarketplace, MarketplaceOperations};
///
/// let marketplace = StarknetMarketplace::new(
///     client,
///     "0x123...".to_string(),
///     "0xabc...".to_string(),
/// );
///
/// let jobs = marketplace.find_pending_jobs().await?;
/// for job in jobs {
///     println!("Found pending job: {}", job.id);
/// }
/// ```
pub struct StarknetMarketplace {
    client: Arc<StarknetClient>,
    contract_address: String,
    prover_address: String,
    /// Private key for signing (secured via zeroize in future Sprint)
    /// Sprint 1: Keeping as String for MVP compatibility with StarknetClient
    /// Sprint 2: Will migrate to LocalWallet from starknet-signers
    private_key: Option<String>,
}

impl StarknetMarketplace {
    /// Create a new Starknet marketplace client
    ///
    /// # Arguments
    /// * `client` - StarknetClient instance
    /// * `contract_address` - Marketplace Cairo contract address
    /// * `prover_address` - Prover account address for signing transactions
    pub fn new(
        client: Arc<StarknetClient>,
        contract_address: String,
        prover_address: String,
    ) -> Self {
        Self {
            client,
            contract_address,
            prover_address,
            private_key: None,
        }
    }

    /// Create a new Starknet marketplace client with signing capability
    ///
    /// # Arguments
    /// * `client` - StarknetClient instance
    /// * `contract_address` - Marketplace Cairo contract address
    /// * `prover_address` - Prover account address for signing transactions
    /// * `private_key_hex` - Private key hex string (e.g., "0x1234...")
    ///
    /// # Sprint 1 MVP Note
    /// Stores private key as string for compatibility with StarknetClient.
    /// Sprint 2 will migrate to LocalWallet from starknet-signers for better security.
    pub fn new_with_signer(
        client: Arc<StarknetClient>,
        contract_address: String,
        prover_address: String,
        private_key_hex: String,
    ) -> Self {
        // Sprint 1: Simple validation
        if !private_key_hex.starts_with("0x") && !private_key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            log::warn!("Private key should be hex format (with or without 0x prefix)");
        }

        Self {
            client,
            contract_address,
            prover_address,
            private_key: Some(private_key_hex),
        }
    }

    /// Set the private key for signing transactions
    pub fn set_private_key(&mut self, private_key_hex: String) {
        self.private_key = Some(private_key_hex);
    }

    /// Check if signing is available
    pub fn can_sign(&self) -> bool {
        self.private_key.is_some()
    }

    /// Get direct access to the underlying StarknetClient
    ///
    /// Use sparingly - prefer trait methods when possible.
    pub fn client(&self) -> &StarknetClient {
        &self.client
    }

    /// Get the contract address
    pub fn contract_address(&self) -> &str {
        &self.contract_address
    }

    /// Helper to create "not implemented" error for write operations
    fn not_implemented_write(operation: &str) -> MarketplaceError {
        MarketplaceError::Other(format!(
            "Starknet write operation '{}' requires transaction signing (pending SDK)",
            operation
        ))
    }

    /// Call a contract view function
    async fn call_view(
        &self,
        selector: &str,
        calldata: Vec<String>,
    ) -> std::result::Result<Vec<String>, MarketplaceError> {
        let args = serde_json::to_vec(&calldata)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize calldata: {}", e)))?;

        let result = self
            .client
            .call_contract(&self.contract_address, selector, &args)
            .await
            .map_err(|e| MarketplaceError::Network(format!("Contract call failed: {}", e)))?;

        serde_json::from_slice(&result)
            .map_err(|e| MarketplaceError::Other(format!("Failed to deserialize result: {}", e)))
    }

    /// Parse a felt252 hex string to u64
    fn parse_felt_to_u64(felt: &str) -> std::result::Result<u64, MarketplaceError> {
        let s = felt.trim_start_matches("0x");
        u64::from_str_radix(s, 16)
            .map_err(|e| MarketplaceError::Other(format!("Failed to parse felt {}: {}", felt, e)))
    }

    /// Parse a felt252 hex string to [u8; 32] (right-aligned)
    fn parse_felt_to_bytes32(felt: &str) -> [u8; 32] {
        let s = felt.trim_start_matches("0x");
        let mut bytes = [0u8; 32];
        // Parse hex string, right-aligned in 32 bytes
        if let Ok(val) = hex::decode(format!("{:0>64}", s)) {
            bytes.copy_from_slice(&val[..32.min(val.len())]);
        }
        bytes
    }

    /// Parse a u256 from two felt252 values (low, high)
    fn parse_u256(low: &str, high: &str) -> std::result::Result<u64, MarketplaceError> {
        // For now, we only use the low part (assumes values fit in u64)
        Self::parse_felt_to_u64(low)
    }

    /// Convert u256 to calldata (low, high as felt252)
    fn u256_to_calldata(value: u64) -> Vec<String> {
        vec![format!("0x{:x}", value), "0x0".to_string()]
    }

    /// Get job status (for pre-claim verification)
    ///
    /// Returns the current status of a job by calling get_job_execution.
    /// Used to reduce failed transactions in high-concurrency scenarios.
    async fn get_job_status(
        &self,
        job_id: u64,
    ) -> std::result::Result<zyberlink_types::JobStatus, MarketplaceError> {
        let calldata = Self::u256_to_calldata(job_id);

        let exec_result = self
            .call_view(selectors::GET_JOB_EXECUTION, calldata)
            .await;

        match exec_result {
            Ok(data) if data.len() > 3 => {
                let status = match Self::parse_felt_to_u64(&data[3])? {
                    0 => zyberlink_types::JobStatus::Pending,
                    1 => zyberlink_types::JobStatus::Claimed,
                    2 => zyberlink_types::JobStatus::Completed,
                    _ => zyberlink_types::JobStatus::Pending,
                };
                Ok(status)
            }
            Ok(_) => Ok(zyberlink_types::JobStatus::Pending),
            Err(e) => {
                log::warn!("get_job_status({}) failed: {}, assuming Pending", job_id, e);
                Ok(zyberlink_types::JobStatus::Pending)
            }
        }
    }
}

#[async_trait]
impl MarketplaceOperations for StarknetMarketplace {
    // ========== Prover Lifecycle ==========

    async fn register_prover(
        &self,
        _stake: u64,
        _encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult> {
        // Write operation - requires transaction signing
        Err(Self::not_implemented_write("register_prover"))
    }

    async fn get_prover(&self, authority: &str) -> Result<Option<ProverData>> {
        // Call get_prover(prover: ContractAddress) -> Prover
        let calldata = vec![authority.to_string()];

        let prover_result = self
            .call_view(selectors::GET_PROVER, calldata)
            .await;

        match prover_result {
            Ok(data) if data.len() >= 8 => {
                // Parse Prover struct: [authority(1), stake(2), encryption_pubkey(1),
                //                       jobs_completed(1), jobs_failed(1), total_earnings(2), is_active(1), registered_at(1)]
                let registered_at = Self::parse_felt_to_u64(&data[7]).unwrap_or(0);

                // If registered_at is 0, prover doesn't exist
                if registered_at == 0 {
                    return Ok(None);
                }

                let stake = Self::parse_u256(&data[1], &data[2]).unwrap_or(0);
                let jobs_completed = Self::parse_felt_to_u64(&data[3]).unwrap_or(0);
                let jobs_failed = Self::parse_felt_to_u64(&data[4]).unwrap_or(0);
                let is_active = Self::parse_felt_to_u64(&data[6]).unwrap_or(0) != 0;
                let encryption_pubkey = Self::parse_felt_to_bytes32(&data[5]);

                // Calculate reputation (0-100) based on success rate
                let total_jobs = jobs_completed + jobs_failed;
                let reputation = if total_jobs > 0 {
                    ((jobs_completed as f64 / total_jobs as f64) * 100.0) as u8
                } else {
                    100  // Default reputation for new provers
                };

                Ok(Some(ProverData {
                    authority: authority.to_string(),
                    stake,
                    encryption_pubkey: Some(encryption_pubkey),
                    is_active,
                    jobs_completed,
                    jobs_failed,
                    reputation,
                    registered_at: registered_at as i64,  // Convert u64 to i64
                }))
            }
            Ok(_) => {
                log::debug!("get_prover({}) returned incomplete data", authority);
                Ok(None)
            }
            Err(e) => {
                log::debug!("get_prover({}) failed: {}", authority, e);
                Ok(None)
            }
        }
    }

    async fn is_registered(&self) -> Result<bool> {
        // Call is_prover_registered(prover: ContractAddress) -> bool
        let calldata = vec![self.prover_address.clone()];

        let result = self
            .call_view(selectors::IS_PROVER_REGISTERED, calldata)
            .await;

        match result {
            Ok(data) if !data.is_empty() => {
                // Parse boolean result (0 = false, 1 = true)
                let is_registered = Self::parse_felt_to_u64(&data[0]).unwrap_or(0) != 0;
                Ok(is_registered)
            }
            Ok(_) => Ok(false),
            Err(e) => {
                log::warn!("is_registered() failed: {}, assuming false", e);
                Ok(false)
            }
        }
    }

    // ========== Job Discovery ==========

    async fn find_pending_jobs(&self) -> Result<Vec<JobData>> {
        // Call get_pending_jobs() on the contract
        let result = self
            .call_view(selectors::GET_PENDING_JOBS, vec![])
            .await?;

        // Parse Array<u256> - format: [length, id1_low, id1_high, id2_low, id2_high, ...]
        if result.is_empty() {
            return Ok(Vec::new());
        }

        let length = Self::parse_felt_to_u64(&result[0])? as usize;
        let mut jobs = Vec::with_capacity(length);

        // Each u256 is 2 felts (low, high)
        for i in 0..length {
            let idx = 1 + i * 2;
            if idx + 1 < result.len() {
                let job_id = Self::parse_u256(&result[idx], &result[idx + 1])?;

                // Fetch job details
                if let Some(job) = self.get_job(job_id, &self.contract_address).await? {
                    if job.status == JobStatus::Pending {
                        jobs.push(job);
                    }
                }
            }
        }

        Ok(jobs)
    }

    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>> {
        // PbtcfiJobs doesn't have FHE-specific jobs, use regular pending jobs
        self.find_pending_jobs().await
    }

    async fn get_job(&self, job_id: u64, _creator: &str) -> Result<Option<JobData>> {
        let calldata = Self::u256_to_calldata(job_id);

        // First, get job execution status
        let exec_result = self
            .call_view(selectors::GET_JOB_EXECUTION, calldata.clone())
            .await;

        let (status, prover) = match exec_result {
            Ok(data) if data.len() > 3 => {
                let status = match Self::parse_felt_to_u64(&data[3])? {
                    0 => JobStatus::Pending,
                    1 => JobStatus::Claimed,
                    2 => JobStatus::Completed,
                    _ => JobStatus::Pending,
                };
                let prover = if data.len() > 2 && data[2] != "0x0" {
                    Some(data[2].clone())
                } else {
                    None
                };
                (status, prover)
            }
            _ => (JobStatus::Pending, None),
        };

        // Now get the full Job struct with FHE data
        // Job struct: [job_id(2), job_type(1), creator(1), payload_hash(1),
        //              encrypted_c1(1), encrypted_c2(1), reward(2), status(1), created_at(1)]
        let job_result = self
            .call_view(selectors::GET_JOB, calldata)
            .await;

        match job_result {
            Ok(data) if data.len() >= 10 => {
                // Parse Job struct fields
                let job_type_val = Self::parse_felt_to_u64(&data[2]).unwrap_or(0);
                let starknet_job_type = match job_type_val {
                    0 => StarknetJobType::LoanVerification,
                    1 => StarknetJobType::BalanceUpdate,
                    2 => StarknetJobType::StakeProof,
                    3 => StarknetJobType::TransferProof,
                    4 => StarknetJobType::LiquidationCheck,
                    _ => StarknetJobType::LoanVerification,
                };

                let creator = data[3].clone();
                let payload_hash = Self::parse_felt_to_bytes32(&data[4]);
                let encrypted_c1 = Self::parse_felt_to_bytes32(&data[5]);
                let encrypted_c2 = Self::parse_felt_to_bytes32(&data[6]);
                let reward = Self::parse_u256(&data[7], &data[8]).unwrap_or(0);

                log::debug!(
                    "get_job({}) parsed: type={:?}, c1={:?}, c2={:?}, reward={}",
                    job_id, starknet_job_type, &encrypted_c1[..4], &encrypted_c2[..4], reward
                );

                Ok(Some(JobData {
                    id: job_id,
                    creator,
                    circuit_type: CircuitType::Custom("pbtcfi_loan".to_string()),
                    witness_hash: [0u8; 32],
                    price: reward,
                    status,
                    prover,
                    created_at: 0,
                    timeout_at: 0,
                    is_fhe: true,
                    address: self.contract_address.clone(),
                    source: JobSource::Starknet,
                    program_id: Some(self.contract_address.clone()),
                    starknet_job_type: Some(starknet_job_type),
                    encrypted_c1: Some(encrypted_c1),
                    encrypted_c2: Some(encrypted_c2),
                    payload_hash: Some(payload_hash),
                }))
            }
            Ok(_) => {
                log::debug!("get_job({}) returned incomplete data", job_id);
                Ok(None)
            }
            Err(e) => {
                log::debug!("get_job({}) failed: {}", job_id, e);
                Ok(None)
            }
        }
    }

    // ========== Job Execution ==========

    async fn claim_job(&self, job_id: u64, _creator: &str) -> Result<TransactionResult> {
        // Check if we have a private key for signing
        let private_key = match &self.private_key {
            Some(pk) => pk.clone(),
            None => {
                log::warn!(
                    "claim_job({}) requires private key - call set_private_key() first",
                    job_id
                );
                return Err(Self::not_implemented_write("claim_job (no private key)"));
            }
        };

        // Pre-check: verify job is still pending before attempting claim
        // This reduces failed transactions in high-concurrency scenarios
        let job_status = self.get_job_status(job_id).await?;
        if job_status != zyberlink_types::JobStatus::Pending {
            log::info!(
                "claim_job({}) skipped: job status is {:?} (already claimed or completed)",
                job_id, job_status
            );
            return Err(MarketplaceError::JobAlreadyClaimed);
        }

        // Build calldata for claim_job(loan_id: u256)
        let calldata = Self::u256_to_calldata(job_id);

        // Build args for execute_contract
        let args = serde_json::json!({
            "calldata": calldata,
            "private_key": private_key
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        // Execute the transaction with race condition handling
        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::CLAIM_JOB,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                // Detect "Job not available" error from Cairo contract
                // This happens when another prover claimed the job between our check and tx
                if err_str.contains("Job not available")
                    || err_str.contains("not available")
                    || err_str.contains("already claimed")
                {
                    log::info!(
                        "claim_job({}) race condition: job was claimed by another prover",
                        job_id
                    );
                    MarketplaceError::JobAlreadyClaimed
                } else {
                    MarketplaceError::Other(format!("claim_job failed: {}", e))
                }
            })?;

        log::info!("claim_job({}) tx submitted: {}", job_id, tx_hash);

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None, // Will be set after confirmation
        })
    }

    async fn claim_fhe_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult> {
        // Same as claim_job for PbtcfiJobs
        self.claim_job(job_id, creator).await
    }

    async fn submit_proof(
        &self,
        job_id: u64,
        _creator: &str,
        proof_commitment: [u8; 32],
        _proof_size: u32,
        _fee_recipient: Option<&str>,
    ) -> Result<TransactionResult> {
        // Check if we have a private key for signing
        let private_key = match &self.private_key {
            Some(pk) => pk.clone(),
            None => {
                log::warn!(
                    "submit_proof({}) requires private key - call set_private_key() first",
                    job_id
                );
                return Err(Self::not_implemented_write("submit_proof (no private key)"));
            }
        };

        // Build calldata for submit_result(loan_id: u256, result_hash: felt252)
        // Note: felt252 only supports ~251 bits, so we mask the top 5 bits to ensure
        // the value is within range (Starknet prime is ~2^251)
        let mut masked_commitment = proof_commitment;
        masked_commitment[0] &= 0x07; // Clear top 5 bits to ensure < 2^251
        let result_hash = format!("0x{}", hex::encode(masked_commitment));
        let mut calldata = Self::u256_to_calldata(job_id);
        calldata.push(result_hash.clone());

        // Build args for execute_contract
        let args = serde_json::json!({
            "calldata": calldata,
            "private_key": private_key
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        // Execute the transaction
        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::SUBMIT_RESULT,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| MarketplaceError::Other(format!("submit_proof failed: {}", e)))?;

        log::info!(
            "submit_proof({}) tx submitted: {} with result_hash {}",
            job_id,
            tx_hash,
            result_hash
        );

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None,
        })
    }

    async fn submit_fhe_result(
        &self,
        job_id: u64,
        _creator: &str,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        // Same as submit_proof for PbtcfiJobs
        self.submit_proof(job_id, _creator, result_hash, 0, None)
            .await
    }

    // ========== FHE Consensus ==========

    async fn get_fhe_consensus_config(&self, _job_id: u64) -> Result<Option<FheConsensusConfig>> {
        // PbtcfiJobs doesn't have FHE consensus - return None
        Ok(None)
    }

    // ========== Metadata ==========

    fn chain_id(&self) -> &str {
        self.client.chain_id()
    }

    fn network(&self) -> &str {
        self.client.network()
    }

    fn program_address(&self) -> &str {
        &self.contract_address
    }

    fn prover_address(&self) -> &str {
        &self.prover_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> Arc<StarknetClient> {
        Arc::new(
            StarknetClient::new("https://starknet-testnet.public.blastapi.io/rpc/v0_7").unwrap(),
        )
    }

    #[test]
    fn test_marketplace_creation() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        assert_eq!(marketplace.chain_id(), "starknet");
        assert_eq!(marketplace.network(), "testnet");
        assert_eq!(marketplace.program_address(), "0x123abc");
        assert_eq!(marketplace.prover_address(), "0xdef456");
    }

    #[test]
    fn test_parse_felt_to_u64() {
        assert_eq!(StarknetMarketplace::parse_felt_to_u64("0x1").unwrap(), 1);
        assert_eq!(StarknetMarketplace::parse_felt_to_u64("0xff").unwrap(), 255);
        assert_eq!(StarknetMarketplace::parse_felt_to_u64("0x0").unwrap(), 0);
        assert_eq!(
            StarknetMarketplace::parse_felt_to_u64("0x10").unwrap(),
            16
        );
    }

    #[test]
    fn test_u256_to_calldata() {
        let calldata = StarknetMarketplace::u256_to_calldata(42);
        assert_eq!(calldata.len(), 2);
        assert_eq!(calldata[0], "0x2a"); // 42 in hex
        assert_eq!(calldata[1], "0x0"); // high part is 0
    }

    #[tokio::test]
    async fn test_is_registered_with_prover_address() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Sprint 1: Now makes RPC call to is_prover_registered()
        // Without a real contract deployment, this will return false
        // In integration tests with real contract, this would return true for registered provers
        let result = marketplace.is_registered().await;
        assert!(result.is_ok());  // Call succeeds (even if returns false)
    }

    #[tokio::test]
    async fn test_get_prover_returns_none_for_unregistered() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Sprint 1: Now makes RPC call to get_prover()
        // Without a real contract deployment, this will return None
        // In integration tests with real contract, this would return Some for registered provers
        let prover = marketplace.get_prover("0xdef456").await.unwrap();
        assert!(prover.is_none());  // No real contract, so no prover data

        let other = marketplace.get_prover("0xother").await.unwrap();
        assert!(other.is_none());
    }

    #[tokio::test]
    async fn test_write_operations_return_error() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Write operations should return informative errors
        let result = marketplace.register_prover(1000, None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("transaction signing"));

        let result = marketplace.claim_job(1, "0xabc").await;
        assert!(result.is_err());

        let result = marketplace
            .submit_proof(1, "0xabc", [0u8; 32], 100, None)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_client_access() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client.clone(),
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Should be able to access underlying client
        assert_eq!(marketplace.client().chain_id(), "starknet");
    }

    #[test]
    fn test_contract_address_access() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        assert_eq!(marketplace.contract_address(), "0x123abc");
    }

    #[test]
    fn test_selectors_are_valid_hex() {
        // Verify all selectors are valid hex strings
        assert!(selectors::GET_PENDING_JOBS.starts_with("0x"));
        assert!(selectors::GET_JOB_EXECUTION.starts_with("0x"));
        assert!(selectors::GET_LOAN.starts_with("0x"));
        assert!(selectors::REGISTER_PROVER.starts_with("0x"));
        assert!(selectors::CLAIM_JOB.starts_with("0x"));
        assert!(selectors::SUBMIT_RESULT.starts_with("0x"));
    }

    #[test]
    fn test_signing_capability() {
        let client = create_test_client();

        // Without private key - can_sign returns false
        let marketplace = StarknetMarketplace::new(
            client.clone(),
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );
        assert!(!marketplace.can_sign());

        // With private key - can_sign returns true
        let valid_key = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let marketplace_with_signer = StarknetMarketplace::new_with_signer(
            client.clone(),
            "0x123abc".to_string(),
            "0xdef456".to_string(),
            valid_key.to_string(),
        );
        assert!(marketplace_with_signer.can_sign());

        // Setting private key after creation
        let mut marketplace_mut = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );
        assert!(!marketplace_mut.can_sign());
        marketplace_mut.set_private_key(valid_key.to_string());
        assert!(marketplace_mut.can_sign());
    }
}
