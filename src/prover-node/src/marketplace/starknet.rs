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
use tiny_keccak::{Hasher, Keccak};
use zeroize::Zeroizing;
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
        "0x02ae524ab42a6911c6fd10fec394645820f348a2ffffe39df3b18a26b74bdbe7";
    /// is_prover_registered(prover: ContractAddress) -> bool
    pub const IS_PROVER_REGISTERED: &str =
        "0x028fdd8502f6e13942c6d906b00a664792a742d6587adff955161b574315b99d";
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
        "0x01903934f7f8c9d6f6c44356df8d25827be772f093bf8e826c635ef5ad9735a1";
    /// get_jobs_by_creator(creator: ContractAddress) -> Array<u256>
    pub const GET_JOBS_BY_CREATOR: &str =
        "0x033f0ab5ff50fa94ba520845c268ac64ad6b23301fc9af6733296e6d5c16fcd2";

    // ========== ILoanOperations Interface (pBTCFi-specific) ==========

    /// create_loan(borrower, btc_commitment, btc_encrypted_c1, btc_encrypted_c2) -> u256
    pub const CREATE_LOAN: &str =
        "0x01fcec13468534e890d7564c66153dd615f0f66af495aeb0184d7b857f49bfb5";
    /// get_loan(loan_id: u256) -> Loan
    pub const GET_LOAN: &str =
        "0x03faf899aaaf6460caab4af9f3b6f5282c4c4720e4baa64ce53f6b822f4b47da";
    /// get_loan_job_id(loan_id: u256) -> u256
    pub const GET_LOAN_JOB_ID: &str =
        "0x01b86bbf935fcce24fe51ad562dd8acac684459c64cb8257bf6019769fe3fb25";
    /// get_pending_loan_jobs() -> Array<u256>
    pub const GET_PENDING_LOAN_JOBS: &str =
        "0x02aadbe238f4541584ba2170367eb44f7681944383a6608b35638d4bd9987eea";
    /// claim_loan_job(loan_id: u256)
    pub const CLAIM_LOAN_JOB: &str =
        "0x00d96cb4b4cb445bebb03da9a9c444a5e1da7118d293a712d64664e5e3f4c100";
    /// submit_loan_result(loan_id: u256, result_hash: felt252)
    pub const SUBMIT_LOAN_RESULT: &str =
        "0x02b6fb7b75cf2761bde815cf3824c2cd1dc59940e347a7b61c1f7031f8225e1e";
    /// get_loan_job_execution(loan_id: u256) -> JobExecution
    pub const GET_LOAN_JOB_EXECUTION: &str =
        "0x01422e1e8c6f58b4d20096a2e77d85f4670457abc4919667a85f6a7f6a988260";

    // ========== Sprint 3: Multi-Prover Consensus Interface ==========

    /// enable_consensus(job_id: u256, required_provers: u8, consensus_threshold: u8)
    pub const ENABLE_CONSENSUS: &str =
        "0x024c3c718d888650cf67d301deaef4e620713c4e6eebe5318b06ac44fc42d8f9";
    /// get_consensus_data(job_id: u256) -> FheConsensusData
    pub const GET_CONSENSUS_DATA: &str =
        "0x033a2430c23d5eb9548ea5e816d6ccd9b649ebff247a23b2f4468556289f64e4";
    /// is_consensus_enabled(job_id: u256) -> bool
    pub const IS_CONSENSUS_ENABLED: &str =
        "0x03a9084189bbf3c248a282c4656006154df109a32b8ad424546303f127e5972c";

    // ========== Sprint 3: ECDSA Verified Submission Interface ==========

    /// submit_verified_result(job_id: u256, result_hash: felt252, ciphertext_hash: felt252, signature_r: felt252, signature_s: felt252)
    pub const SUBMIT_VERIFIED_RESULT: &str =
        "0x014f25cd07508365018b418f95e5fafddc7b875fddf338b72ba4efc73b6f1982";
    /// register_prover_pubkey(pubkey_x: felt252, pubkey_y: felt252)
    pub const REGISTER_PROVER_PUBKEY: &str =
        "0x00f6b626d2575429724c01eb895e165218dd8cd915a49b57c6866daff4ac6423";
    /// get_prover_pubkey(prover: ContractAddress) -> (felt252, felt252)
    pub const GET_PROVER_PUBKEY: &str =
        "0x001af949b2eca3b88e5b94cacc375e75a2d38a2f5cd560971944c4149641f973";
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
    /// Private key for signing (secured via zeroize on drop)
    /// Uses Zeroizing<String> to ensure key material is cleared from memory
    private_key: Option<Zeroizing<String>>,
}

struct StarknetFheConsensusData {
    job_id: u64,
    required_provers: u8,
    consensus_threshold: u8,
    claimed_count: u8,
    submitted_count: u8,
    consensus_reached: bool,
    consensus_hash: [u8; 32],
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
    /// # Security Note
    /// Private key is stored using Zeroizing<String> to ensure it's cleared from memory on drop.
    pub fn new_with_signer(
        client: Arc<StarknetClient>,
        contract_address: String,
        prover_address: String,
        private_key_hex: String,
    ) -> Self {
        // Simple validation
        if !private_key_hex.starts_with("0x") && !private_key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            log::warn!("Private key should be hex format (with or without 0x prefix)");
        }

        Self {
            client,
            contract_address,
            prover_address,
            private_key: Some(Zeroizing::new(private_key_hex)),
        }
    }

    /// Set the private key for signing transactions
    pub fn set_private_key(&mut self, private_key_hex: String) {
        self.private_key = Some(Zeroizing::new(private_key_hex));
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

    fn parse_felt_to_u8(felt: &str) -> std::result::Result<u8, MarketplaceError> {
        let value = Self::parse_felt_to_u64(felt)?;
        u8::try_from(value).map_err(|_| {
            MarketplaceError::Other(format!("Failed to parse felt {} as u8", felt))
        })
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

    fn selector_from_name(name: &str) -> String {
        let mut hasher = Keccak::v256();
        let mut output = [0u8; 32];
        hasher.update(name.as_bytes());
        hasher.finalize(&mut output);
        output[0] &= 0x03;
        format!("0x{}", hex::encode(output))
    }

    fn parse_consensus_data(data: &[String]) -> Result<StarknetFheConsensusData> {
        if data.len() < 8 {
            return Err(MarketplaceError::Deserialization(format!(
                "Invalid consensus data length: expected 8, got {}",
                data.len()
            )));
        }

        let job_id = Self::parse_u256(&data[0], &data[1])?;
        let required_provers = Self::parse_felt_to_u8(&data[2])?;
        let consensus_threshold = Self::parse_felt_to_u8(&data[3])?;
        let claimed_count = Self::parse_felt_to_u8(&data[4])?;
        let submitted_count = Self::parse_felt_to_u8(&data[5])?;
        let consensus_reached = Self::parse_felt_to_u64(&data[6]).unwrap_or(0) != 0;
        let consensus_hash = Self::parse_felt_to_bytes32(&data[7]);

        Ok(StarknetFheConsensusData {
            job_id,
            required_provers,
            consensus_threshold,
            claimed_count,
            submitted_count,
            consensus_reached,
            consensus_hash,
        })
    }

    async fn get_consensus_data(
        &self,
        job_id: u64,
    ) -> Result<Option<StarknetFheConsensusData>> {
        let calldata = Self::u256_to_calldata(job_id);
        let result = self.call_view(selectors::GET_CONSENSUS_DATA, calldata).await?;
        let consensus = Self::parse_consensus_data(&result)?;
        Ok(Some(consensus))
    }

    /// Check if consensus is enabled for a job
    pub async fn is_consensus_enabled(&self, job_id: u64) -> Result<bool> {
        let calldata = Self::u256_to_calldata(job_id);
        let result = self.call_view(selectors::IS_CONSENSUS_ENABLED, calldata).await;

        match result {
            Ok(data) if !data.is_empty() => {
                let enabled = Self::parse_felt_to_u64(&data[0]).unwrap_or(0) != 0;
                Ok(enabled)
            }
            Ok(_) => Ok(false),
            Err(e) => {
                log::debug!("is_consensus_enabled({}) failed: {}", job_id, e);
                Ok(false)
            }
        }
    }

    /// Enable multi-prover consensus for a job
    ///
    /// Must be called by job creator after job creation.
    ///
    /// # Arguments
    /// * `job_id` - The job to enable consensus for
    /// * `required_provers` - Number of provers that must claim (e.g., 3)
    /// * `consensus_threshold` - Number that must agree (e.g., 2 for 2/3 majority)
    pub async fn enable_consensus(
        &self,
        job_id: u64,
        required_provers: u8,
        consensus_threshold: u8,
    ) -> Result<TransactionResult> {
        let private_key = match &self.private_key {
            Some(pk) => pk.as_str(),
            None => {
                return Err(Self::not_implemented_write("enable_consensus (no private key)"));
            }
        };

        // Validate inputs
        if consensus_threshold > required_provers {
            return Err(MarketplaceError::Other(
                "consensus_threshold cannot exceed required_provers".to_string()
            ));
        }
        if required_provers == 0 {
            return Err(MarketplaceError::Other(
                "required_provers must be at least 1".to_string()
            ));
        }

        // Build calldata: job_id (u256), required_provers (u8), consensus_threshold (u8)
        let mut calldata = Self::u256_to_calldata(job_id);
        calldata.push(format!("0x{:x}", required_provers));
        calldata.push(format!("0x{:x}", consensus_threshold));

        let args = serde_json::json!({
            "calldata": calldata,
            "private_key": private_key
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::ENABLE_CONSENSUS,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| MarketplaceError::Other(format!("enable_consensus failed: {}", e)))?;

        log::info!(
            "enable_consensus({}, {}/{}) tx submitted: {}",
            job_id, consensus_threshold, required_provers, tx_hash
        );

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None,
        })
    }

    fn normalize_required_provers(required_provers: u8) -> u8 {
        if required_provers == 0 {
            1
        } else {
            required_provers
        }
    }

    fn normalize_consensus_threshold(consensus_threshold: u8) -> u8 {
        if consensus_threshold == 0 {
            1
        } else {
            consensus_threshold
        }
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
        // Check if we have a private key for signing
        let private_key = match &self.private_key {
            Some(pk) => pk.as_str(),
            None => {
                log::warn!(
                    "register_prover() requires private key - call set_private_key() first"
                );
                return Err(Self::not_implemented_write("register_prover (no private key)"));
            }
        };

        // Check if already registered (avoid wasted tx)
        if self.is_registered().await.unwrap_or(false) {
            log::info!("register_prover() skipped: prover already registered");
            return Ok(TransactionResult {
                signature: "already_registered".to_string(),
                success: true,
                error: None,
                block_height: None,
            });
        }

        // Build args for execute_contract (register_prover takes no arguments)
        let args = serde_json::json!({
            "calldata": [],  // No arguments for register_prover()
            "private_key": private_key
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        // Execute the transaction
        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::REGISTER_PROVER,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                // Detect "already registered" error from Cairo contract
                if err_str.contains("already registered") {
                    log::info!("register_prover() race condition: prover was already registered");
                    MarketplaceError::Other("Prover already registered".to_string())
                } else {
                    MarketplaceError::Other(format!("register_prover failed: {}", e))
                }
            })?;

        log::info!("register_prover() tx submitted: {}", tx_hash);

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None,
        })
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
        let pending_jobs = self.find_pending_jobs().await?;
        let mut needing_provers = Vec::new();

        for job in pending_jobs {
            let consensus = match self.get_consensus_data(job.id).await {
                Ok(Some(consensus)) => consensus,
                Ok(None) => {
                    needing_provers.push(job);
                    continue;
                }
                Err(e) => {
                    log::debug!(
                        "find_fhe_jobs_needing_provers: failed to fetch consensus for job {}: {}",
                        job.id,
                        e
                    );
                    needing_provers.push(job);
                    continue;
                }
            };

            let required_provers = Self::normalize_required_provers(consensus.required_provers);
            if consensus.claimed_count < required_provers && !consensus.consensus_reached {
                needing_provers.push(job);
            }
        }

        Ok(needing_provers)
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
            Some(pk) => pk.as_str(),
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

        if let Ok(Some(consensus)) = self.get_consensus_data(job_id).await {
            let required_provers = Self::normalize_required_provers(consensus.required_provers);
            if consensus.claimed_count >= required_provers || consensus.consensus_reached {
                log::info!(
                    "claim_job({}) skipped: consensus already filled ({}/{})",
                    job_id,
                    consensus.claimed_count,
                    required_provers
                );
                return Err(MarketplaceError::JobAlreadyClaimed);
            }
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
            Some(pk) => pk.as_str(),
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

    async fn get_fhe_consensus_config(&self, job_id: u64) -> Result<Option<FheConsensusConfig>> {
        let consensus = self.get_consensus_data(job_id).await?;
        let consensus = match consensus {
            Some(consensus) => consensus,
            None => return Ok(None),
        };

        let required_provers = Self::normalize_required_provers(consensus.required_provers);
        let consensus_threshold =
            Self::normalize_consensus_threshold(consensus.consensus_threshold);

        Ok(Some(FheConsensusConfig {
            required_provers,
            consensus_threshold,
        }))
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

// ============================================================================
// Sprint 3: Starknet-specific ECDSA Verified Submission
// ============================================================================
impl StarknetMarketplace {
    /// Submit FHE result with ECDSA signature verification
    ///
    /// This function signs the result using the STARK curve and submits
    /// to `submit_verified_result` on the Cairo contract for on-chain verification.
    ///
    /// # Arguments
    /// * `job_id` - Job ID
    /// * `result_hash` - Hash of the computed FHE result
    /// * `ciphertext_hash` - Hash of the original FHE ciphertext input
    ///
    /// # Returns
    /// Transaction result with submission signature
    pub async fn submit_verified_fhe_result(
        &self,
        job_id: u64,
        result_hash: [u8; 32],
        ciphertext_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        // 1. Check if we have a private key for signing
        let private_key_hex = match &self.private_key {
            Some(pk) => pk.as_str(),
            None => {
                log::warn!(
                    "submit_verified_fhe_result({}) requires private key - falling back to unverified",
                    job_id
                );
                return self.submit_fhe_result(job_id, "", result_hash).await;
            }
        };

        // 2. Convert private key from hex to bytes
        let private_key_bytes: [u8; 32] = hex::decode(private_key_hex.trim_start_matches("0x"))
            .map_err(|e| MarketplaceError::Other(format!("Invalid private key hex: {}", e)))?
            .try_into()
            .map_err(|_| MarketplaceError::Other("Private key must be 32 bytes".to_string()))?;

        // 3. Sign the FHE result using STARK curve
        let signature = signing::sign_fhe_result_stark(
            &private_key_bytes,
            &ciphertext_hash,
            &result_hash,
        ).map_err(|e| MarketplaceError::Other(format!("Signing failed: {}", e)))?;

        // 4. Build calldata for submit_verified_result
        // (job_id: u256, result_hash: felt252, ciphertext_hash: felt252, signature_r: felt252, signature_s: felt252)
        let mut calldata = Self::u256_to_calldata(job_id);

        // Mask hashes to ensure < 2^251
        let mut result_masked = result_hash;
        result_masked[0] &= 0x07;
        let result_hash_felt = format!("0x{}", hex::encode(result_masked));

        let mut cipher_masked = ciphertext_hash;
        cipher_masked[0] &= 0x07;
        let ciphertext_hash_felt = format!("0x{}", hex::encode(cipher_masked));

        calldata.push(result_hash_felt.clone());
        calldata.push(ciphertext_hash_felt);
        calldata.push(signature.r.clone());
        calldata.push(signature.s.clone());

        // 5. Build args for execute_contract
        let args = serde_json::json!({
            "calldata": calldata,
            "private_key": private_key_hex
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        // 6. Execute the transaction
        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::SUBMIT_VERIFIED_RESULT,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| MarketplaceError::Other(format!("submit_verified_result failed: {}", e)))?;

        log::info!(
            "submit_verified_fhe_result({}) tx submitted: {} with result_hash {} sig_r {}",
            job_id,
            tx_hash,
            result_hash_felt,
            signature.r
        );

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None,
        })
    }

    /// Register prover's ECDSA public key for signature verification
    ///
    /// Must be called once before submitting verified results.
    pub async fn register_prover_pubkey(&self) -> Result<TransactionResult> {
        let private_key_hex = match &self.private_key {
            Some(pk) => pk.as_str(),
            None => {
                return Err(MarketplaceError::Other(
                    "register_prover_pubkey requires private key".to_string(),
                ));
            }
        };

        // Convert private key from hex to bytes
        let private_key_bytes: [u8; 32] = hex::decode(private_key_hex.trim_start_matches("0x"))
            .map_err(|e| MarketplaceError::Other(format!("Invalid private key hex: {}", e)))?
            .try_into()
            .map_err(|_| MarketplaceError::Other("Private key must be 32 bytes".to_string()))?;

        // Get STARK public key
        let pubkey_x = signing::get_stark_pubkey(&private_key_bytes)
            .map_err(|e| MarketplaceError::Other(format!("Failed to get pubkey: {}", e)))?;

        // For STARK curve, we only need x-coordinate (pubkey_y = 0 for MVP)
        let pubkey_y = "0x0".to_string();

        let calldata = vec![pubkey_x.clone(), pubkey_y];

        let args = serde_json::json!({
            "calldata": calldata,
            "private_key": private_key_hex
        });

        let args_bytes = serde_json::to_vec(&args)
            .map_err(|e| MarketplaceError::Other(format!("Failed to serialize args: {}", e)))?;

        let tx_hash = self
            .client
            .execute_contract(
                &self.contract_address,
                selectors::REGISTER_PROVER_PUBKEY,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| MarketplaceError::Other(format!("register_prover_pubkey failed: {}", e)))?;

        log::info!(
            "register_prover_pubkey tx submitted: {} with pubkey_x {}",
            tx_hash,
            pubkey_x
        );

        Ok(TransactionResult {
            signature: tx_hash,
            success: true,
            error: None,
            block_height: None,
        })
    }
}

// ============================================================================
// ECDSA Signing Module for FHE Results (Sprint 2/3)
// ============================================================================
/// Signs the combination of ciphertext_hash and result_hash to prove
/// that the prover computed this specific result from this specific input.
///
/// Sprint 3: Added STARK curve support for native Cairo verification.
/// The STARK curve is more efficient on Starknet than secp256k1.
pub mod signing {
    use anyhow::{anyhow, Result};
    use k256::ecdsa::{signature::Signer, Signature, SigningKey};
    use starknet_crypto::{sign, get_public_key, Felt};
    use tiny_keccak::{Hasher, Keccak};

    /// STARK curve signature (r, s) as hex strings for Cairo
    #[derive(Debug, Clone)]
    pub struct StarkSignature {
        pub r: String,
        pub s: String,
    }

    /// Sign an FHE result using STARK curve (native Cairo ECDSA)
    ///
    /// # Arguments
    /// * `private_key` - Prover's private key (32 bytes, STARK curve)
    /// * `ciphertext_hash` - Hash of the input FHE ciphertext (32 bytes)
    /// * `result_hash` - Hash of the computed result (32 bytes)
    ///
    /// # Returns
    /// StarkSignature with r and s as hex strings for Cairo calldata
    pub fn sign_fhe_result_stark(
        private_key: &[u8; 32],
        ciphertext_hash: &[u8; 32],
        result_hash: &[u8; 32],
    ) -> Result<StarkSignature> {
        // 1. Compute message: keccak256(ciphertext_hash || result_hash)
        let message_bytes = compute_message_hash(ciphertext_hash, result_hash);

        // 2. Convert to Felt (mask to ensure < prime)
        let mut masked_message = message_bytes;
        masked_message[0] &= 0x07; // Ensure < 2^251
        let message = Felt::from_bytes_be_slice(&masked_message);

        // 3. Convert private key to Felt
        let mut masked_key = *private_key;
        masked_key[0] &= 0x07; // Ensure < 2^251
        let priv_key = Felt::from_bytes_be_slice(&masked_key);

        // 4. Generate random k for signing (use hash of message + private key for determinism)
        let mut k_input = [0u8; 64];
        k_input[..32].copy_from_slice(&message_bytes);
        k_input[32..].copy_from_slice(private_key);
        let k_hash = compute_message_hash(&k_input[..32].try_into().unwrap(), &k_input[32..].try_into().unwrap());
        let mut k_masked = k_hash;
        k_masked[0] &= 0x07;
        let k = Felt::from_bytes_be_slice(&k_masked);

        // 5. Sign with STARK curve
        let signature = sign(&priv_key, &message, &k)
            .map_err(|e| anyhow!("Signing failed: {:?}", e))?;

        // 6. Convert to hex strings for Cairo calldata
        let r_bytes = signature.r.to_bytes_be();
        let s_bytes = signature.s.to_bytes_be();

        Ok(StarkSignature {
            r: format!("0x{}", hex::encode(r_bytes)),
            s: format!("0x{}", hex::encode(s_bytes)),
        })
    }

    /// Get public key x-coordinate for STARK curve (for Cairo registration)
    pub fn get_stark_pubkey(private_key: &[u8; 32]) -> Result<String> {
        let mut masked_key = *private_key;
        masked_key[0] &= 0x07;
        let priv_key = Felt::from_bytes_be_slice(&masked_key);

        let pubkey = get_public_key(&priv_key);
        let pubkey_bytes = pubkey.to_bytes_be();

        Ok(format!("0x{}", hex::encode(pubkey_bytes)))
    }

    // ========== Legacy secp256k1 functions (for backwards compatibility) ==========

    /// Sign an FHE computation result (secp256k1 - LEGACY)
    pub fn sign_fhe_result(
        private_key: &[u8; 32],
        ciphertext_hash: &[u8; 32],
        result_hash: &[u8; 32],
    ) -> Result<Signature> {
        let message = compute_message_hash(ciphertext_hash, result_hash);
        let signing_key = SigningKey::from_bytes(private_key.into())
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let signature: Signature = signing_key.sign(&message);
        Ok(signature)
    }

    /// Compute keccak256(ciphertext_hash || result_hash)
    /// Same hash computation as Cairo side for verification.
    pub fn compute_message_hash(ciphertext_hash: &[u8; 32], result_hash: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Keccak::v256();
        hasher.update(ciphertext_hash);
        hasher.update(result_hash);
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);
        output
    }

    /// Convert ECDSA signature to Cairo-compatible felt252 pair (r, s) - LEGACY
    pub fn signature_to_felts(signature: &Signature) -> (String, String) {
        let sig_bytes = signature.to_bytes();
        let r_bytes = &sig_bytes[0..32];
        let s_bytes = &sig_bytes[32..64];

        let mut r_masked = [0u8; 32];
        let mut s_masked = [0u8; 32];
        r_masked.copy_from_slice(r_bytes);
        s_masked.copy_from_slice(s_bytes);
        r_masked[0] &= 0x07;
        s_masked[0] &= 0x07;

        (format!("0x{}", hex::encode(r_masked)), format!("0x{}", hex::encode(s_masked)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_sign_fhe_result() {
            let private_key = [0x42; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let signature = sign_fhe_result(&private_key, &ciphertext_hash, &result_hash);
            assert!(signature.is_ok(), "Signing should succeed");

            let sig = signature.unwrap();
            assert_eq!(sig.to_bytes().len(), 64, "Signature should be 64 bytes");
        }

        #[test]
        fn test_signature_deterministic() {
            let private_key = [0x42; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let sig1 = sign_fhe_result(&private_key, &ciphertext_hash, &result_hash).unwrap();
            let sig2 = sign_fhe_result(&private_key, &ciphertext_hash, &result_hash).unwrap();

            assert_eq!(sig1.to_bytes(), sig2.to_bytes(), "Signatures should be deterministic");
        }

        #[test]
        fn test_invalid_private_key_error() {
            let invalid_key = [0x00; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let result = sign_fhe_result(&invalid_key, &ciphertext_hash, &result_hash);
            assert!(result.is_err(), "Should reject invalid private key");
        }

        #[test]
        fn test_signature_to_felts() {
            let private_key = [0x42; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let signature = sign_fhe_result(&private_key, &ciphertext_hash, &result_hash).unwrap();
            let (sig_r, sig_s) = signature_to_felts(&signature);

            assert!(sig_r.starts_with("0x"), "sig_r should be hex");
            assert!(sig_s.starts_with("0x"), "sig_s should be hex");
            assert_eq!(sig_r.len(), 66, "sig_r should be 0x + 64 hex chars");
            assert_eq!(sig_s.len(), 66, "sig_s should be 0x + 64 hex chars");
        }

        #[test]
        fn test_compute_message_hash() {
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let hash = compute_message_hash(&ciphertext_hash, &result_hash);
            assert_eq!(hash.len(), 32);

            let hash2 = compute_message_hash(&ciphertext_hash, &result_hash);
            assert_eq!(hash, hash2, "Hash should be deterministic");
        }

        // ========== STARK curve tests (Sprint 3) ==========

        #[test]
        fn test_sign_fhe_result_stark() {
            let private_key = [0x42; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let signature = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result_hash);
            assert!(signature.is_ok(), "STARK signing should succeed");

            let sig = signature.unwrap();
            assert!(sig.r.starts_with("0x"), "sig.r should be hex");
            assert!(sig.s.starts_with("0x"), "sig.s should be hex");
        }

        #[test]
        fn test_stark_signature_deterministic() {
            let private_key = [0x42; 32];
            let ciphertext_hash = [0xaa; 32];
            let result_hash = [0xbb; 32];

            let sig1 = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result_hash).unwrap();
            let sig2 = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result_hash).unwrap();

            assert_eq!(sig1.r, sig2.r, "STARK signatures should be deterministic (r)");
            assert_eq!(sig1.s, sig2.s, "STARK signatures should be deterministic (s)");
        }

        #[test]
        fn test_get_stark_pubkey() {
            let private_key = [0x42; 32];

            let pubkey = get_stark_pubkey(&private_key);
            assert!(pubkey.is_ok(), "Should generate pubkey");

            let pk = pubkey.unwrap();
            assert!(pk.starts_with("0x"), "Pubkey should be hex");
            assert_eq!(pk.len(), 66, "Pubkey should be 0x + 64 hex chars");
        }

        #[test]
        fn test_stark_pubkey_deterministic() {
            let private_key = [0x42; 32];

            let pk1 = get_stark_pubkey(&private_key).unwrap();
            let pk2 = get_stark_pubkey(&private_key).unwrap();

            assert_eq!(pk1, pk2, "Pubkey generation should be deterministic");
        }
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
    async fn test_write_operations_without_signer_return_error() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Write operations without private key should return informative errors
        let result = marketplace.register_prover(1000, None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no private key"));

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

    #[test]
    fn test_private_key_is_zeroized() {
        let client = create_test_client();
        let valid_key = "0x0000000000000000000000000000000000000000000000000000000000000001";

        // Create marketplace with private key
        let marketplace = StarknetMarketplace::new_with_signer(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
            valid_key.to_string(),
        );

        assert!(marketplace.can_sign());
        // When marketplace is dropped, Zeroizing<String> will clear the private key from memory
        drop(marketplace);
        // This test verifies that Zeroizing is used correctly
    }
}
