use anyhow::{Context, Result};
use log::{error, info, warn};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::sync::Arc;
use zyberlink_sdk::{fetch_fhe_consensus, fetch_job, MarketplaceClient};
use zyberlink_types::{CircuitType, FheOperation, JobStatus};

use crate::circuits::{CensusCircuit, DemographicsCircuit, PassportCircuit, VotingCircuit};
use crate::core::CircuitRegistry;
use crate::gateway::GatewayClient;
use crate::halo2_prover::{Halo2Prover, OrchardWitness};
use crate::tui;
use crate::witness_encryption::WitnessEncryption;
use crate::witness_fetcher::WitnessFetcher;
use zyberlink_fhe::{deserialize_server_key, FheEngine};

/// Job processor handles claiming, processing, and submitting proofs for jobs
pub struct JobProcessor {
    client: Arc<MarketplaceClient>,
    keypair: Arc<Keypair>,
    halo2_prover: Arc<Halo2Prover>,
    witness_encryption: Arc<WitnessEncryption>,
    witness_fetcher: Arc<WitnessFetcher>,
    gateway_client: Arc<GatewayClient>,
    fhe_engine: Option<Arc<FheEngine>>,
}

impl JobProcessor {
    /// Create a new JobProcessor
    pub fn new(
        client: Arc<MarketplaceClient>,
        keypair: Arc<Keypair>,
        halo2_prover: Arc<Halo2Prover>,
        witness_encryption: Arc<WitnessEncryption>,
        witness_fetcher: Arc<WitnessFetcher>,
        gateway_client: Arc<GatewayClient>,
        fhe_engine: Option<Arc<FheEngine>>,
    ) -> Self {
        Self {
            client,
            keypair,
            halo2_prover,
            witness_encryption,
            witness_fetcher,
            gateway_client,
            fhe_engine,
        }
    }

    /// Process a single job: claim -> prove -> submit
    pub async fn process_job(
        &self,
        job_pda: Pubkey,
        job_id: u64,
        circuit_type: CircuitType,
        witness_hash: [u8; 32],
        job_price: u64,
        tui_state: Option<Arc<tui::TUIState>>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!("[Job {}] Starting processing", job_id);

        // Step 1: Claim the job
        self.claim_job(&job_pda, job_id, &circuit_type).await?;

        // Step 2: Download witness from backend
        let witness_bytes = self.download_witness(&witness_hash, job_id).await?;

        // Step 3: Generate proof based on circuit type
        let proof_bytes = self
            .generate_proof(job_id, &circuit_type, &witness_bytes)
            .await?;

        // Step 4: Submit result based on job type
        self.submit_result(
            &job_pda,
            job_id,
            &circuit_type,
            &proof_bytes,
            &witness_hash,
        )
        .await?;

        info!("[Job {}] Completed!", job_id);

        // Update TUI stats on successful completion
        self.update_tui_stats(tui_state, start_time, job_id, job_price, &circuit_type);

        Ok(())
    }

    /// Claim a job (different instruction for FHE vs ZK)
    async fn claim_job(
        &self,
        job_pda: &Pubkey,
        job_id: u64,
        circuit_type: &CircuitType,
    ) -> Result<()> {
        info!("[Job {}] Claiming job...", job_id);

        let claim_ix = match circuit_type {
            CircuitType::FheComputation(_) => {
                // FHE multi-prover jobs use claim_fhe_job_instruction
                self.client
                    .claim_fhe_job_instruction(&self.keypair.pubkey(), job_pda, job_id)
                    .context("Failed to build FHE claim instruction")?
            }
            _ => {
                // ZK single-prover jobs use claim_job_instruction
                self.client
                    .claim_job_instruction(&self.keypair.pubkey(), job_pda)
                    .context("Failed to build claim instruction")?
            }
        };

        match self
            .client
            .send_and_confirm_transaction(&[claim_ix], &[&*self.keypair])
        {
            Ok(sig) => {
                info!("[Job {}] Claimed successfully (sig: {})", job_id, sig);
            }
            Err(e) => {
                warn!("[Job {}] Failed to claim (already claimed?): {}", job_id, e);
                return Err(e);
            }
        }

        // Verify claim succeeded
        self.verify_claim(job_pda, job_id, circuit_type).await?;

        Ok(())
    }

    /// Verify that the job was successfully claimed
    async fn verify_claim(
        &self,
        job_pda: &Pubkey,
        job_id: u64,
        circuit_type: &CircuitType,
    ) -> Result<()> {
        let job = fetch_job(&self.client.rpc_client, job_pda)
            .context("Failed to fetch job after claim")?;

        // For ZK jobs: verify we claimed it exclusively
        if let CircuitType::ZcashOrchard = circuit_type {
            if job.status != JobStatus::Claimed || job.prover != Some(self.keypair.pubkey()) {
                warn!("[Job {}] ZK job not claimed by us, aborting", job_id);
                return Err(anyhow::anyhow!("Job not claimed by us"));
            }
        }

        // For FHE jobs: verify we're in the claimed_provers list
        if let CircuitType::FheComputation(_) = circuit_type {
            let (fhe_pda, _) = self.client.get_fhe_consensus_pda(job_id);
            let fhe_data = fetch_fhe_consensus(&self.client.rpc_client, &fhe_pda)
                .context("Failed to fetch FHE consensus data")?;

            let our_pubkey = self.keypair.pubkey();
            let is_claimed = fhe_data.claimed_provers[..fhe_data.claimed_count as usize]
                .iter()
                .any(|p| *p == our_pubkey);

            if !is_claimed {
                warn!(
                    "[Job {}] We are not in the claimed_provers list, aborting",
                    job_id
                );
                return Err(anyhow::anyhow!("Not in claimed_provers list"));
            }

            // Check if we already submitted a result
            let our_index = fhe_data.claimed_provers[..fhe_data.claimed_count as usize]
                .iter()
                .position(|p| *p == our_pubkey);

            if let Some(idx) = our_index {
                if fhe_data.result_submitted[idx] {
                    info!("[Job {}] Already submitted FHE result, skipping", job_id);
                    return Err(anyhow::anyhow!("Already submitted"));
                }
            }
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
        job_id: u64,
        circuit_type: &CircuitType,
        proof_bytes: &[u8],
        witness_hash: &[u8; 32],
    ) -> Result<()> {
        match circuit_type {
            CircuitType::ZcashOrchard => {
                self.submit_zk_proof(job_pda, job_id, proof_bytes).await?;
            }
            CircuitType::FheComputation(ref op) => {
                self.submit_fhe_result(job_pda, job_id, proof_bytes, witness_hash, op)
                    .await?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported circuit type: {:?}",
                    circuit_type
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
        proof_bytes: &[u8],
    ) -> Result<()> {
        info!("[Job {}] Submitting ZK proof...", job_id);

        // Fetch job to get creator
        let job = fetch_job(&self.client.rpc_client, job_pda)
            .context("Failed to fetch job before submit")?;

        // Generate proof commitment (hash of actual proof)
        let proof_commitment = Self::generate_proof_commitment(proof_bytes);
        let proof_size = proof_bytes.len() as u32;

        // Fetch config to get protocol fee recipient
        let (config_pda, _) = self.client.get_config_pda();
        let config_account = self
            .client
            .rpc_client
            .get_account(&config_pda)
            .context("Failed to fetch config account")?;

        // Extract protocol_fee_recipient from config
        let protocol_fee_recipient = if config_account.data.len() >= 86 {
            Pubkey::try_from(&config_account.data[54..86])?
        } else {
            return Err(anyhow::anyhow!("Invalid config account"));
        };

        let submit_ix = self
            .client
            .submit_proof_instruction_with_recipient(
                &self.keypair.pubkey(),
                job_pda,
                &job.creator,
                &protocol_fee_recipient,
                proof_commitment,
                proof_size,
            )
            .context("Failed to build submit proof instruction")?;

        match self
            .client
            .send_and_confirm_transaction(&[submit_ix], &[&*self.keypair])
        {
            Ok(sig) => {
                info!(
                    "[Job {}] ZK proof submitted successfully (sig: {})",
                    job_id, sig
                );
            }
            Err(e) => {
                error!("[Job {}] Failed to submit ZK proof: {}", job_id, e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Submit FHE result
    async fn submit_fhe_result(
        &self,
        job_pda: &Pubkey,
        job_id: u64,
        proof_bytes: &[u8],
        witness_hash: &[u8; 32],
        operation: &FheOperation,
    ) -> Result<()> {
        info!("[Job {}] Submitting FHE result...", job_id);

        // Use deterministic commitment for consensus
        let result_hash =
            FheEngine::deterministic_commitment(witness_hash, operation.name(), job_id);

        info!(
            "[Job {}] FHE deterministic commitment: {} (op: {})",
            job_id,
            hex::encode(&result_hash[..8]),
            operation.name()
        );

        // Store encrypted result in witness backend
        self.upload_fhe_result(job_id, proof_bytes).await?;

        // Build SubmitFheResult instruction
        let submit_ix = self
            .client
            .submit_fhe_result_instruction(&self.keypair.pubkey(), job_pda, job_id, result_hash)
            .context("Failed to build submit FHE result instruction")?;

        match self
            .client
            .send_and_confirm_transaction(&[submit_ix], &[&*self.keypair])
        {
            Ok(sig) => {
                info!(
                    "[Job {}] FHE result submitted successfully (sig: {})",
                    job_id, sig
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
                        job_id
                    );
                } else {
                    error!("[Job {}] Failed to submit FHE result: {}", job_id, e);
                    return Err(e);
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
