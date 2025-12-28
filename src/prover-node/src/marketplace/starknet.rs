//! Starknet marketplace implementation
//!
//! Implements `MarketplaceOperations` for Starknet blockchain using `StarknetClient`.
//! This allows prover-node to work with Starknet Cairo contracts.
//!
//! # Current Status
//!
//! This is a stub implementation. Full functionality requires:
//! - Cairo marketplace contract deployment
//! - Contract ABI integration
//! - Starknet SDK method implementations in StarknetClient

use super::{
    FheConsensusConfig, JobData, MarketplaceError, MarketplaceOperations, ProverData, Result,
    TransactionResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use zyberlink_chain_client::{ChainClient, StarknetClient};

/// Starknet marketplace wrapper
///
/// Implements `MarketplaceOperations` by delegating to `StarknetClient`.
/// Most methods are currently stubs returning errors until Cairo contracts are deployed.
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
/// // Currently returns empty vec - stub implementation
/// let jobs = marketplace.find_pending_jobs().await?;
/// ```
pub struct StarknetMarketplace {
    client: Arc<StarknetClient>,
    contract_address: String,
    prover_address: String,
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
        }
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

    /// Helper to create "not implemented" error
    fn not_implemented(operation: &str) -> MarketplaceError {
        MarketplaceError::Other(format!(
            "Starknet marketplace operation '{}' requires Cairo contract implementation",
            operation
        ))
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
        Err(Self::not_implemented("register_prover"))
    }

    async fn get_prover(&self, _authority: &str) -> Result<Option<ProverData>> {
        // Stub: return None until Cairo contract read is implemented
        Ok(None)
    }

    async fn is_registered(&self) -> Result<bool> {
        // Stub: return false until Cairo contract read is implemented
        Ok(false)
    }

    // ========== Job Discovery ==========

    async fn find_pending_jobs(&self) -> Result<Vec<JobData>> {
        // Stub: return empty vec until Cairo contract read is implemented
        Ok(Vec::new())
    }

    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>> {
        // Stub: return empty vec until Cairo contract read is implemented
        Ok(Vec::new())
    }

    async fn get_job(&self, _job_id: u64, _creator: &str) -> Result<Option<JobData>> {
        // Stub: return None until Cairo contract read is implemented
        Ok(None)
    }

    // ========== Job Execution ==========

    async fn claim_job(&self, _job_id: u64, _creator: &str) -> Result<TransactionResult> {
        Err(Self::not_implemented("claim_job"))
    }

    async fn claim_fhe_job(&self, _job_id: u64, _creator: &str) -> Result<TransactionResult> {
        Err(Self::not_implemented("claim_fhe_job"))
    }

    async fn submit_proof(
        &self,
        _job_id: u64,
        _creator: &str,
        _proof_commitment: [u8; 32],
        _proof_size: u32,
        _fee_recipient: Option<&str>,
    ) -> Result<TransactionResult> {
        Err(Self::not_implemented("submit_proof"))
    }

    async fn submit_fhe_result(
        &self,
        _job_id: u64,
        _creator: &str,
        _result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        Err(Self::not_implemented("submit_fhe_result"))
    }

    // ========== FHE Consensus ==========

    async fn get_fhe_consensus_config(&self, _job_id: u64) -> Result<Option<FheConsensusConfig>> {
        // Stub: return None until Cairo contract read is implemented
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

    #[tokio::test]
    async fn test_stub_operations_return_empty() {
        let client = create_test_client();
        let marketplace = StarknetMarketplace::new(
            client,
            "0x123abc".to_string(),
            "0xdef456".to_string(),
        );

        // Read operations should return empty/none without error
        assert!(marketplace.find_pending_jobs().await.unwrap().is_empty());
        assert!(marketplace
            .find_fhe_jobs_needing_provers()
            .await
            .unwrap()
            .is_empty());
        assert!(marketplace.get_job(1, "0xabc").await.unwrap().is_none());
        assert!(marketplace.get_prover("0xabc").await.unwrap().is_none());
        assert!(!marketplace.is_registered().await.unwrap());
        assert!(marketplace
            .get_fhe_consensus_config(1)
            .await
            .unwrap()
            .is_none());
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
            .contains("Cairo contract implementation"));

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
}
