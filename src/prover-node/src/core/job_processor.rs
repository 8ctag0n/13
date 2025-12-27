use anyhow::{Context, Result};
use log::{error, info, warn};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::sync::Arc;
use zyberlink_types::{CircuitType, FheOperation, JobStatus};

use crate::circuits::{CensusCircuit, DemographicsCircuit, PassportCircuit, VotingCircuit};
use crate::core::{CiphertextFetcher, CircuitRegistry};
use crate::engines::{
    FheBalanceEngine, FheBalanceInput, FheBalanceProcessor, FheJobParams,
    JobType as FheJobType, EncryptedValue, TfheCiphertext,
};
use crate::gateway::GatewayClient;
use crate::halo2_prover::{Halo2Prover, OrchardWitness};
use crate::marketplace::{JobData, JobSource, MarketplaceOperations, StarknetJobType};
use crate::tui;
use crate::witness_encryption::WitnessEncryption;
use crate::witness_fetcher::WitnessFetcher;
use zyberlink_fhe::{deserialize_server_key, FheEngine};

// Futarchy FHE imports
use fhe_client_sdk::FutarchyFheClient;
use crate::futarchy::{FutarchyPoolWorker, FutarchyPoolJob, CiphertextFetcher as FutarchyCiphertextFetcher};

/// Job processor handles claiming, processing, and submitting proofs for jobs
///
/// Uses `MarketplaceOperations` trait for multi-chain support.
/// Accepts any implementation of the trait (Solana, Ethereum, etc.)
pub struct JobProcessor {
    marketplace: Arc<dyn MarketplaceOperations>,
    keypair: Arc<Keypair>,
    halo2_prover: Arc<Halo2Prover>,
    witness_encryption: Arc<WitnessEncryption>,
    witness_fetcher: Arc<WitnessFetcher>,
    gateway_client: Arc<GatewayClient>,
    fhe_engine: Option<Arc<FheEngine>>,
    /// Futarchy FHE worker for encrypted pool updates
    futarchy_worker: Option<Arc<FutarchyPoolWorker>>,
    /// App server URL for fetching Futarchy ciphertexts
    futarchy_app_server_url: Option<String>,
    /// FHE Balance Engine for Starknet jobs (pBTCFi, pLST)
    fhe_balance_engine: Arc<FheBalanceEngine>,
    /// Blink server URL for fetching off-chain TFHE ciphertexts
    blink_server_url: Option<String>,
}

impl JobProcessor {
    /// Create a new JobProcessor
    pub fn new(
        marketplace: Arc<dyn MarketplaceOperations>,
        keypair: Arc<Keypair>,
        halo2_prover: Arc<Halo2Prover>,
        witness_encryption: Arc<WitnessEncryption>,
        witness_fetcher: Arc<WitnessFetcher>,
        gateway_client: Arc<GatewayClient>,
        fhe_engine: Option<Arc<FheEngine>>,
    ) -> Self {
        // Create FHE balance engine (real if FHE engine available, mock otherwise)
        let fhe_balance_engine = if let Some(ref engine) = fhe_engine {
            Arc::new(FheBalanceEngine::new(engine.clone()))
        } else {
            Arc::new(FheBalanceEngine::new_mock())
        };

        Self {
            marketplace,
            keypair,
            halo2_prover,
            witness_encryption,
            witness_fetcher,
            gateway_client,
            fhe_engine,
            futarchy_worker: None,
            futarchy_app_server_url: None,
            fhe_balance_engine,
            blink_server_url: None,
        }
    }

    /// Configure blink server URL for fetching off-chain TFHE ciphertexts
    pub fn with_blink_server(mut self, url: String) -> Self {
        self.blink_server_url = Some(url);
        self
    }

    /// Configure Futarchy FHE support
    ///
    /// # Arguments
    /// * `app_server_url` - URL of the app server storing ciphertexts
    pub fn with_futarchy_support(mut self, app_server_url: String) -> Self {
        // Create FHE client for Futarchy (this generates keys, takes a few seconds)
        match FutarchyFheClient::new() {
            Ok(fhe_client) => {
                let worker = FutarchyPoolWorker::with_client(fhe_client, &app_server_url);
                self.futarchy_worker = Some(Arc::new(worker));
                self.futarchy_app_server_url = Some(app_server_url);
                info!("Futarchy FHE support enabled");
            }
            Err(e) => {
                warn!("Failed to initialize Futarchy FHE client: {}. Futarchy jobs will fail.", e);
            }
        }
        self
    }


    /// Process a single job: claim -> prove -> submit
    pub async fn process_job(
        &self,
        job: &JobData,
        tui_state: Option<Arc<tui::TUIState>>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        let job_id = job.id;
        info!("[Job {}] Starting processing", job_id);

        // Route to chain-specific processor
        match job.source {
            JobSource::Starknet => {
                return self.process_starknet_job(job, tui_state, start_time).await;
            }
            _ => {}
        }

        // Parse job address (Solana/Aptos)
        let job_pda: Pubkey = job.address.parse()
            .context("Invalid job address")?;

        // Step 1: Claim the job
        self.claim_job(&job_pda, job).await?;

        // Step 2: Download witness from backend
        let witness_bytes = self.download_witness(&job.witness_hash, job_id).await?;

        // Step 3: Generate proof based on circuit type
        let proof_bytes = self
            .generate_proof(job_id, &job.circuit_type, &witness_bytes)
            .await?;

        // Step 4: Submit result based on job type
        self.submit_result(&job_pda, job, &proof_bytes).await?;

        info!("[Job {}] Completed!", job_id);

        // Update TUI stats on successful completion
        self.update_tui_stats(tui_state, start_time, job_id, job.price, &job.circuit_type);

        Ok(())
    }

    /// Process a Starknet FHE job using FheBalanceEngine
    async fn process_starknet_job(
        &self,
        job: &JobData,
        tui_state: Option<Arc<tui::TUIState>>,
        start_time: std::time::Instant,
    ) -> Result<()> {
        let job_id = job.id;
        info!("[Job {}] Processing Starknet FHE job", job_id);

        // Get Starknet-specific job type
        let starknet_job_type = job.starknet_job_type
            .ok_or_else(|| anyhow::anyhow!("Starknet job missing job_type"))?;

        // Get encrypted data from job
        let encrypted_c1 = job.encrypted_c1
            .ok_or_else(|| anyhow::anyhow!("Starknet job missing encrypted_c1"))?;
        let encrypted_c2 = job.encrypted_c2
            .ok_or_else(|| anyhow::anyhow!("Starknet job missing encrypted_c2"))?;

        info!(
            "[Job {}] Job type: {:?}, encrypted data present",
            job_id, starknet_job_type
        );

        // Step 1: Claim the job on Starknet
        info!("[Job {}] Claiming job on Starknet...", job_id);
        let claim_result = match self.marketplace.claim_job(job_id, &job.creator).await {
            Ok(result) => result,
            Err(e) => {
                // Handle race condition gracefully - another prover claimed first
                let err_str = e.to_string();
                if err_str.contains("already claimed") || err_str.contains("Job already claimed") {
                    info!(
                        "[Job {}] Skipping: job was claimed by another prover (race condition)",
                        job_id
                    );
                    return Ok(()); // Graceful exit, continue to next job
                }
                return Err(anyhow::anyhow!("Failed to claim Starknet job: {}", e));
            }
        };
        info!("[Job {}] Claimed (tx: {})", job_id, claim_result.signature);

        // Step 2: Build FHE input (fetches TFHE ciphertexts if available)
        let fhe_input = self.build_fhe_balance_input(
            job_id,
            starknet_job_type,
            encrypted_c1,
            encrypted_c2,
            job.payload_hash,
        ).await?;

        let engine = Arc::clone(&self.fhe_balance_engine);
        let fhe_result = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(engine.process(&fhe_input))
        })
        .await
        .context("FHE processing task panicked")??;

        info!(
            "[Job {}] FHE computation complete: success={}, result_hash={}",
            job_id,
            fhe_result.success,
            hex::encode(&fhe_result.result_hash[..8])
        );

        // Step 3: Submit result to Starknet contract
        info!("[Job {}] Submitting result to Starknet...", job_id);
        let submit_result = self.marketplace
            .submit_fhe_result(job_id, &job.creator, fhe_result.result_hash)
            .await
            .context("Failed to submit Starknet result")?;
        info!("[Job {}] Result submitted (tx: {})", job_id, submit_result.signature);

        info!("[Job {}] Starknet job completed!", job_id);

        // Update TUI stats
        if let Some(ref tui) = tui_state {
            let duration = start_time.elapsed().as_secs_f64();
            tui.update_stats(|stats| {
                stats.jobs_completed += 1;
                stats.total_earnings_lamports += job.price;
                let total_completed = stats.jobs_completed as f64;
                let old_avg = stats.avg_proof_time_secs;
                stats.avg_proof_time_secs =
                    ((old_avg * (total_completed - 1.0)) + duration) / total_completed;
            });

            let job_type_name = format!("Starknet:{:?}", starknet_job_type);
            tui.add_recent_job(tui::RecentJob {
                id: job_id,
                job_type: job_type_name,
                status: "completed".to_string(),
                duration_secs: duration,
                earnings_lamports: job.price,
            });
        }

        Ok(())
    }

    /// Build FheBalanceInput from Starknet job data
    ///
    /// If `payload_hash` is provided and blink_server_url is configured,
    /// fetches real TFHE ciphertexts from off-chain storage.
    async fn build_fhe_balance_input(
        &self,
        job_id: u64,
        starknet_job_type: StarknetJobType,
        encrypted_c1: [u8; 32],
        encrypted_c2: [u8; 32],
        payload_hash: Option<[u8; 32]>,
    ) -> Result<FheBalanceInput> {
        let job_type = match starknet_job_type {
            StarknetJobType::LoanVerification => FheJobType::LoanVerification,
            StarknetJobType::BalanceUpdate => FheJobType::BalanceUpdate,
            StarknetJobType::StakeProof => FheJobType::StakeProof,
            StarknetJobType::TransferProof => FheJobType::TransferProof,
            StarknetJobType::LiquidationCheck => FheJobType::LiquidationCheck,
        };

        let current_encrypted = EncryptedValue::from_felts(encrypted_c1, encrypted_c2);

        // Build params based on job type (using defaults for MVP)
        let params = match job_type {
            FheJobType::LoanVerification => FheJobParams::LoanVerification {
                min_collateral_ratio: 150,
                btc_price_oracle: 50000,
            },
            FheJobType::BalanceUpdate => FheJobParams::BalanceUpdate { is_mint: true },
            FheJobType::StakeProof => FheJobParams::StakeProof {
                min_stake_amount: 1000,
                staking_period: 86400,
            },
            FheJobType::TransferProof => FheJobParams::TransferProof {
                transfer_amount_encrypted: EncryptedValue::from_felts([0u8; 32], [0u8; 32]),
            },
            FheJobType::LiquidationCheck => FheJobParams::LiquidationCheck {
                liquidation_threshold: 110,
                current_price: 48000,
            },
            FheJobType::LtvCheck => FheJobParams::LtvCheck {
                ltv_threshold_bps: 7500, // 75% default
                collateral_price_usd: 1_000_000, // $1 with 6 decimals
            },
        };

        // Fetch real TFHE ciphertexts if payload_hash is available
        let (tfhe_current, tfhe_delta) = match (&self.blink_server_url, payload_hash) {
            (Some(url), Some(hash)) => {
                info!("[Job {}] Fetching TFHE ciphertext from blink-server...", job_id);
                let fetcher = CiphertextFetcher::new(url.clone());

                match fetcher.fetch_with_retry(&hash, 3).await {
                    Ok(ciphertext) => {
                        info!(
                            "[Job {}] Fetched TFHE ciphertext: {} bytes",
                            job_id,
                            ciphertext.data.len()
                        );
                        // For now, we only fetch the current balance ciphertext
                        // Delta would require a separate hash (future enhancement)
                        (Some(ciphertext), None)
                    }
                    Err(e) => {
                        warn!(
                            "[Job {}] Failed to fetch TFHE ciphertext: {}. Falling back to mock mode.",
                            job_id, e
                        );
                        (None, None)
                    }
                }
            }
            (None, Some(_)) => {
                warn!(
                    "[Job {}] payload_hash present but blink_server_url not configured. Using mock mode.",
                    job_id
                );
                (None, None)
            }
            _ => {
                info!("[Job {}] No payload_hash, using mock FHE mode.", job_id);
                (None, None)
            }
        };

        Ok(FheBalanceInput {
            job_id,
            job_type,
            current_encrypted,
            delta_encrypted: None,
            params,
            tfhe_current,
            tfhe_delta,
        })
    }

    /// Claim a job (different instruction for FHE vs ZK)
    async fn claim_job(
        &self,
        job_pda: &Pubkey,
        job: &JobData,
    ) -> Result<()> {
        info!("[Job {}] Claiming job...", job.id);

        // Use trait method - prefer v2 for FHE jobs (supports new generators)
        let result = if job.is_fhe || matches!(job.circuit_type, CircuitType::FheComputation(_)) {
            // FHE multi-prover jobs - use v2 which handles new FHE-Generator
            self.marketplace
                .claim_fhe_job_v2(job)
                .await
                .context("Failed to claim FHE job")?
        } else {
            // ZK single-prover jobs
            self.marketplace
                .claim_job(job.id, &job.creator)
                .await
                .context("Failed to claim job")?
        };

        info!("[Job {}] Claimed successfully (sig: {})", job.id, result.signature);

        // Verify claim succeeded
        self.verify_claim(job_pda, job.id, &job.creator, &job.circuit_type, &job.source).await?;

        Ok(())
    }

    /// Verify that the job was successfully claimed
    async fn verify_claim(
        &self,
        _job_pda: &Pubkey,
        job_id: u64,
        creator: &str,
        circuit_type: &CircuitType,
        job_source: &JobSource,
    ) -> Result<()> {
        // For new FHE-Generator jobs, skip verification - the claim tx succeeded
        if matches!(job_source, JobSource::FheGenerator | JobSource::ZkGenerator) {
            info!("[Job {}] Claim verified (new generator tx succeeded)", job_id);
            return Ok(());
        }

        let job = self.marketplace.get_job(job_id, creator).await?
            .ok_or_else(|| anyhow::anyhow!("Job not found after claim"))?;

        // For ZK jobs: verify we claimed it exclusively
        if let CircuitType::ZcashOrchard = circuit_type {
            let our_address = self.keypair.pubkey().to_string();
            if job.status != JobStatus::Claimed || job.prover.as_ref() != Some(&our_address) {
                warn!("[Job {}] ZK job not claimed by us, aborting", job_id);
                return Err(anyhow::anyhow!("Job not claimed by us"));
            }
        }

        // For FHE jobs: verify consensus is properly configured
        if let CircuitType::FheComputation(_) = circuit_type {
            let fhe_config = self.marketplace.get_fhe_consensus_config(job_id).await?
                .ok_or_else(|| anyhow::anyhow!("FHE consensus not found"))?;

            // Basic validation - job should be FHE type
            if !job.is_fhe {
                warn!("[Job {}] Job is not marked as FHE type", job_id);
                return Err(anyhow::anyhow!("Job is not FHE type"));
            }

            info!(
                "[Job {}] FHE consensus configured: required_provers={}, threshold={}",
                job_id, fhe_config.required_provers, fhe_config.consensus_threshold
            );
        }

        Ok(())
    }

    /// Download witness from backend
    async fn download_witness(&self, witness_hash: &[u8; 32], job_id: u64) -> Result<Vec<u8>> {
        info!(
            "[Job {}] Downloading encrypted witness from backend...",
            job_id
        );

        let witness_bytes = self
            .witness_fetcher
            .download_witness(witness_hash)
            .await
            .context("Failed to download witness from backend")?;

        info!(
            "[Job {}] Downloaded encrypted witness ({} bytes)",
            job_id,
            witness_bytes.len()
        );

        Ok(witness_bytes)
    }

    /// Generate proof based on circuit type
    async fn generate_proof(
        &self,
        job_id: u64,
        circuit_type: &CircuitType,
        witness_bytes: &[u8],
    ) -> Result<Vec<u8>> {
        info!(
            "[Job {}] Generating proof (circuit: {:?})...",
            job_id, circuit_type
        );

        let proof_bytes = match circuit_type {
            CircuitType::ZcashOrchard => {
                self.generate_zk_proof(job_id, witness_bytes).await?
            }
            CircuitType::FheComputation(ref operation) => {
                self.generate_fhe_proof(job_id, witness_bytes, operation)
                    .await?
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported circuit type: {:?}",
                    circuit_type
                ));
            }
        };

        info!(
            "[Job {}] Result generated successfully ({} bytes)",
            job_id,
            proof_bytes.len()
        );

        Ok(proof_bytes)
    }

    /// Generate ZK proof using Halo2
    async fn generate_zk_proof(&self, job_id: u64, witness_bytes: &[u8]) -> Result<Vec<u8>> {
        // Decrypt witness
        info!("[Job {}] Decrypting witness data...", job_id);
        let witness = self
            .witness_encryption
            .decrypt_witness(witness_bytes)
            .context("Failed to decrypt witness data")?;
        info!("[Job {}] Witness decrypted successfully", job_id);

        // Validate witness
        witness.validate().context("Invalid witness data")?;

        // Generate real Halo2 proof
        let proof = Self::real_generate_proof(self.halo2_prover.clone(), witness).await?;
        info!(
            "[Job {}] Halo2 proof generated successfully ({} bytes)",
            job_id,
            proof.len()
        );

        Ok(proof)
    }

    /// Generate FHE proof (execute FHE computation)
    async fn generate_fhe_proof(
        &self,
        job_id: u64,
        witness_bytes: &[u8],
        operation: &FheOperation,
    ) -> Result<Vec<u8>> {
        // Handle Futarchy pool updates separately (different data source)
        if let FheOperation::FutarchyPoolUpdate {
            market_id,
            side,
            pool_ciphertext_hash,
            bet_ciphertext_hash,
        } = operation
        {
            return self
                .execute_futarchy_pool_update(
                    job_id,
                    market_id,
                    *side,
                    pool_ciphertext_hash,
                    bet_ciphertext_hash,
                )
                .await;
        }

        info!("[Job {}] Executing FHE operation: {:?}", job_id, operation);

        // DEBUG: Hash the full witness to verify all provers get identical data
        let witness_full_hash = FheEngine::hash_result(witness_bytes);
        info!(
            "[Job {}] DEBUG witness_hash: {} ({} bytes)",
            job_id,
            hex::encode(&witness_full_hash[..16]),
            witness_bytes.len()
        );

        // Parse witness format: [encrypted_data_len (4 bytes)] [encrypted_data] [server_key]
        if witness_bytes.len() < 4 {
            return Err(anyhow::anyhow!(
                "Witness too short to contain length prefix"
            ));
        }

        let encrypted_data_len = u32::from_le_bytes([
            witness_bytes[0],
            witness_bytes[1],
            witness_bytes[2],
            witness_bytes[3],
        ]) as usize;

        let header_size = 4;
        let encrypted_data_end = header_size + encrypted_data_len;

        if witness_bytes.len() < encrypted_data_end {
            return Err(anyhow::anyhow!(
                "Witness too short: expected at least {} bytes, got {}",
                encrypted_data_end,
                witness_bytes.len()
            ));
        }

        let encrypted_data = &witness_bytes[header_size..encrypted_data_end];
        let server_key_bytes = &witness_bytes[encrypted_data_end..];

        // DEBUG: Hash each component
        let enc_hash = FheEngine::hash_result(encrypted_data);
        let key_hash = FheEngine::hash_result(server_key_bytes);
        info!(
            "[Job {}] DEBUG enc_data: {} | server_key: {}",
            job_id,
            hex::encode(&enc_hash[..8]),
            hex::encode(&key_hash[..8])
        );

        // Deserialize server key and create FHE engine
        info!("[Job {}] Initializing FHE engine from witness...", job_id);
        let server_key = deserialize_server_key(server_key_bytes)
            .context("Failed to deserialize server key from witness")?;
        let engine = Arc::new(FheEngine::new(server_key));
        info!("[Job {}] FHE engine initialized", job_id);

        // Perform FHE computation with extracted encrypted data
        let result_bytes = Self::execute_fhe_computation(engine.clone(), encrypted_data, operation)
            .await?;

        // Hash result for consensus
        let result_hash = FheEngine::hash_result(&result_bytes);

        info!(
            "[Job {}] DEBUG result_hash: {} ({} bytes)",
            job_id,
            hex::encode(&result_hash[..16]),
            result_bytes.len()
        );

        Ok(result_bytes)
    }

    /// Submit result based on job type
    async fn submit_result(
        &self,
        job_pda: &Pubkey,
        job: &JobData,
        proof_bytes: &[u8],
    ) -> Result<()> {
        match &job.circuit_type {
            CircuitType::ZcashOrchard => {
                self.submit_zk_proof(job_pda, job.id, &job.creator, proof_bytes).await?;
            }
            CircuitType::FheComputation(ref op) => {
                self.submit_fhe_result(job_pda, job, proof_bytes, op).await?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported circuit type: {:?}",
                    job.circuit_type
                ));
            }
        }

        Ok(())
    }

    /// Submit ZK proof
    async fn submit_zk_proof(
        &self,
        job_pda: &Pubkey,
        job_id: u64,
        creator: &str,
        proof_bytes: &[u8],
    ) -> Result<()> {
        info!("[Job {}] Submitting ZK proof...", job_id);

        // Generate proof commitment (hash of actual proof)
        let proof_commitment = Self::generate_proof_commitment(proof_bytes);
        let proof_size = proof_bytes.len() as u32;

        // Use trait method to submit proof (no fee_recipient needed)
        let result = self.marketplace
            .submit_proof(job_id, creator, proof_commitment, proof_size, None)
            .await
            .context("Failed to submit proof")?;

        info!(
            "[Job {}] ZK proof submitted successfully (sig: {})",
            job_id, result.signature
        );

        Ok(())
    }

    /// Submit FHE result
    async fn submit_fhe_result(
        &self,
        _job_pda: &Pubkey,
        job: &JobData,
        proof_bytes: &[u8],
        operation: &FheOperation,
    ) -> Result<()> {
        info!("[Job {}] Submitting FHE result...", job.id);

        // Use deterministic commitment for consensus
        let result_hash =
            FheEngine::deterministic_commitment(&job.witness_hash, operation.name(), job.id);

        info!(
            "[Job {}] FHE deterministic commitment: {} (op: {})",
            job.id,
            hex::encode(&result_hash[..8]),
            operation.name()
        );

        // Store encrypted result in witness backend
        self.upload_fhe_result(job.id, proof_bytes).await?;

        // Use trait method to submit FHE result - v2 supports new FHE-Generator
        match self.marketplace
            .submit_fhe_result_v2(job, result_hash)
            .await
        {
            Ok(result) => {
                info!(
                    "[Job {}] FHE result submitted successfully (sig: {})",
                    job.id, result.signature
                );
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check if job was already completed by other provers (race condition)
                if err_str.contains("custom program error: 0x20")
                    || err_str.contains("InvalidJobStatus")
                    || err_str.contains("Job must be in Pending or Claimed")
                {
                    info!(
                        "[Job {}] Job already completed by other provers, skipping submit",
                        job.id
                    );
                } else {
                    error!("[Job {}] Failed to submit FHE result: {}", job.id, e);
                    return Err(anyhow::anyhow!("Failed to submit FHE result: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Upload FHE result to gateway
    async fn upload_fhe_result(&self, job_id: u64, proof_bytes: &[u8]) -> Result<()> {
        info!(
            "[Job {}] Uploading encrypted result to gateway...",
            job_id
        );

        let stored_commitment = self
            .gateway_client
            .submit_fhe_result(job_id, proof_bytes)
            .await
            .context("Failed to submit FHE result to gateway")?;

        info!(
            "[Job {}] FHE result stored with commitment: {}",
            job_id, stored_commitment
        );

        Ok(())
    }

    /// Generate real Halo2 proof (CPU-intensive, runs in blocking thread)
    async fn real_generate_proof(
        prover: Arc<Halo2Prover>,
        witness: OrchardWitness,
    ) -> Result<Vec<u8>> {
        tokio::task::spawn_blocking(move || prover.generate_orchard_proof(witness))
            .await
            .context("Proof generation task panicked")?
    }

    /// Execute FHE computation (runs in blocking thread since it's CPU-intensive)
    async fn execute_fhe_computation(
        engine: Arc<FheEngine>,
        encrypted_input: &[u8],
        operation: &FheOperation,
    ) -> Result<Vec<u8>> {
        let input_bytes = encrypted_input.to_vec();
        let operation = operation.clone();

        tokio::task::spawn_blocking(move || {
            // Set server key in this thread's context
            engine.set_key_for_thread();

            match operation {
                FheOperation::Add(constant) => engine.compute_add(&input_bytes, constant),
                FheOperation::Multiply(constant) => engine.compute_multiply(&input_bytes, constant),

                FheOperation::Sum { expected_count } => {
                    let inputs: Vec<Vec<u8>> = bincode::deserialize(&input_bytes)
                        .context("Failed to deserialize Sum inputs")?;

                    if inputs.len() != expected_count as usize {
                        anyhow::bail!(
                            "Expected {} inputs for Sum operation, got {}",
                            expected_count,
                            inputs.len()
                        );
                    }

                    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
                    engine.compute_sum(&input_refs)
                }

                FheOperation::Threshold {
                    threshold,
                    greater_or_equal,
                } => PassportCircuit::compute_threshold(&input_bytes, threshold, greater_or_equal),

                FheOperation::RangeCheck { min, max } => {
                    PassportCircuit::compute_range_check(&input_bytes, min, max)
                }

                FheOperation::Average { expected_count } => {
                    let inputs: Vec<Vec<u8>> = bincode::deserialize(&input_bytes)
                        .context("Failed to deserialize Average inputs")?;

                    if inputs.len() != expected_count as usize {
                        anyhow::bail!(
                            "Expected {} inputs for Average operation, got {}",
                            expected_count,
                            inputs.len()
                        );
                    }

                    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

                    let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(input_refs)
                        .context("Failed to compute average")?;

                    bincode::serialize(&(encrypted_sum, count))
                        .context("Failed to serialize average result")
                }

                FheOperation::CountIf {
                    ref predicate,
                    expected_count,
                } => {
                    let inputs: Vec<Vec<u8>> = bincode::deserialize(&input_bytes)
                        .context("Failed to deserialize CountIf inputs")?;

                    if inputs.len() != expected_count as usize {
                        anyhow::bail!(
                            "Expected {} inputs for CountIf operation, got {}",
                            expected_count,
                            inputs.len()
                        );
                    }

                    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
                    engine.compute_count_if(&input_refs, predicate)
                }

                FheOperation::Histogram { ref bins } => {
                    let inputs: Vec<Vec<u8>> = bincode::deserialize(&input_bytes)
                        .context("Failed to deserialize Histogram inputs")?;

                    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

                    VotingCircuit::compute_histogram(input_refs, bins)
                        .context("Failed to compute histogram")
                }

                // FutarchyPoolUpdate is handled separately in generate_fhe_proof
                // This branch should never be reached
                FheOperation::FutarchyPoolUpdate { .. } => {
                    anyhow::bail!("FutarchyPoolUpdate should be handled by execute_futarchy_pool_update")
                }
            }
        })
        .await
        .context("FHE computation task panicked")?
    }

    /// Generate proof commitment (hash of actual proof)
    fn generate_proof_commitment(proof: &[u8]) -> [u8; 32] {
        use solana_sdk::hash::hash;
        let hash_result = hash(proof);
        hash_result.to_bytes()
    }

    /// Execute Futarchy pool update (homomorphic addition)
    ///
    /// This is different from regular FHE operations because:
    /// 1. Ciphertexts come from app server, not witness backend
    /// 2. Uses FheUint64 (larger ciphertexts, ~500KB each)
    /// 3. Returns result hash (result stored off-chain)
    async fn execute_futarchy_pool_update(
        &self,
        job_id: u64,
        market_id: &[u8; 32],
        side: bool,
        pool_ciphertext_hash: &[u8; 32],
        bet_ciphertext_hash: &[u8; 32],
    ) -> Result<Vec<u8>> {
        info!(
            "[Job {}] Executing Futarchy pool update (side: {})",
            job_id,
            if side { "YES" } else { "NO" }
        );

        // Verify Futarchy support is enabled
        let worker = self.futarchy_worker.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Futarchy support not enabled. Call with_futarchy_support() on JobProcessor"
            )
        })?;

        let app_server_url = self.futarchy_app_server_url.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Futarchy app server URL not configured")
        })?;

        // Convert market_id bytes to string (pubkey)
        let market_id_str = solana_sdk::pubkey::Pubkey::from(*market_id).to_string();

        // Fetch ciphertexts from blockchain using hashes
        // In the new flow, we need to fetch actual ciphertexts from Position and Market accounts
        // For now, we'll still use the app server fetcher (legacy flow)
        // TODO: Migrate to direct blockchain fetch when poller is fully integrated

        let fetcher = FutarchyCiphertextFetcher::new(app_server_url);

        let pool_ciphertext = fetcher
            .fetch_by_hash(pool_ciphertext_hash)
            .context("Failed to fetch pool ciphertext from app server")?;

        let bet_ciphertext = fetcher
            .fetch_by_hash(bet_ciphertext_hash)
            .context("Failed to fetch bet ciphertext from app server")?;

        // Create job data for the worker with full ciphertexts
        let futarchy_job = FutarchyPoolJob {
            job_id,
            market_id: market_id_str,
            side,
            bet_ciphertext,
            pool_ciphertext,
            bet_ciphertext_hash: *bet_ciphertext_hash,
            pool_ciphertext_hash: *pool_ciphertext_hash,
        };

        // Process job (CPU-intensive, run in blocking thread)
        let worker_clone: Arc<FutarchyPoolWorker> = Arc::clone(worker);
        let result = tokio::task::spawn_blocking(move || {
            worker_clone.process_and_submit(&futarchy_job)
        })
        .await
        .context("Futarchy pool update task panicked")??;

        info!(
            "[Job {}] Futarchy pool update completed. Result hash: {}",
            job_id,
            hex::encode(&result.result_hash[..8])
        );

        // Return the result hash as the "proof" bytes
        // This will be submitted on-chain for consensus
        Ok(result.result_hash.to_vec())
    }

    /// Update TUI stats on successful completion
    fn update_tui_stats(
        &self,
        tui_state: Option<Arc<tui::TUIState>>,
        start_time: std::time::Instant,
        job_id: u64,
        job_price: u64,
        circuit_type: &CircuitType,
    ) {
        if let Some(ref tui) = tui_state {
            let duration = start_time.elapsed().as_secs_f64();

            tui.update_stats(|stats| {
                stats.jobs_completed += 1;
                stats.total_earnings_lamports += job_price;

                let total_completed = stats.jobs_completed as f64;
                let old_avg = stats.avg_proof_time_secs;
                stats.avg_proof_time_secs =
                    ((old_avg * (total_completed - 1.0)) + duration) / total_completed;
            });

            let circuit_name = match circuit_type {
                CircuitType::ZcashOrchard => "ZK Proof".to_string(),
                CircuitType::FheComputation(_) => "FHE Comp".to_string(),
                _ => "Unknown".to_string(),
            };

            tui.add_recent_job(tui::RecentJob {
                id: job_id,
                job_type: circuit_name,
                status: "completed".to_string(),
                duration_secs: duration,
                earnings_lamports: job_price,
            });
        }
    }
}
