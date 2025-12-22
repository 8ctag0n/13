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
    Result, TransactionResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use zyberlink_chain_client::{ChainClient, StarknetClient};
use zyberlink_types::{CircuitType, JobStatus};

/// Cairo function selectors (starknet_keccak of function name, truncated to 250 bits)
/// Pre-computed for PbtcfiJobs contract interface
mod selectors {
    /// get_pending_jobs() -> Array<u256>
    pub const GET_PENDING_JOBS: &str =
        "0x03b257e49fd4fa83c6951edd7b6f94ccb2aca13dd843240b43ef53a1a7fd000a";
    /// get_job_execution(loan_id: u256) -> JobExecution
    pub const GET_JOB_EXECUTION: &str =
        "0x02999d7d778e809ab93af886ca536f83c6c3986d95e902ce55361e03c641b83f";
    /// get_loan(loan_id: u256) -> Loan
    pub const GET_LOAN: &str =
        "0x03faf899aaaf6460caab4af9f3b6f5282c4c4720e4baa64ce53f6b822f4b47da";
    /// register_prover()
    pub const REGISTER_PROVER: &str =
        "0x0292f780aa61b49fbf9aa0ebc904a38993d165ee53ea8484e2c443258ebcbee7";
    /// claim_job(loan_id: u256)
    pub const CLAIM_JOB: &str =
        "0x014884a95bf7734febaf1e5f0ef0f2d2bf7014034b2362b7551c2b716029e2a0";
    /// submit_result(loan_id: u256, result_hash: felt252)
    pub const SUBMIT_RESULT: &str =
        "0x038b5fc200cc1074ba697b44368d487adb9924b6e1ed1aa05782e121915fc058";
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
    /// Private key for signing transactions (optional, enables write operations)
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
    /// * `private_key` - Private key hex string (e.g., "0x1234...")
    pub fn new_with_signer(
        client: Arc<StarknetClient>,
        contract_address: String,
        prover_address: String,
        private_key: String,
    ) -> Self {
        Self {
            client,
            contract_address,
            prover_address,
            private_key: Some(private_key),
        }
    }

    /// Set the private key for signing transactions
    pub fn set_private_key(&mut self, private_key: String) {
        self.private_key = Some(private_key);
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

    /// Parse a u256 from two felt252 values (low, high)
    fn parse_u256(low: &str, high: &str) -> std::result::Result<u64, MarketplaceError> {
        // For now, we only use the low part (assumes values fit in u64)
        Self::parse_felt_to_u64(low)
    }

    /// Convert u256 to calldata (low, high as felt252)
    fn u256_to_calldata(value: u64) -> Vec<String> {
        vec![format!("0x{:x}", value), "0x0".to_string()]
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
        // PbtcfiJobs stores provers in a Map<ContractAddress, Prover>
        // For now, check if the prover address matches and return basic data
        if authority == self.prover_address {
            Ok(Some(ProverData {
                authority: authority.to_string(),
                stake: 0,
                encryption_pubkey: None,
                is_active: true,
                jobs_completed: 0,
                jobs_failed: 0,
                reputation: 100,
                registered_at: 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn is_registered(&self) -> Result<bool> {
        // For MVP, assume registered if we have a prover address
        // Full implementation needs to read registered_provers storage
        Ok(!self.prover_address.is_empty())
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
        // Call get_job_execution(loan_id: u256) -> JobExecution
        let calldata = Self::u256_to_calldata(job_id);
        let result = self
            .call_view(selectors::GET_JOB_EXECUTION, calldata)
            .await;

        match result {
            Ok(data) if !data.is_empty() => {
                // Parse JobExecution struct:
                // loan_id: u256 (2 felts), prover: ContractAddress (1 felt),
                // status: JobStatus (1 felt), result_hash: felt252 (1 felt),
                // claimed_at: u64 (1 felt), completed_at: u64 (1 felt)
                let status = if data.len() > 4 {
                    match Self::parse_felt_to_u64(&data[4])? {
                        0 => JobStatus::Pending,
                        1 => JobStatus::Claimed,
                        2 => JobStatus::Completed,
                        _ => JobStatus::Pending,
                    }
                } else {
                    JobStatus::Pending
                };

                let prover = if data.len() > 2 {
                    data[2].clone()
                } else {
                    String::new()
                };

                Ok(Some(JobData {
                    id: job_id,
                    creator: self.contract_address.clone(),
                    circuit_type: CircuitType::Custom("pbtcfi_loan".to_string()),
                    witness_hash: [0u8; 32],
                    price: 0,
                    status,
                    prover: if prover.is_empty() || prover == "0x0" {
                        None
                    } else {
                        Some(prover)
                    },
                    created_at: 0,
                    timeout_at: 0,
                    is_fhe: false,
                    address: self.contract_address.clone(),
                    source: JobSource::Legacy, // Starknet jobs use Legacy source for now
                    program_id: Some(self.contract_address.clone()),
                }))
            }
            Ok(_) => Ok(None),
            Err(e) => {
                // Job might not exist - return None instead of error
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

        // Build calldata for claim_job(loan_id: u256)
        let calldata = Self::u256_to_calldata(job_id);

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
                selectors::CLAIM_JOB,
                &args_bytes,
                &self.prover_address,
            )
            .await
            .map_err(|e| MarketplaceError::Other(format!("claim_job failed: {}", e)))?;

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
        let result_hash = format!("0x{}", hex::encode(proof_commitment));
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

        // Should return true since prover_address is not empty
        assert!(marketplace.is_registered().await.unwrap());
    }

    #[tokio::test]
    async fn test_get_prover_returns_self() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Should return Some when querying own address
        let prover = marketplace.get_prover("0xdef456").await.unwrap();
        assert!(prover.is_some());
        assert_eq!(prover.unwrap().authority, "0xdef456");

        // Should return None for other addresses
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
        let marketplace_with_signer = StarknetMarketplace::new_with_signer(
            client.clone(),
            "0x123abc".to_string(),
            "0xdef456".to_string(),
            "0x1234567890abcdef".to_string(),
        );
        assert!(marketplace_with_signer.can_sign());

        // Setting private key after creation
        let mut marketplace_mut = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );
        assert!(!marketplace_mut.can_sign());
        marketplace_mut.set_private_key("0xabcdef".to_string());
        assert!(marketplace_mut.can_sign());
    }
}
