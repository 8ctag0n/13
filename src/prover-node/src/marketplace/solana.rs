//! Solana marketplace implementation
//!
//! Wraps the existing `MarketplaceClient` from zyberlink-sdk to implement
//! the `MarketplaceOperations` trait for Solana blockchain.

use super::{
    FheConsensusConfig, JobData, MarketplaceError, MarketplaceOperations, ProverData, Result,
    TransactionResult,
};
use async_trait::async_trait;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::sync::Arc;
use zyberlink_sdk::{
    fetch_job, fetch_prover, find_fhe_jobs_needing_provers, find_pending_jobs,
    JobAccount, MarketplaceClient, ProverAccount,
};
use zyberlink_types::{CircuitType, JobStatus};

/// Solana marketplace wrapper
///
/// Implements `MarketplaceOperations` by delegating to `MarketplaceClient`.
/// This allows prover-node to work with Solana without direct SDK coupling.
///
/// # Example
///
/// ```rust,ignore
/// use prover_node::marketplace::{SolanaMarketplace, MarketplaceOperations};
///
/// let marketplace = SolanaMarketplace::new(
///     "https://api.devnet.solana.com",
///     program_id,
///     keypair,
/// )?;
///
/// let jobs = marketplace.find_pending_jobs().await?;
/// ```
pub struct SolanaMarketplace {
    client: MarketplaceClient,
    keypair: Arc<Keypair>,
    rpc_url: String,
    network: String,
    program_id: Pubkey,
}

impl SolanaMarketplace {
    /// Create a new Solana marketplace client
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint URL
    /// * `program_id` - Marketplace program ID
    /// * `keypair` - Prover keypair for signing transactions
    pub fn new(rpc_url: &str, program_id: Pubkey, keypair: Arc<Keypair>) -> Result<Self> {
        let client = MarketplaceClient::new_with_commitment(
            rpc_url.to_string(),
            program_id,
            CommitmentConfig::confirmed(),
        );

        let network = Self::detect_network(rpc_url);

        Ok(Self {
            client,
            keypair,
            rpc_url: rpc_url.to_string(),
            network,
            program_id,
        })
    }

    /// Create with custom commitment level
    pub fn new_with_commitment(
        rpc_url: &str,
        program_id: Pubkey,
        keypair: Arc<Keypair>,
        commitment: CommitmentConfig,
    ) -> Result<Self> {
        let client =
            MarketplaceClient::new_with_commitment(rpc_url.to_string(), program_id, commitment);

        let network = Self::detect_network(rpc_url);

        Ok(Self {
            client,
            keypair,
            rpc_url: rpc_url.to_string(),
            network,
            program_id,
        })
    }

    /// Detect network from RPC URL
    fn detect_network(rpc_url: &str) -> String {
        if rpc_url.contains("mainnet") {
            "mainnet".to_string()
        } else if rpc_url.contains("devnet") {
            "devnet".to_string()
        } else if rpc_url.contains("testnet") {
            "testnet".to_string()
        } else {
            "localnet".to_string()
        }
    }

    /// Get direct access to the underlying MarketplaceClient
    ///
    /// Use sparingly - prefer trait methods when possible.
    pub fn inner(&self) -> &MarketplaceClient {
        &self.client
    }

    /// Get the prover keypair
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Convert Solana JobAccount to generic JobData
    fn job_to_data(job: &JobAccount, job_pda: &Pubkey) -> JobData {
        // Convert circuit_type u8 to CircuitType enum
        let circuit_type = Self::circuit_type_from_u8(job.circuit_type);
        let is_fhe = job.fhe_consensus_bump.is_some();

        JobData {
            id: job.id,
            creator: job.creator.to_string(),
            circuit_type,
            witness_hash: job.witness_hash,
            price: job.price_lamports,
            status: job.status.clone(),
            prover: job.prover.map(|p| p.to_string()),
            created_at: job.created_at,
            timeout_at: job.timeout_at,
            is_fhe,
            address: job_pda.to_string(),
        }
    }

    /// Convert circuit type u8 to CircuitType enum
    fn circuit_type_from_u8(val: u8) -> CircuitType {
        match val {
            0 => CircuitType::ZcashOrchard,
            1 => CircuitType::AnonymousVote,
            2 => CircuitType::Credential,
            _ => CircuitType::Custom(format!("unknown_{}", val)),
        }
    }

    /// Convert Solana ProverAccount to generic ProverData
    fn prover_to_data(prover: &ProverAccount) -> ProverData {
        ProverData {
            authority: prover.authority.to_string(),
            stake: prover.stake_amount,
            encryption_pubkey: Some(prover.encryption_pubkey),
            is_active: prover.is_active,
            jobs_completed: prover.total_jobs_completed,
            jobs_failed: prover.total_jobs_failed,
            reputation: (prover.reputation_score / 10) as u8, // Convert 0-1000 to 0-100
            registered_at: prover.registration_timestamp,
        }
    }
}

#[async_trait]
impl MarketplaceOperations for SolanaMarketplace {
    async fn register_prover(
        &self,
        stake: u64,
        encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult> {
        let pubkey = encryption_pubkey.unwrap_or([0u8; 32]);

        let instruction = self
            .client
            .register_prover_instruction(&self.keypair.pubkey(), stake, pubkey)
            .map_err(|e| MarketplaceError::ChainError(e.to_string()))?;

        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| MarketplaceError::TransactionFailed(e.to_string()))?;

        Ok(TransactionResult::success(signature.to_string()))
    }

    async fn get_prover(&self, authority: &str) -> Result<Option<ProverData>> {
        let authority_pubkey: Pubkey = authority
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", authority, e)))?;

        let (prover_pda, _) = self.client.get_prover_pda(&authority_pubkey);

        match fetch_prover(&self.client.rpc_client, &prover_pda) {
            Ok(prover) => Ok(Some(Self::prover_to_data(&prover))),
            Err(_) => Ok(None),
        }
    }

    async fn is_registered(&self) -> Result<bool> {
        let prover = self.get_prover(&self.keypair.pubkey().to_string()).await?;
        Ok(prover.is_some())
    }

    async fn find_pending_jobs(&self) -> Result<Vec<JobData>> {
        let jobs = find_pending_jobs(&self.client.rpc_client, &self.program_id)
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        Ok(jobs
            .into_iter()
            .filter(|(_, job)| job.fhe_consensus_bump.is_none()) // Exclude FHE jobs
            .map(|(pda, job)| Self::job_to_data(&job, &pda))
            .collect())
    }

    async fn find_fhe_jobs_needing_provers(&self) -> Result<Vec<JobData>> {
        let jobs = find_fhe_jobs_needing_provers(
            &self.client.rpc_client,
            &self.program_id,
            &self.keypair.pubkey(),
        )
        .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        Ok(jobs
            .into_iter()
            .map(|(pda, job, _fhe_data)| Self::job_to_data(&job, &pda))
            .collect())
    }

    async fn get_job(&self, job_id: u64, creator: &str) -> Result<Option<JobData>> {
        let creator_pubkey: Pubkey = creator
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", creator, e)))?;

        let (job_pda, _) = self.client.get_job_pda(&creator_pubkey, job_id);

        match fetch_job(&self.client.rpc_client, &job_pda) {
            Ok(job) => Ok(Some(Self::job_to_data(&job, &job_pda))),
            Err(_) => Ok(None),
        }
    }

    async fn claim_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult> {
        let creator_pubkey: Pubkey = creator
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", creator, e)))?;

        let (job_pda, _) = self.client.get_job_pda(&creator_pubkey, job_id);

        let instruction = self
            .client
            .claim_job_instruction(&self.keypair.pubkey(), &job_pda)
            .map_err(|e| MarketplaceError::ChainError(e.to_string()))?;

        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| {
                if e.to_string().contains("already claimed") {
                    MarketplaceError::JobAlreadyClaimed
                } else if e.to_string().contains("expired") {
                    MarketplaceError::JobExpired
                } else {
                    MarketplaceError::TransactionFailed(e.to_string())
                }
            })?;

        Ok(TransactionResult::success(signature.to_string()))
    }

    async fn claim_fhe_job(&self, job_id: u64, creator: &str) -> Result<TransactionResult> {
        let creator_pubkey: Pubkey = creator
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", creator, e)))?;

        let (job_pda, _) = self.client.get_job_pda(&creator_pubkey, job_id);

        let instruction = self
            .client
            .claim_fhe_job_instruction(&self.keypair.pubkey(), &job_pda, job_id)
            .map_err(|e| MarketplaceError::ChainError(e.to_string()))?;

        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| {
                if e.to_string().contains("already claimed") {
                    MarketplaceError::JobAlreadyClaimed
                } else if e.to_string().contains("expired") {
                    MarketplaceError::JobExpired
                } else {
                    MarketplaceError::TransactionFailed(e.to_string())
                }
            })?;

        Ok(TransactionResult::success(signature.to_string()))
    }

    async fn submit_proof(
        &self,
        job_id: u64,
        creator: &str,
        proof_commitment: [u8; 32],
        proof_size: u32,
        fee_recipient: Option<&str>,
    ) -> Result<TransactionResult> {
        let creator_pubkey: Pubkey = creator
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", creator, e)))?;

        let (job_pda, _) = self.client.get_job_pda(&creator_pubkey, job_id);

        let instruction = if let Some(recipient) = fee_recipient {
            let recipient_pubkey: Pubkey = recipient
                .parse()
                .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", recipient, e)))?;

            self.client
                .submit_proof_instruction_with_recipient(
                    &self.keypair.pubkey(),
                    &job_pda,
                    &creator_pubkey,
                    &recipient_pubkey,
                    proof_commitment,
                    proof_size,
                )
                .map_err(|e| MarketplaceError::ChainError(e.to_string()))?
        } else {
            self.client
                .submit_proof_instruction(
                    &self.keypair.pubkey(),
                    &job_pda,
                    &creator_pubkey,
                    proof_commitment,
                    proof_size,
                )
                .map_err(|e| MarketplaceError::ChainError(e.to_string()))?
        };

        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| MarketplaceError::TransactionFailed(e.to_string()))?;

        Ok(TransactionResult::success(signature.to_string()))
    }

    async fn submit_fhe_result(
        &self,
        job_id: u64,
        creator: &str,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        let creator_pubkey: Pubkey = creator
            .parse()
            .map_err(|e| MarketplaceError::InvalidAddress(format!("{}: {}", creator, e)))?;

        let (job_pda, _) = self.client.get_job_pda(&creator_pubkey, job_id);

        let instruction = self
            .client
            .submit_fhe_result_instruction(&self.keypair.pubkey(), &job_pda, job_id, result_hash)
            .map_err(|e| MarketplaceError::ChainError(e.to_string()))?;

        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| MarketplaceError::TransactionFailed(e.to_string()))?;

        Ok(TransactionResult::success(signature.to_string()))
    }

    async fn get_fhe_consensus_config(&self, job_id: u64) -> Result<Option<FheConsensusConfig>> {
        // FHE consensus config is stored in the FheConsensusData account
        let jobs = find_fhe_jobs_needing_provers(
            &self.client.rpc_client,
            &self.program_id,
            &self.keypair.pubkey(),
        )
        .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        for (_pda, job, fhe_data) in jobs {
            if job.id == job_id {
                return Ok(Some(FheConsensusConfig {
                    required_provers: fhe_data.required_provers,
                    consensus_threshold: fhe_data.consensus_threshold,
                }));
            }
        }

        Ok(None)
    }

    fn chain_id(&self) -> &str {
        "solana"
    }

    fn network(&self) -> &str {
        &self.network
    }

    fn program_address(&self) -> &str {
        // Return cached string to avoid allocation on each call
        // This is a slight hack but avoids lifetime issues
        Box::leak(self.program_id.to_string().into_boxed_str())
    }

    fn prover_address(&self) -> &str {
        Box::leak(self.keypair.pubkey().to_string().into_boxed_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_network() {
        assert_eq!(
            SolanaMarketplace::detect_network("https://api.mainnet-beta.solana.com"),
            "mainnet"
        );
        assert_eq!(
            SolanaMarketplace::detect_network("https://api.devnet.solana.com"),
            "devnet"
        );
        assert_eq!(
            SolanaMarketplace::detect_network("http://localhost:8899"),
            "localnet"
        );
    }

    #[test]
    fn test_prover_reputation_conversion() {
        // Test that 0-1000 scale converts to 0-100
        let prover = ProverAccount {
            authority: Pubkey::new_unique(),
            stake_amount: 1_000_000_000,
            reputation_score: 850, // Should become 85
            total_jobs_completed: 10,
            total_jobs_failed: 1,
            avg_completion_time_secs: 30,
            is_active: true,
            registration_timestamp: 0,
            total_earnings_lamports: 0,
            encryption_pubkey: [0u8; 32],
            bump: 255,
        };

        let data = SolanaMarketplace::prover_to_data(&prover);
        assert_eq!(data.reputation, 85);
    }

    #[test]
    fn test_circuit_type_conversion() {
        assert!(matches!(
            SolanaMarketplace::circuit_type_from_u8(0),
            CircuitType::ZcashOrchard
        ));
        assert!(matches!(
            SolanaMarketplace::circuit_type_from_u8(1),
            CircuitType::AnonymousVote
        ));
        assert!(matches!(
            SolanaMarketplace::circuit_type_from_u8(2),
            CircuitType::Credential
        ));
        assert!(matches!(
            SolanaMarketplace::circuit_type_from_u8(99),
            CircuitType::Custom(_)
        ));
    }
}
