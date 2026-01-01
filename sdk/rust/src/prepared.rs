//! # PreparedOperation - Layer 2 API
//!
//! Operations that can be inspected, simulated, and composed before execution.
//!
//! ## Example
//!
//! ```ignore
//! // Prepare an operation
//! let prep = zyber.prepare_sum(&[100, 200, 300]).await?;
//!
//! // Inspect before spending SOL
//! println!("Estimated cost: {} lamports", prep.estimate_cost());
//! println!("Job PDA: {}", prep.job_pda());
//!
//! // Simulate to verify it will work
//! prep.simulate().await?;
//!
//! // Execute when ready
//! let result = prep.execute().await?;
//! ```
//!
//! ## Composition with other instructions
//!
//! ```ignore
//! // Get the instruction for composing with other transactions
//! let ix = prep.instruction();
//!
//! // Combine with other instructions
//! let tx = Transaction::new_signed_with_payer(
//!     &[some_other_ix, ix, another_ix],
//!     Some(&payer),
//!     &[&keypair],
//!     blockhash,
//! );
//! ```

use anyhow::{anyhow, Context, Result};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::Duration;

use crate::client::MarketplaceClient;
use crate::config::NetworkConfig;
use zyberlink_types::{FheConsensusConfig, FheOperation};

/// A prepared FHE operation ready for inspection, simulation, or execution
pub struct PreparedOperation {
    /// Operation type
    pub(crate) operation: FheOperation,
    /// Encrypted witness data
    pub(crate) witness_data: Vec<u8>,
    /// Witness commitment (hash)
    pub(crate) witness_commitment: [u8; 32],
    /// Job ID to use
    pub(crate) job_id: u64,
    /// FHE configuration
    pub(crate) fhe_config: FheConsensusConfig,
    /// Price in lamports
    pub(crate) price_lamports: u64,
    /// Timeout duration
    pub(crate) timeout: Duration,
    /// Network configuration
    pub(crate) config: NetworkConfig,
    /// Low-level client
    pub(crate) client: Arc<MarketplaceClient>,
    /// User keypair
    pub(crate) keypair: Arc<solana_sdk::signature::Keypair>,
    /// HTTP client for backend
    pub(crate) http: reqwest::Client,
    /// Client key for decryption
    pub(crate) client_key: Arc<tfhe::ClientKey>,
    /// Cached instruction (computed lazily)
    cached_instruction: Option<Instruction>,
}

/// Cost breakdown for an operation
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Job creation cost (rent + compute)
    pub job_creation_lamports: u64,
    /// Price offered to provers
    pub prover_payment_lamports: u64,
    /// Estimated transaction fee
    pub tx_fee_lamports: u64,
    /// Total estimated cost
    pub total_lamports: u64,
}

impl CostEstimate {
    /// Convert total to SOL
    pub fn total_sol(&self) -> f64 {
        self.total_lamports as f64 / 1_000_000_000.0
    }
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the simulation succeeded
    pub success: bool,
    /// Compute units consumed
    pub compute_units: u64,
    /// Logs from simulation
    pub logs: Vec<String>,
    /// Error message if failed
    pub error: Option<String>,
}

/// PDAs involved in this operation
#[derive(Debug, Clone)]
pub struct OperationPdas {
    /// Job PDA
    pub job: Pubkey,
    /// Escrow PDA
    pub escrow: Pubkey,
    /// FHE consensus PDA
    pub fhe_consensus: Pubkey,
    /// Config PDA
    pub config: Pubkey,
}

impl PreparedOperation {
    /// Create a new prepared operation (internal constructor)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        operation: FheOperation,
        witness_data: Vec<u8>,
        witness_commitment: [u8; 32],
        job_id: u64,
        fhe_config: FheConsensusConfig,
        price_lamports: u64,
        timeout: Duration,
        config: NetworkConfig,
        client: Arc<MarketplaceClient>,
        keypair: Arc<solana_sdk::signature::Keypair>,
        http: reqwest::Client,
        client_key: Arc<tfhe::ClientKey>,
    ) -> Self {
        Self {
            operation,
            witness_data,
            witness_commitment,
            job_id,
            fhe_config,
            price_lamports,
            timeout,
            config,
            client,
            keypair,
            http,
            client_key,
            cached_instruction: None,
        }
    }

    // =========================================================================
    // Inspection Methods
    // =========================================================================

    /// Get the operation type
    pub fn operation(&self) -> &FheOperation {
        &self.operation
    }

    /// Get the job ID
    pub fn job_id(&self) -> u64 {
        self.job_id
    }

    /// Get PDAs involved in this operation
    pub fn pdas(&self) -> OperationPdas {
        let creator = self.keypair.pubkey();
        let (job, _) = self.client.get_job_pda(&creator, self.job_id);
        let (escrow, _) = self.client.get_escrow_pda(&job);
        let (fhe_consensus, _) = self.client.get_fhe_consensus_pda(self.job_id);
        let (config, _) = self.client.get_config_pda();

        OperationPdas {
            job,
            escrow,
            fhe_consensus,
            config,
        }
    }

    /// Get the job PDA (convenience method)
    pub fn job_pda(&self) -> Pubkey {
        self.pdas().job
    }

    /// Estimate the cost of this operation
    pub fn estimate_cost(&self) -> CostEstimate {
        // Base rent exemption for job account (~2000 bytes)
        let job_creation_lamports = 15_000_000; // ~0.015 SOL rent exempt minimum

        // Transaction fee (5000 lamports per signature, typically 1-2 signatures)
        let tx_fee_lamports = 10_000;

        let total = job_creation_lamports + self.price_lamports + tx_fee_lamports;

        CostEstimate {
            job_creation_lamports,
            prover_payment_lamports: self.price_lamports,
            tx_fee_lamports,
            total_lamports: total,
        }
    }

    /// Get the witness data size in bytes
    pub fn witness_size(&self) -> usize {
        self.witness_data.len()
    }

    /// Get the witness commitment
    pub fn witness_commitment(&self) -> &[u8; 32] {
        &self.witness_commitment
    }

    /// Get the number of required provers
    pub fn required_provers(&self) -> u8 {
        self.fhe_config.required_provers
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    // =========================================================================
    // Instruction Building
    // =========================================================================

    /// Get the Solana instruction for this operation
    ///
    /// Use this when you want to combine this instruction with others
    /// in a single transaction.
    pub fn instruction(&mut self) -> Result<Instruction> {
        if let Some(ref ix) = self.cached_instruction {
            return Ok(ix.clone());
        }

        let ix = self.client.create_job_instruction(
            &self.keypair.pubkey(),
            self.job_id,
            zyberlink_types::CircuitType::FheComputation(self.fhe_config.operation.clone()),
            self.witness_commitment,
            self.witness_data.len() as u32,
            self.price_lamports,
            self.timeout.as_secs() as i64,
            Some(self.fhe_config.clone()),
        )?;

        self.cached_instruction = Some(ix.clone());
        Ok(ix)
    }

    /// Get the accounts needed for this instruction
    pub fn accounts(&mut self) -> Result<Vec<solana_sdk::instruction::AccountMeta>> {
        let ix = self.instruction()?;
        Ok(ix.accounts)
    }

    // =========================================================================
    // Simulation
    // =========================================================================

    /// Simulate the operation without submitting to the network
    ///
    /// This performs a dry-run to verify the transaction would succeed,
    /// without spending any SOL.
    pub async fn simulate(&mut self) -> Result<SimulationResult> {
        let ix = self.instruction()?;
        let recent_blockhash = self.client.rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );

        let result = self.client.rpc_client.simulate_transaction(&tx)?;

        let success = result.value.err.is_none();
        let logs = result.value.logs.unwrap_or_default();
        let compute_units = result.value.units_consumed.unwrap_or(0);
        let error = result.value.err.map(|e| format!("{:?}", e));

        Ok(SimulationResult {
            success,
            compute_units,
            logs,
            error,
        })
    }

    // =========================================================================
    // Execution
    // =========================================================================

    /// Execute the operation and wait for the result
    ///
    /// This performs the full flow:
    /// 1. Upload witness to backend
    /// 2. Create job on-chain
    /// 3. Poll for result
    /// 4. Decrypt and return
    pub async fn execute(mut self) -> Result<u64> {
        // Upload witness to backend
        let witness_commitment = self.upload_witness().await?;

        // Verify commitment matches
        if witness_commitment != self.witness_commitment {
            return Err(anyhow!(
                "Witness commitment mismatch: expected {:?}, got {:?}",
                hex::encode(self.witness_commitment),
                hex::encode(witness_commitment)
            ));
        }

        // Create job on-chain
        let ix = self.instruction()?;
        let recent_blockhash = self.client.rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );

        self.client
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&tx)
            .context("Failed to create job on-chain")?;

        // Poll for result
        let encrypted_result = self.poll_result().await?;

        // Decrypt result
        let result: u8 = zyberlink_fhe::decrypt_result(&encrypted_result, &self.client_key)
            .context("Failed to decrypt result")?;

        Ok(result as u64)
    }

    /// Execute only the on-chain transaction, don't wait for result
    ///
    /// Returns the transaction signature. Use this when you want to
    /// handle polling separately.
    pub async fn submit(mut self) -> Result<Signature> {
        // Upload witness to backend
        self.upload_witness().await?;

        // Create job on-chain
        let ix = self.instruction()?;
        let recent_blockhash = self.client.rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );

        let signature = self.client
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&tx)
            .context("Failed to submit job transaction")?;

        Ok(signature)
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Upload witness data to backend
    async fn upload_witness(&self) -> Result<[u8; 32]> {
        let url = format!("{}/api/witness", self.config.backend_url);

        let response = self
            .http
            .post(&url)
            .body(self.witness_data.clone())
            .send()
            .await
            .context("Failed to upload witness")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Witness upload failed: {}",
                response.status()
            ));
        }

        let result: serde_json::Value = response.json().await?;
        let commitment_hex = result["commitment"]
            .as_str()
            .ok_or_else(|| anyhow!("No commitment in response"))?;

        let commitment_bytes = hex::decode(commitment_hex)?;
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&commitment_bytes[..32]);

        Ok(commitment)
    }

    /// Poll for job result
    async fn poll_result(&self) -> Result<Vec<u8>> {
        let url = format!("{}/api/jobs/{}/result", self.config.backend_url, self.job_id);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                return Err(anyhow!("Job {} timed out after {:?}", self.job_id, self.timeout));
            }

            let response = self.http.get(&url).send().await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if let Some(status) = result["status"].as_str() {
                    match status {
                        "completed" => {
                            let encrypted_result = result["encrypted_result"]
                                .as_str()
                                .ok_or_else(|| anyhow!("No encrypted_result in response"))?;
                            use base64::Engine;
                            return Ok(base64::engine::general_purpose::STANDARD.decode(encrypted_result)?);
                        }
                        "failed" => {
                            let error = result["error"].as_str().unwrap_or("Unknown error");
                            return Err(anyhow!("Job failed: {}", error));
                        }
                        _ => {
                            // Still processing, wait and retry
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimate() {
        let estimate = CostEstimate {
            job_creation_lamports: 15_000_000,
            prover_payment_lamports: 10_000_000,
            tx_fee_lamports: 10_000,
            total_lamports: 25_010_000,
        };

        assert!(estimate.total_sol() > 0.025);
        assert!(estimate.total_sol() < 0.026);
    }
}
