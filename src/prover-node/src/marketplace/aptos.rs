//! Aptos marketplace implementation
//!
//! Implements the `MarketplaceOperations` trait for Aptos blockchain.
//! This is a stub implementation until Move contracts are deployed.

use super::{
    FheConsensusConfig, JobData, MarketplaceError, MarketplaceOperations, ProverData, Result,
    TransactionResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use zyberlink_chain_client::{AptosClient, ChainClient};

/// Aptos marketplace implementation
///
/// This implementation uses `AptosClient` to interact with Aptos Move contracts.
/// Currently, most methods are stubs that return appropriate errors until
/// the Move contracts are deployed and integrated.
///
/// # Example
///
/// ```rust,ignore
/// use prover_node::marketplace::{AptosMarketplace, MarketplaceOperations};
///
/// let marketplace = AptosMarketplace::new(
///     aptos_client,
///     "0x123...abc".to_string(),
///     "0x456...def".to_string(),
/// );
///
/// let jobs = marketplace.find_pending_jobs().await?;
/// ```
pub struct AptosMarketplace {
    client: Arc<AptosClient>,
    program_address: String,
    prover_address: String,
    network: String,
    private_key: Option<String>,
}

impl AptosMarketplace {
    /// Create a new Aptos marketplace client
    ///
    /// # Arguments
    /// * `client` - Aptos client instance
    /// * `program_address` - Marketplace module address (e.g., "0x123...abc")
    /// * `prover_address` - Prover account address
    /// * `private_key` - Optional private key for signing (hex)
    pub fn new(
        client: Arc<AptosClient>,
        program_address: String,
        prover_address: String,
        private_key: Option<String>,
    ) -> Self {
        // Get network from ChainClient trait implementation
        let network = client.network().to_string();

        Self {
            client,
            program_address,
            prover_address,
            network,
            private_key,
        }
    }

    /// Get direct access to the underlying AptosClient
    ///
    /// Use sparingly - prefer trait methods when possible.
    pub fn inner(&self) -> &AptosClient {
        &self.client
    }

    /// Create a stub error for unimplemented operations
    fn not_implemented(operation: &str) -> MarketplaceError {
        MarketplaceError::Other(format!(
            "Aptos marketplace operation '{}' not yet implemented - Move contracts pending deployment",
            operation
        ))
    }

    fn get_signer_key(&self) -> Result<&String> {
        self.private_key.as_ref().ok_or_else(|| {
            MarketplaceError::Other("Private key required for this operation".to_string())
        })
    }

    /// Submit entry function using Aptos REST API with JSON payload
    /// This uses the signing_message + submit flow to avoid BCS serialization issues
    async fn submit_entry_function_json(
        &self,
        module_name: &str,
        function_name: &str,
        type_args: Vec<String>,
        args: Vec<serde_json::Value>,
    ) -> Result<String> {
        let private_key_hex = self.get_signer_key()?;
        let rpc_url = self.client.rpc_url().trim_end_matches("/v1");

        // Build the transaction payload
        let function_id = format!("{}::{}::{}", self.program_address, module_name, function_name);

        // Get account sequence number
        let account_url = format!("{}/v1/accounts/{}", rpc_url, self.prover_address);
        let account_resp: serde_json::Value = reqwest::Client::new()
            .get(&account_url)
            .send()
            .await
            .map_err(|e| MarketplaceError::Network(format!("Failed to get account: {}", e)))?
            .json()
            .await
            .map_err(|e| MarketplaceError::Deserialization(format!("Failed to parse account: {}", e)))?;

        let sequence_number = account_resp["sequence_number"]
            .as_str()
            .ok_or_else(|| MarketplaceError::Deserialization("Missing sequence_number".to_string()))?
            .parse::<u64>()
            .map_err(|e| MarketplaceError::Deserialization(format!("Invalid sequence_number: {}", e)))?;

        // Get chain info for gas estimation and expiration
        let ledger_url = format!("{}/v1", rpc_url);
        let ledger_resp: serde_json::Value = reqwest::Client::new()
            .get(&ledger_url)
            .send()
            .await
            .map_err(|e| MarketplaceError::Network(format!("Failed to get ledger info: {}", e)))?
            .json()
            .await
            .map_err(|e| MarketplaceError::Deserialization(format!("Failed to parse ledger: {}", e)))?;

        let ledger_timestamp = ledger_resp["ledger_timestamp"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0) / 1_000_000; // Convert from microseconds to seconds

        // Build transaction request for signing_message endpoint
        let tx_request = serde_json::json!({
            "sender": self.prover_address,
            "sequence_number": sequence_number.to_string(),
            "max_gas_amount": "200000",
            "gas_unit_price": "100",
            "expiration_timestamp_secs": (ledger_timestamp + 600).to_string(),
            "payload": {
                "type": "entry_function_payload",
                "function": function_id,
                "type_arguments": type_args,
                "arguments": args,
            }
        });

        // Get signing message from Aptos API
        let encode_url = format!("{}/v1/transactions/encode_submission", rpc_url);
        let encode_resp = reqwest::Client::new()
            .post(&encode_url)
            .json(&tx_request)
            .send()
            .await
            .map_err(|e| MarketplaceError::Network(format!("Failed to encode tx: {}", e)))?;

        if !encode_resp.status().is_success() {
            let error_text = encode_resp.text().await.unwrap_or_default();
            return Err(MarketplaceError::TransactionFailed(format!(
                "Failed to encode transaction: {}", error_text
            )));
        }

        let signing_message: String = encode_resp.json().await
            .map_err(|e| MarketplaceError::Deserialization(format!("Failed to parse signing message: {}", e)))?;

        // Decode the hex signing message and sign it directly
        // Aptos signing_message from encode_submission is already pre-hashed
        let message_bytes = hex::decode(signing_message.trim_start_matches("0x"))
            .map_err(|e| MarketplaceError::Other(format!("Invalid signing message hex: {}", e)))?;

        // Sign with Ed25519 - sign the message directly (Aptos already provides the hash)
        use ed25519_dalek::{Signer, SigningKey};

        let private_key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
            .map_err(|e| MarketplaceError::Other(format!("Invalid private key hex: {}", e)))?;

        let signing_key = SigningKey::from_bytes(&private_key_bytes.try_into()
            .map_err(|_| MarketplaceError::Other("Private key must be 32 bytes".to_string()))?);

        // Sign the message directly - Aptos expects this
        let signature = signing_key.sign(&message_bytes);
        let public_key = signing_key.verifying_key();

        // Submit signed transaction
        let submit_request = serde_json::json!({
            "sender": self.prover_address,
            "sequence_number": sequence_number.to_string(),
            "max_gas_amount": "200000",
            "gas_unit_price": "100",
            "expiration_timestamp_secs": (ledger_timestamp + 600).to_string(),
            "payload": {
                "type": "entry_function_payload",
                "function": function_id,
                "type_arguments": type_args,
                "arguments": args,
            },
            "signature": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}", hex::encode(public_key.as_bytes())),
                "signature": format!("0x{}", hex::encode(signature.to_bytes())),
            }
        });

        let submit_url = format!("{}/v1/transactions", rpc_url);
        let submit_resp = reqwest::Client::new()
            .post(&submit_url)
            .json(&submit_request)
            .send()
            .await
            .map_err(|e| MarketplaceError::Network(format!("Failed to submit tx: {}", e)))?;

        if !submit_resp.status().is_success() {
            let error_text = submit_resp.text().await.unwrap_or_default();
            return Err(MarketplaceError::TransactionFailed(format!(
                "Transaction submission failed: {}", error_text
            )));
        }

        let submit_result: serde_json::Value = submit_resp.json().await
            .map_err(|e| MarketplaceError::Deserialization(format!("Failed to parse submit response: {}", e)))?;

        submit_result["hash"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| MarketplaceError::Other("Missing transaction hash in response".to_string()))
    }
}

#[async_trait]
impl MarketplaceOperations for AptosMarketplace {
    // ========== Prover Lifecycle ==========

    async fn register_prover(
        &self,
        _stake: u64,
        _encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult> {
        // Stub: Would call Move function like:
        // marketplace::register_prover(stake, encryption_pubkey)
        Err(Self::not_implemented("register_prover"))
    }

    async fn get_prover(&self, authority: &str) -> Result<Option<ProverData>> {
        // Stub: Would query Move resource like:
        // let resource = client.call_contract(
        //     &self.program_address,
        //     "marketplace",
        //     "get_prover",
        //     vec![authority],
        // ).await?;
        log::debug!(
            "get_prover called for authority: {} (stub implementation)",
            authority
        );
        Ok(None)
    }

    async fn is_registered(&self) -> Result<bool> {
        // Stub: Check if prover exists
        let prover = self.get_prover(&self.prover_address).await?;
        Ok(prover.is_some())
    }

    // ========== Job Discovery ==========

    async fn find_pending_jobs(&self) -> Result<Vec<JobData>> {
        // Aptos doesn't have a get_pending_jobs function, so we poll jobs sequentially
        // Starting from job_id=0 until we hit non-existent jobs
        // This is inefficient but works for MVP - production needs event indexing

        let mut pending_jobs = Vec::new();
        let max_poll = 100; // Safety limit to prevent infinite loops

        for job_id in 0..max_poll {
            match self.get_job(job_id, &self.program_address).await {
                Ok(Some(job)) if job.status == zyberlink_types::JobStatus::Pending => {
                    pending_jobs.push(job);
                }
                Ok(Some(_)) => {
                    // Job exists but not pending, continue polling
                }
                Ok(None) => {
                    // Job doesn't exist - we've reached the end
                    break;
                }
                Err(e) => {
                    log::debug!("Error polling job {}: {}", job_id, e);
                    break;
                }
            }
        }

        log::debug!("find_pending_jobs found {} pending jobs", pending_jobs.len());
        Ok(pending_jobs)
    }

    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>> {
        // Get all pending jobs and filter for FHE jobs that need provers
        let pending_jobs = self.find_pending_jobs().await?;
        let mut needing_provers = Vec::new();

        for job in pending_jobs {
            if !job.is_fhe {
                continue;
            }

            // Check consensus config to see if more provers needed
            match self.get_fhe_consensus_config(job.id).await {
                Ok(Some(config)) => {
                    // Check how many provers have claimed
                    // For now, assume if job is pending, it needs provers
                    // A more sophisticated version would check claimed_provers.len() < required_provers
                    needing_provers.push(job);
                }
                Ok(None) => {
                    // No consensus config - shouldn't happen for FHE jobs
                    log::warn!("FHE job {} has no consensus config", job.id);
                }
                Err(e) => {
                    log::debug!("Failed to get consensus config for job {}: {}", job.id, e);
                }
            }
        }

        log::debug!("find_fhe_jobs_needing_provers found {} jobs", needing_provers.len());
        Ok(needing_provers)
    }

    async fn get_job(&self, job_id: u64, _creator: &str) -> Result<Option<JobData>> {
        // Call jobs::get_job(registry_addr, job_id)
        let args = vec![
            serde_json::json!(self.program_address.clone()),
            serde_json::json!(job_id.to_string()),
        ];

        let result = self.client
            .call_view_function(
                &self.program_address,
                "jobs",
                "get_job",
                vec![],
                args,
            )
            .await;

        match result {
            Ok(data) if !data.is_empty() => {
                // Response is [{...job object...}] - take first element
                let job_obj = &data[0];

                // Parse Job struct from Aptos response object
                let id = job_obj["id"].as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(job_id);

                let creator = job_obj["creator"].as_str().unwrap_or("0x0").to_string();
                let prover_addr = job_obj["prover"].as_str().unwrap_or("0x0");
                let prover = if prover_addr != "0x0" {
                    Some(prover_addr.to_string())
                } else {
                    None
                };

                let status_val = job_obj["status"].as_u64().unwrap_or(0) as u8;
                let status = match status_val {
                    0 => zyberlink_types::JobStatus::Pending,
                    1 => zyberlink_types::JobStatus::Claimed,
                    2 => zyberlink_types::JobStatus::Completed,
                    _ => zyberlink_types::JobStatus::Pending,
                };

                let circuit_type = job_obj["circuit_type"].as_u64().unwrap_or(0) as u8;
                let circuit = map_aptos_circuit_type(circuit_type);

                let price = job_obj["price"].as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let created_at = job_obj["created_at"].as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let timeout_at = job_obj["timeout_at"].as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let has_fhe_consensus = job_obj["has_fhe_consensus"].as_bool().unwrap_or(false);

                // Parse witness_hash from hex string into [u8; 32]
                let witness_hash_vec = job_obj["witness_hash"].as_str()
                    .map(|s| hex::decode(s.trim_start_matches("0x")).unwrap_or_default())
                    .unwrap_or_default();
                let mut witness_hash = [0u8; 32];
                let len = witness_hash_vec.len().min(32);
                witness_hash[..len].copy_from_slice(&witness_hash_vec[..len]);

                log::debug!(
                    "get_job({}) parsed: status={:?}, circuit_type={}, price={}, is_fhe={}",
                    job_id, status, circuit_type, price, has_fhe_consensus
                );

                Ok(Some(JobData {
                    id,
                    creator,
                    circuit_type: circuit,
                    witness_hash,
                    price,
                    status,
                    prover,
                    created_at: created_at as i64,
                    timeout_at: timeout_at as i64,
                    is_fhe: has_fhe_consensus,
                    address: self.program_address.clone(),
                    source: super::JobSource::Aptos,
                    program_id: Some(self.program_address.clone()),
                    starknet_job_type: None,
                    encrypted_c1: None,
                    encrypted_c2: None,
                    payload_hash: None,
                }))
            }
            Ok(_) => {
                log::debug!("get_job({}) returned empty data", job_id);
                Ok(None)
            }
            Err(e) => {
                // Job doesn't exist or other error
                if e.to_string().contains("JOB_NOT_FOUND") || e.to_string().contains("NOT_INITIALIZED") {
                    Ok(None)
                } else {
                    log::debug!("get_job({}) failed: {}", job_id, e);
                    Ok(None)
                }
            }
        }
    }

    // ========== Job Execution ==========

    async fn claim_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult> {
        // Stub: Would call Move entry function like:
        // let tx = client.execute_contract(
        //     &self.program_address,
        //     "marketplace",
        //     "claim_job",
        //     vec![job_id, creator],
        // ).await?;
        log::warn!(
            "claim_job called for job_id: {}, creator: {} (not implemented)",
            job_id,
            creator
        );
        Err(Self::not_implemented("claim_job"))
    }

    async fn claim_fhe_job(&self, job_id: u64, _creator: &str) -> Result<TransactionResult> {
        // Use Aptos REST API to submit transaction via JSON
        // This avoids complex BCS serialization issues
        let tx_hash = self.submit_entry_function_json(
            "jobs",
            "claim_fhe_job",
            vec![],
            vec![
                serde_json::json!(self.program_address.clone()),
                serde_json::json!(job_id.to_string()),
            ],
        ).await?;

        log::info!("claim_fhe_job submitted: {}", tx_hash);
        Ok(TransactionResult::success(tx_hash))
    }

    async fn submit_proof(
        &self,
        job_id: u64,
        creator: &str,
        proof_commitment: [u8; 32],
        proof_size: u32,
        fee_recipient: Option<&str>,
    ) -> Result<TransactionResult> {
        // Stub: Would call Move entry function like:
        // let tx = client.execute_contract(
        //     &self.program_address,
        //     "marketplace",
        //     "submit_proof",
        //     vec![job_id, creator, proof_commitment, proof_size, fee_recipient],
        // ).await?;
        log::warn!(
            "submit_proof called for job_id: {}, creator: {}, proof_size: {}, fee_recipient: {:?} (not implemented)",
            job_id,
            creator,
            proof_size,
            fee_recipient
        );
        let _ = proof_commitment; // Suppress unused warning
        Err(Self::not_implemented("submit_proof"))
    }

    async fn submit_fhe_result(
        &self,
        job_id: u64,
        _creator: &str,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        // Use Aptos REST API to submit transaction via JSON
        let tx_hash = self.submit_entry_function_json(
            "jobs",
            "submit_fhe_result",
            vec![],
            vec![
                serde_json::json!(self.program_address.clone()),
                serde_json::json!(job_id.to_string()),
                serde_json::json!(format!("0x{}", hex::encode(result_hash))),
            ],
        ).await?;

        log::info!("submit_fhe_result submitted: {}", tx_hash);
        Ok(TransactionResult::success(tx_hash))
    }

    // ========== FHE Consensus ==========

    async fn get_fhe_consensus_config(&self, job_id: u64) -> Result<Option<FheConsensusConfig>> {
        // Call jobs::get_fhe_consensus(registry_addr, job_id)
        let args = vec![
            serde_json::json!(self.program_address.clone()),
            serde_json::json!(job_id.to_string()),
        ];

        let result = self.client
            .call_view_function(
                &self.program_address,
                "jobs",
                "get_fhe_consensus",
                vec![],
                args,
            )
            .await;

        match result {
            Ok(data) if data.len() >= 8 => {
                // Parse FheConsensusData struct
                // Fields: job_id, operation_type, operation_param1, operation_param2, operation_param3,
                //         required_provers, consensus_threshold, submission_timeout, claimed_provers,
                //         result_hashes, result_submitted, results_count, consensus_hash, finalized

                let required_provers = data[5].as_u64().unwrap_or(1) as u8;
                let consensus_threshold = data[6].as_u64().unwrap_or(1) as u8;

                log::debug!(
                    "get_fhe_consensus_config({}) -> required={}, threshold={}",
                    job_id, required_provers, consensus_threshold
                );

                Ok(Some(FheConsensusConfig {
                    required_provers,
                    consensus_threshold,
                }))
            }
            Ok(_) => {
                log::debug!("get_fhe_consensus_config({}) returned incomplete data", job_id);
                Ok(None)
            }
            Err(e) => {
                // Consensus doesn't exist for this job
                if e.to_string().contains("NOT_FHE_JOB") {
                    Ok(None)
                } else {
                    log::debug!("get_fhe_consensus_config({}) failed: {}", job_id, e);
                    Ok(None)
                }
            }
        }
    }

    // ========== Metadata ==========

    fn chain_id(&self) -> &str {
        "aptos"
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

// ============================================================================
// Helper functions for parsing Aptos view function responses
// ============================================================================

/// Parse vector<u8> from Aptos JSON response
/// Aptos returns vectors as JSON arrays of numbers
fn parse_vector_u8(value: &serde_json::Value) -> [u8; 32] {
    let mut result = [0u8; 32];

    if let Some(arr) = value.as_array() {
        for (i, byte_val) in arr.iter().enumerate() {
            if i >= 32 {
                break;
            }
            if let Some(byte) = byte_val.as_u64() {
                result[i] = byte as u8;
            }
        }
    }

    result
}

/// Map Aptos circuit type (u8) to CircuitType enum
///
/// Aptos circuit types:
/// - 0-3: ZK circuits (Zcash Orchard, Sapling, Anonymous Vote, Credential)
/// - 4-11: FHE operations (Add, Multiply, Sum, Threshold, etc.)
fn map_aptos_circuit_type(circuit_type: u8) -> zyberlink_types::CircuitType {
    use zyberlink_types::{CircuitType, FheOperation};

    match circuit_type {
        0 => CircuitType::ZcashOrchard,
        1 => CircuitType::Custom("zcash_sapling".to_string()),
        2 => CircuitType::AnonymousVote,
        3 => CircuitType::Credential,
        4 => CircuitType::FheComputation(FheOperation::Add(1)), // Add with default operand
        5 => CircuitType::FheComputation(FheOperation::Multiply(2)), // Multiply with default operand
        6 => CircuitType::FheComputation(FheOperation::Sum { expected_count: 10 }), // Sum with default count
        7 => CircuitType::FheComputation(FheOperation::Threshold { threshold: 18, greater_or_equal: true }),
        8 => CircuitType::FheComputation(FheOperation::RangeCheck { min: 0, max: 100 }),
        9 => CircuitType::FheComputation(FheOperation::Average { expected_count: 10 }),
        10 => CircuitType::FheComputation(FheOperation::CountIf {
            predicate: zyberlink_types::FhePredicate::GreaterThan(50),
            expected_count: 10
        }),
        11 => CircuitType::FheComputation(FheOperation::Histogram {
            bins: vec![] // Will be populated from job params
        }),
        _ => CircuitType::Custom(format!("unknown_circuit_{}", circuit_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let client = Arc::new(
            AptosClient::new(
                "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
                None,
            )
            .expect("Failed to create Aptos client"),
        );
        let marketplace = AptosMarketplace::new(
            client,
            "0x123".to_string(),
            "0x456".to_string(),
            None,
        );

        assert_eq!(marketplace.chain_id(), "aptos");
        assert_eq!(marketplace.network(), "testnet");
        assert_eq!(marketplace.program_address(), "0x123");
        assert_eq!(marketplace.prover_address(), "0x456");
    }

    #[tokio::test]
    async fn test_stub_implementations() {
        let client = Arc::new(
            AptosClient::new(
                "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
                None,
            )
            .expect("Failed to create Aptos client"),
        );
        let marketplace = AptosMarketplace::new(
            client,
            "0x123".to_string(),
            "0x456".to_string(),
            None,
        );

        // Test that stubs return appropriate results
        assert!(marketplace.find_pending_jobs().await.unwrap().is_empty());
        assert!(marketplace.find_fhe_jobs_needing_provers().await.unwrap().is_empty());
        assert!(marketplace.get_prover("0x789").await.unwrap().is_none());
        assert!(!marketplace.is_registered().await.unwrap());

        // Test that unimplemented operations return errors
        assert!(marketplace.register_prover(1000, None).await.is_err());
        assert!(marketplace.claim_job(1, "0x789").await.is_err());
        assert!(marketplace.submit_proof(1, "0x789", [0u8; 32], 100, None).await.is_err());
    }

    #[tokio::test]
    #[ignore] // Only run with local Aptos node
    async fn test_get_real_job() {
        let client = Arc::new(
            AptosClient::local().expect("Failed to create local Aptos client"),
        );
        let registry_addr = "0x8513a06eb9dcc92de56de2906c0b0ea4b99f92ac234411d2bf80c8f5bdb3fee5";
        let marketplace = AptosMarketplace::new(
            client,
            registry_addr.to_string(),
            "0xtest".to_string(),
            None,
        );

        // Try to get job #0
        match marketplace.get_job(0, registry_addr).await {
            Ok(Some(job)) => {
                println!("Successfully fetched job #0: {:?}", job);
                assert_eq!(job.id, 0);
                assert!(job.is_fhe);
            }
            Ok(None) => {
                println!("Job #0 not found (may not exist yet)");
            }
            Err(e) => {
                println!("Error fetching job #0: {}", e);
            }
        }
    }

    #[test]
    fn test_circuit_type_mapping() {
        // Test ZK circuits
        let zk_orchard = map_aptos_circuit_type(0);
        assert_eq!(zk_orchard, zyberlink_types::CircuitType::ZcashOrchard);

        // Test FHE operations
        let fhe_add = map_aptos_circuit_type(4);
        assert!(matches!(
            fhe_add,
            zyberlink_types::CircuitType::FheComputation(_)
        ));
    }

    #[tokio::test]
    #[ignore] // Only run with local Aptos node + registered prover
    async fn test_claim_and_submit_fhe_job() {
        // Private key from aptos-dev container
        let private_key = "0xf69aba325061576f8816972ca86793220a4235b6183135fe7cc19e840b88ff55";
        let registry_addr = "0x8513a06eb9dcc92de56de2906c0b0ea4b99f92ac234411d2bf80c8f5bdb3fee5";
        let prover_addr = registry_addr; // Same account

        let client = Arc::new(
            AptosClient::local().expect("Failed to create local Aptos client"),
        );

        let marketplace = AptosMarketplace::new(
            client,
            registry_addr.to_string(),
            prover_addr.to_string(),
            Some(private_key.to_string()),
        );

        // Find pending FHE jobs
        let pending = marketplace.find_pending_jobs().await.unwrap();
        println!("Found {} pending jobs", pending.len());

        if pending.is_empty() {
            println!("No pending jobs - create one with submit_fhe_job first");
            return;
        }

        // Get last FHE job (most recent, less likely to be expired)
        let job = pending.iter().rev().find(|j| j.is_fhe).expect("No FHE job found");
        println!("Claiming FHE job #{}", job.id);

        // Claim the job
        match marketplace.claim_fhe_job(job.id, &job.creator).await {
            Ok(result) => {
                println!("Claim TX: {}", result.signature);
                // Wait for transaction confirmation
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                println!("Claim failed: {}", e);
                return;
            }
        }

        // Submit FHE result
        let result_hash = [0xab; 32]; // Mock result hash
        match marketplace.submit_fhe_result(job.id, &job.creator, result_hash).await {
            Ok(result) => {
                println!("Submit TX: {}", result.signature);
                assert!(result.success);
            }
            Err(e) => {
                println!("Submit failed: {}", e);
            }
        }

        // Verify job status changed
        if let Ok(Some(updated_job)) = marketplace.get_job(job.id, registry_addr).await {
            println!("Job #{} status after submit: {:?}", job.id, updated_job.status);
        }
    }
}
