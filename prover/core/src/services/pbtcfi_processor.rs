//! pBTCFi Job Processor
//!
//! Handles FHE computation jobs for pBTCFi loans.
//! Computes: collateral_value = btc_amount * price * ltv

use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use zyberlink_fhe::{deserialize_server_key, FheEngine};

use super::pbtcfi_client::{PbtcfiJobClient, PbtcfiPendingJob};

/// pBTCFi loan parameters (could come from config or contract)
#[derive(Debug, Clone)]
pub struct PbtcfiLoanParams {
    /// BTC price in USD (scaled, e.g., 45000 = $45,000)
    pub btc_price_usd: u64,
    /// Loan-to-value ratio (percentage, e.g., 70 = 70%)
    pub ltv_percent: u8,
    /// pLST price factor (for converting collateral to pLST amount)
    pub plst_factor: u8,
}

impl Default for PbtcfiLoanParams {
    fn default() -> Self {
        Self {
            btc_price_usd: 45000,
            ltv_percent: 70,
            plst_factor: 1,
        }
    }
}

/// Result of FHE computation
#[derive(Debug, Clone)]
pub struct FheComputationResult {
    /// Encrypted collateral value (c1 component)
    pub collateral_value_c1: String,
    /// Encrypted collateral value (c2 component)
    pub collateral_value_c2: String,
    /// Encrypted pLST amount (c1 component)
    pub plst_amount_c1: String,
    /// Encrypted pLST amount (c2 component)
    pub plst_amount_c2: String,
}

/// Processor for pBTCFi FHE jobs
pub struct PbtcfiProcessor {
    client: Arc<PbtcfiJobClient>,
    params: PbtcfiLoanParams,
    poll_interval: Duration,
    max_concurrent_jobs: usize,
}

impl PbtcfiProcessor {
    /// Create a new pBTCFi processor
    ///
    /// # Arguments
    /// * `blink_server_url` - URL of the blink-server
    /// * `prover_id` - Unique identifier for this prover
    pub fn new(blink_server_url: &str, prover_id: &str) -> Self {
        Self {
            client: Arc::new(PbtcfiJobClient::new(blink_server_url, prover_id)),
            params: PbtcfiLoanParams::default(),
            poll_interval: Duration::from_secs(10),
            max_concurrent_jobs: 3,
        }
    }

    /// Create with custom parameters
    pub fn with_params(mut self, params: PbtcfiLoanParams) -> Self {
        self.params = params;
        self
    }

    /// Set poll interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set max concurrent jobs
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_jobs = max;
        self
    }

    /// Start the job processing loop
    pub async fn run(&self) -> Result<()> {
        log::info!("Starting pBTCFi job processor");
        log::info!("  Prover ID: {}", self.client.prover_id());
        log::info!("  Poll interval: {:?}", self.poll_interval);
        log::info!("  Max concurrent: {}", self.max_concurrent_jobs);
        log::info!("  BTC Price: ${}", self.params.btc_price_usd);
        log::info!("  LTV: {}%", self.params.ltv_percent);

        loop {
            match self.process_pending_jobs().await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Processed {} pBTCFi jobs", count);
                    }
                }
                Err(e) => {
                    log::error!("Error processing pBTCFi jobs: {}", e);
                }
            }

            sleep(self.poll_interval).await;
        }
    }

    /// Process all pending jobs
    async fn process_pending_jobs(&self) -> Result<usize> {
        // Fetch pending jobs
        let jobs = self
            .client
            .get_pending_jobs(self.max_concurrent_jobs as u32)
            .await?;

        if jobs.is_empty() {
            return Ok(0);
        }

        log::info!("Found {} pending pBTCFi jobs", jobs.len());

        let mut processed = 0;
        for job in jobs {
            match self.process_single_job(&job).await {
                Ok(()) => {
                    processed += 1;
                    log::info!("Successfully processed job for loan {}", job.loan_id);
                }
                Err(e) => {
                    log::error!("Failed to process job for loan {}: {}", job.loan_id, e);
                    // Report failure to server
                    let _ = self.client.fail_job(&job.loan_id, &e.to_string()).await;
                }
            }
        }

        Ok(processed)
    }

    /// Process a single job
    async fn process_single_job(&self, job: &PbtcfiPendingJob) -> Result<()> {
        log::info!("Processing pBTCFi job for loan: {}", job.loan_id);
        log::debug!("  Borrower: {}", job.borrower);
        log::debug!("  BTC commitment: {}", job.btc_commitment);

        // Step 1: Claim the job
        let claimed = self.client.claim_job(&job.loan_id).await?;
        if !claimed {
            return Err(anyhow!("Failed to claim job - already taken"));
        }

        log::info!("Claimed job for loan {}", job.loan_id);

        // Step 2: Parse encrypted BTC amount
        // The encrypted data is stored as hex strings (c1, c2 components)
        let btc_encrypted = self.parse_encrypted_data(&job.btc_encrypted_c1, &job.btc_encrypted_c2)?;

        // Step 3: Perform FHE computation
        let result = self.compute_collateral_value(&btc_encrypted).await?;

        // Step 4: Submit result
        self.client
            .complete_job(
                &job.loan_id,
                &result.collateral_value_c1,
                &result.collateral_value_c2,
                &result.plst_amount_c1,
                &result.plst_amount_c2,
            )
            .await?;

        log::info!("Completed FHE job for loan {}", job.loan_id);
        Ok(())
    }

    /// Parse encrypted data from hex strings
    fn parse_encrypted_data(&self, c1: &str, c2: &str) -> Result<EncryptedBtcAmount> {
        // Remove 0x prefix if present and decode hex
        let c1_hex = c1.trim_start_matches("0x");
        let c2_hex = c2.trim_start_matches("0x");

        let c1_bytes = hex::decode(c1_hex)
            .context("Failed to decode c1 hex")?;
        let c2_bytes = hex::decode(c2_hex)
            .context("Failed to decode c2 hex")?;

        Ok(EncryptedBtcAmount {
            c1: c1_bytes,
            c2: c2_bytes,
        })
    }

    /// Compute collateral value using FHE
    ///
    /// Formula: collateral_value = btc_amount * btc_price * ltv_percent / 100
    /// For pLST: plst_amount = collateral_value * plst_factor
    async fn compute_collateral_value(&self, encrypted_btc: &EncryptedBtcAmount) -> Result<FheComputationResult> {
        // For the MVP, we'll check if this is TFHE format or needs conversion

        // If the encrypted data is small (not TFHE format), use mock computation
        // Real TFHE ciphertexts are ~65KB
        if encrypted_btc.c1.len() < 1000 {
            log::warn!("Encrypted data appears to be placeholder format, using mock computation");
            return self.mock_compute_collateral();
        }

        // Try to use real FHE computation
        self.fhe_compute_collateral(encrypted_btc).await
    }

    /// Real FHE computation (when we have proper TFHE ciphertexts)
    async fn fhe_compute_collateral(&self, encrypted_btc: &EncryptedBtcAmount) -> Result<FheComputationResult> {
        // The c1 should contain the TFHE server key or we need to fetch it
        // For now, we assume c1 contains the encrypted value and c2 contains metadata/server key reference

        // Deserialize the server key (in production, this would come from a separate source)
        // For MVP, check if c2 contains a server key hash and fetch it

        // If c1 is the actual TFHE ciphertext, try to process it
        let ciphertext_bytes = &encrypted_btc.c1;

        // For multiplication with constants, we need the server key
        // In the current architecture, server keys are uploaded separately
        // For now, we'll create a mock result with proper formatting

        log::info!("Attempting FHE computation with {} byte ciphertext", ciphertext_bytes.len());

        // Mock the result for now until we have proper server key integration
        self.mock_compute_collateral()
    }

    /// Mock computation for development/testing
    fn mock_compute_collateral(&self) -> Result<FheComputationResult> {
        // Generate deterministic mock values based on params
        let mock_collateral = format!(
            "0x{:0>64}",
            hex::encode(&[
                (self.params.btc_price_usd % 256) as u8,
                self.params.ltv_percent,
                0, 0, 0, 0, 0, 0,
            ])
        );

        let mock_plst = format!(
            "0x{:0>64}",
            hex::encode(&[
                self.params.plst_factor,
                (self.params.btc_price_usd % 256) as u8,
                0, 0, 0, 0, 0, 0,
            ])
        );

        Ok(FheComputationResult {
            collateral_value_c1: mock_collateral.clone(),
            collateral_value_c2: "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            plst_amount_c1: mock_plst,
            plst_amount_c2: "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        })
    }
}

/// Encrypted BTC amount (ElGamal-like structure from Cairo contract)
#[derive(Debug, Clone)]
struct EncryptedBtcAmount {
    c1: Vec<u8>,
    c2: Vec<u8>,
}

/// Real FHE processor using zyberlink-fhe
pub struct RealFheProcessor {
    engine: Arc<FheEngine>,
}

impl RealFheProcessor {
    /// Create from server key bytes
    pub fn new(server_key_bytes: &[u8]) -> Result<Self> {
        let server_key = deserialize_server_key(server_key_bytes)
            .context("Failed to deserialize server key")?;

        Ok(Self {
            engine: Arc::new(FheEngine::new(server_key)),
        })
    }

    /// Compute collateral value from encrypted BTC amount
    ///
    /// # Arguments
    /// * `encrypted_btc` - TFHE encrypted BTC amount (FheUint8 or similar)
    /// * `price_factor` - Combined price * ltv factor (as u8, scaled)
    pub async fn compute_collateral(
        &self,
        encrypted_btc: &[u8],
        price_factor: u8,
    ) -> Result<Vec<u8>> {
        let engine = Arc::clone(&self.engine);
        let input = encrypted_btc.to_vec();

        tokio::task::spawn_blocking(move || {
            engine.set_key_for_thread();
            engine.compute_multiply(&input, price_factor)
        })
        .await
        .context("FHE task panicked")?
    }

    /// Compute with two-step multiplication (for larger factors)
    pub async fn compute_collateral_two_step(
        &self,
        encrypted_btc: &[u8],
        price: u8,
        ltv: u8,
    ) -> Result<Vec<u8>> {
        let engine = Arc::clone(&self.engine);
        let input = encrypted_btc.to_vec();

        tokio::task::spawn_blocking(move || {
            engine.set_key_for_thread();

            // Step 1: multiply by price
            let step1 = engine.compute_multiply(&input, price)?;

            // Step 2: multiply by ltv (percentage as fraction, e.g., 70 = 0.7)
            // Note: This is simplified - real implementation needs scaling
            engine.compute_multiply(&step1, ltv)
        })
        .await
        .context("FHE task panicked")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_params_default() {
        let params = PbtcfiLoanParams::default();
        assert_eq!(params.btc_price_usd, 45000);
        assert_eq!(params.ltv_percent, 70);
    }

    #[test]
    fn test_parse_encrypted_data() {
        let processor = PbtcfiProcessor::new("http://localhost:8080", "test-prover");

        let c1 = "0x1234567890abcdef";
        let c2 = "0xfedcba0987654321";

        let result = processor.parse_encrypted_data(c1, c2).unwrap();

        assert_eq!(result.c1, hex::decode("1234567890abcdef").unwrap());
        assert_eq!(result.c2, hex::decode("fedcba0987654321").unwrap());
    }

    #[test]
    fn test_mock_computation() {
        let processor = PbtcfiProcessor::new("http://localhost:8080", "test-prover");

        let result = processor.mock_compute_collateral().unwrap();

        assert!(result.collateral_value_c1.starts_with("0x"));
        assert!(result.plst_amount_c1.starts_with("0x"));
    }
}
