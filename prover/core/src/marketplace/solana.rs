//! Solana marketplace implementation
//!
//! Wraps the existing `MarketplaceClient` from zyberlink-sdk to implement
//! the `MarketplaceOperations` trait for Solana blockchain.

use super::{
    FheConsensusConfig, JobData, JobSource, MarketplaceError, MarketplaceOperations, ProverData,
    Result, TransactionResult,
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
use zyberlink_unified_sdk::query::JobQuery;

// SDKs for new generator programs
use bedrock_sdk::{derive_prover_pda, instructions as bedrock_instructions};
use fhe_generator_sdk::{derive_consensus_pda, instructions as fhe_instructions};

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
    /// Optional unified SDK query for new generators
    unified_query: Option<JobQuery>,
    /// FHE Generator program ID (for new architecture)
    fhe_generator: Option<Pubkey>,
    /// Bedrock program ID (required for FHE-Generator operations)
    bedrock_program: Option<Pubkey>,
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
            unified_query: None,
            fhe_generator: None,
            bedrock_program: None,
        })
    }

    /// Create with generator program IDs for unified SDK support
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint URL
    /// * `program_id` - Legacy marketplace program ID
    /// * `keypair` - Prover keypair
    /// * `zk_generator` - ZK Generator program ID
    /// * `fhe_generator` - FHE Generator program ID
    ///
    /// Note: Bedrock program ID is auto-detected from known deployments
    pub fn new_with_generators(
        rpc_url: &str,
        program_id: Pubkey,
        keypair: Arc<Keypair>,
        zk_generator: Option<Pubkey>,
        fhe_generator: Option<Pubkey>,
    ) -> Result<Self> {
        let client = MarketplaceClient::new_with_commitment(
            rpc_url.to_string(),
            program_id,
            CommitmentConfig::confirmed(),
        );

        let network = Self::detect_network(rpc_url);

        // Create unified query if ANY generator is configured
        // Uses Pubkey::default() for unconfigured generators (will return empty results)
        let unified_query = if zk_generator.is_some() || fhe_generator.is_some() {
            let zk = zk_generator.unwrap_or_default();
            let fhe = fhe_generator.unwrap_or_default();
            log::info!("Creating JobQuery with ZK={}, FHE={}", zk, fhe);
            Some(JobQuery::from_url(rpc_url, zk, fhe))
        } else {
            None
        };

        // Bedrock program ID is the main program_id passed via CLI
        // The prover registers in Bedrock, and jobs are created in FHE-Generator
        let bedrock_program = Some(program_id);

        Ok(Self {
            client,
            keypair,
            rpc_url: rpc_url.to_string(),
            network,
            program_id,
            unified_query,
            fhe_generator,
            bedrock_program,
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
            unified_query: None,
            fhe_generator: None,
            bedrock_program: None,
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

    /// Convert Solana JobAccount to generic JobData (legacy program)
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
            source: JobSource::Legacy,
            program_id: None,
            // Starknet-specific fields (not used for Solana)
            starknet_job_type: None,
            encrypted_c1: None,
            encrypted_c2: None,
            payload_hash: None,
        }
    }

    /// Convert circuit type u8 to CircuitType enum
    fn circuit_type_from_u8(val: u8) -> CircuitType {
        use zyberlink_types::{FheOperation, HistogramBin};
        match val {
            0 => CircuitType::ZcashOrchard,
            1 => CircuitType::AnonymousVote,
            2 => CircuitType::Credential,
            // FHE circuits (4-11) - parameters will be filled from witness/consensus data
            4 => CircuitType::FheComputation(FheOperation::Add(0)),
            5 => CircuitType::FheComputation(FheOperation::Multiply(0)),
            6 => CircuitType::FheComputation(FheOperation::Sum { expected_count: 0 }),
            7 => CircuitType::FheComputation(FheOperation::Threshold { threshold: 0, greater_or_equal: true }),
            8 => CircuitType::FheComputation(FheOperation::RangeCheck { min: 0, max: 255 }),
            9 => CircuitType::FheComputation(FheOperation::Average { expected_count: 0 }),
            10 => CircuitType::FheComputation(FheOperation::CountIf {
                predicate: zyberlink_types::fhe::FhePredicate::GreaterThan(0),
                expected_count: 0,
            }),
            11 => {
                // Reconstruct 4 uniform bins (matches default dev-job bins_count)
                let bins_count = 4usize;
                let bins = (0..bins_count)
                    .map(|i| {
                        let bin_size = 256 / bins_count;
                        let min = (i * bin_size) as u8;
                        let max = ((i + 1) * bin_size - 1) as u8;
                        HistogramBin::new(min, max, &format!("{}-{}", min, max))
                    })
                    .collect();
                CircuitType::FheComputation(FheOperation::Histogram { bins })
            }
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

    /// Claim a job from the new FHE-Generator program
    async fn claim_fhe_generator_job(&self, job: &JobData) -> Result<TransactionResult> {
        // Verify we have the necessary program IDs
        let fhe_program = self.fhe_generator.ok_or_else(|| {
            MarketplaceError::Configuration("FHE Generator program not configured".to_string())
        })?;

        let bedrock_program = self.bedrock_program.ok_or_else(|| {
            MarketplaceError::Configuration("Bedrock program not configured".to_string())
        })?;

        // Parse job address
        let job_pda: Pubkey = job.address.parse().map_err(|e| {
            MarketplaceError::InvalidAddress(format!("Invalid job address: {}", e))
        })?;

        // Derive consensus PDA
        let (consensus_pda, _) = derive_consensus_pda(&fhe_program, job.id);

        // Derive prover PDA in Bedrock program
        let (prover_pda_bedrock, _) = derive_prover_pda(&bedrock_program, &self.keypair.pubkey());

        log::info!(
            "Claiming FHE-Generator job {} - job_pda: {}, consensus: {}, prover_bedrock: {}",
            job.id, job_pda, consensus_pda, prover_pda_bedrock
        );

        // Build claim instruction
        let instruction = fhe_instructions::claim_job(
            &fhe_program,
            &self.keypair.pubkey(),
            &job_pda,
            &consensus_pda,
            &prover_pda_bedrock,
            &bedrock_program,
        );

        // Send transaction
        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("already claimed") || msg.contains("ProverAlreadyClaimed") {
                    MarketplaceError::JobAlreadyClaimed
                } else if msg.contains("expired") || msg.contains("JobExpired") {
                    MarketplaceError::JobExpired
                } else {
                    MarketplaceError::TransactionFailed(msg)
                }
            })?;

        log::info!("Successfully claimed FHE-Generator job {} - tx: {}", job.id, signature);
        Ok(TransactionResult::success(signature.to_string()))
    }

    /// Submit result to the new FHE-Generator program
    async fn submit_fhe_generator_result(&self, job: &JobData, result_hash: [u8; 32]) -> Result<TransactionResult> {
        // Verify we have the necessary program IDs
        let fhe_program = self.fhe_generator.ok_or_else(|| {
            MarketplaceError::Configuration("FHE Generator program not configured".to_string())
        })?;

        // Parse job address
        let job_pda: Pubkey = job.address.parse().map_err(|e| {
            MarketplaceError::InvalidAddress(format!("Invalid job address: {}", e))
        })?;

        // Derive consensus PDA
        let (consensus_pda, _) = derive_consensus_pda(&fhe_program, job.id);

        log::info!(
            "Submitting result for FHE-Generator job {} - job_pda: {}, consensus: {}, hash: {}",
            job.id, job_pda, consensus_pda, hex::encode(&result_hash[..8])
        );

        // Build submit_result instruction
        let instruction = fhe_instructions::submit_result(
            &fhe_program,
            &self.keypair.pubkey(),
            &job_pda,
            &consensus_pda,
            result_hash,
        );

        // Send transaction
        let signature = self
            .client
            .send_and_confirm_transaction(&[instruction], &[&self.keypair])
            .map_err(|e| MarketplaceError::TransactionFailed(e.to_string()))?;

        log::info!("Successfully submitted result for FHE-Generator job {} - tx: {}", job.id, signature);
        Ok(TransactionResult::success(signature.to_string()))
    }
}

#[async_trait]
impl MarketplaceOperations for SolanaMarketplace {
    async fn register_prover(
        &self,
        stake: u64,
        _encryption_pubkey: Option<[u8; 32]>,
    ) -> Result<TransactionResult> {
        // Use bedrock-sdk for registration (matches deployed program)
        let prover_wallet = self.keypair.pubkey();
        let (prover_pda, _) = derive_prover_pda(&self.program_id, &prover_wallet);

        let instruction = bedrock_instructions::register_prover(
            &self.program_id,
            &prover_wallet,
            &prover_pda,
            stake,
        );

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
        let mut all_jobs = Vec::new();

        // 1. Query legacy program (zyberlink) for FHE jobs
        let legacy_jobs = find_fhe_jobs_needing_provers(
            &self.client.rpc_client,
            &self.program_id,
            &self.keypair.pubkey(),
        )
        .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        all_jobs.extend(
            legacy_jobs
                .into_iter()
                .map(|(pda, job, _fhe_data)| Self::job_to_data(&job, &pda))
        );

        // 2. Query new FHE-Generator using unified SDK (if configured)
        if let Some(ref query) = self.unified_query {
            match query.get_fhe_jobs_needing_provers(&self.keypair.pubkey()).await {
                Ok(unified_jobs) => {
                    for job in unified_jobs {
                        // Determine source based on job type
                        let source = if job.is_fhe() {
                            JobSource::FheGenerator
                        } else if job.is_zk() {
                            JobSource::ZkGenerator
                        } else {
                            JobSource::Legacy
                        };

                        all_jobs.push(JobData {
                            id: job.id(),
                            creator: job.creator().to_string(),
                            circuit_type: Self::circuit_type_from_u8(job.circuit_type()),
                            witness_hash: *job.witness_hash(),
                            price: job.price_lamports(),
                            status: job.status(),
                            prover: job.prover().map(|p| p.to_string()),
                            created_at: job.common().created_at,
                            timeout_at: job.common().timeout_at,
                            is_fhe: job.is_fhe(),
                            address: job.address().to_string(),
                            source,
                            program_id: Some(job.program_id().to_string()),
                            // Starknet-specific fields (not used for Solana)
                            starknet_job_type: None,
                            encrypted_c1: None,
                            encrypted_c2: None,
                            payload_hash: None,
                        });
                    }
                    log::info!("Found {} FHE jobs from unified SDK", all_jobs.len());
                }
                Err(e) => {
                    log::warn!("Failed to query FHE-Generator via unified SDK: {}", e);
                }
            }
        }

        Ok(all_jobs)
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

    async fn claim_fhe_job_v2(&self, job: &JobData) -> Result<TransactionResult> {
        // Check if this is a new FHE-Generator job
        if job.source == JobSource::FheGenerator {
            return self.claim_fhe_generator_job(job).await;
        }

        // Fall back to legacy method for Legacy jobs
        self.claim_fhe_job(job.id, &job.creator).await
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

    async fn submit_fhe_result_v2(
        &self,
        job: &JobData,
        result_hash: [u8; 32],
    ) -> Result<TransactionResult> {
        // Check if this is a new FHE-Generator job
        if job.source == JobSource::FheGenerator {
            return self.submit_fhe_generator_result(job, result_hash).await;
        }

        // Fall back to legacy method for Legacy jobs
        self.submit_fhe_result(job.id, &job.creator, result_hash).await
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
