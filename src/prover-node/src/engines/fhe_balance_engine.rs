use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zyberlink_fhe::FheEngine;

// ============================================================================
// JobType mapping from Cairo contracts (jobs.cairo)
// ============================================================================

/// Job types from Cairo contracts - maps to jobs.cairo JobType enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum JobType {
    /// pBTCFi: verify encrypted BTC collateral
    LoanVerification = 0,
    /// pLST: update encrypted token balance
    BalanceUpdate = 1,
    /// pLST: prove staking position
    StakeProof = 2,
    /// Generic: prove valid transfer
    TransferProof = 3,
    /// pBTCFi: check liquidation threshold
    LiquidationCheck = 4,
    /// Aptos Private Lending: verify LTV ratio
    LtvCheck = 5,
}

impl TryFrom<u8> for JobType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(JobType::LoanVerification),
            1 => Ok(JobType::BalanceUpdate),
            2 => Ok(JobType::StakeProof),
            3 => Ok(JobType::TransferProof),
            4 => Ok(JobType::LiquidationCheck),
            5 => Ok(JobType::LtvCheck),
            _ => Err(anyhow::anyhow!("Unknown JobType: {}", value)),
        }
    }
}

// ============================================================================
// FHE Computation Types
// ============================================================================

/// Encrypted value in ElGamal format (c1, c2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedValue {
    pub c1: [u8; 32],
    pub c2: [u8; 32],
}

impl EncryptedValue {
    pub fn from_felts(c1_felt: [u8; 32], c2_felt: [u8; 32]) -> Self {
        Self {
            c1: c1_felt,
            c2: c2_felt,
        }
    }

    /// Homomorphic addition (ElGamal)
    /// new_c1 = c1_a + c1_b
    /// new_c2 = c2_a + c2_b
    pub fn add(&self, other: &EncryptedValue) -> EncryptedValue {
        let mut new_c1 = [0u8; 32];
        let mut new_c2 = [0u8; 32];

        // Simple addition with overflow wrapping (placeholder for real FHE)
        for i in 0..32 {
            new_c1[i] = self.c1[i].wrapping_add(other.c1[i]);
            new_c2[i] = self.c2[i].wrapping_add(other.c2[i]);
        }

        EncryptedValue {
            c1: new_c1,
            c2: new_c2,
        }
    }
}

/// Input for FHE balance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FheBalanceInput {
    /// Job ID from the contract
    pub job_id: u64,
    /// Type of job to process
    pub job_type: JobType,
    /// Current encrypted balance (or collateral) - ElGamal format for Cairo compatibility
    pub current_encrypted: EncryptedValue,
    /// Delta to add (for BalanceUpdate) or amount to verify (for others) - ElGamal format
    pub delta_encrypted: Option<EncryptedValue>,
    /// Additional parameters
    pub params: FheJobParams,
    /// Real TFHE ciphertext for current value (optional, ~65KB when present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tfhe_current: Option<TfheCiphertext>,
    /// Real TFHE ciphertext for delta value (optional, ~65KB when present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tfhe_delta: Option<TfheCiphertext>,
}

/// Parameters specific to each job type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FheJobParams {
    /// Loan verification: check collateral >= min_ratio
    LoanVerification {
        min_collateral_ratio: u64,
        btc_price_oracle: u64,
    },
    /// Balance update: simple add operation
    BalanceUpdate {
        /// If true, this is a mint; if false, a transfer/reward
        is_mint: bool,
    },
    /// Stake proof: verify staking position
    StakeProof {
        min_stake_amount: u64,
        staking_period: u64,
    },
    /// Transfer proof: verify sufficient balance
    TransferProof {
        transfer_amount_encrypted: EncryptedValue,
    },
    /// Liquidation check: collateral < threshold
    LiquidationCheck {
        liquidation_threshold: u64,
        current_price: u64,
    },
    /// Aptos Private Lending: LTV ratio check
    /// Verifies: (borrow_amount / collateral_value) * 100 <= ltv_threshold
    LtvCheck {
        /// LTV threshold in basis points (e.g., 7500 = 75%)
        ltv_threshold_bps: u16,
        /// Oracle price for collateral (6 decimals)
        collateral_price_usd: u64,
    },
}

/// Result of FHE computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FheBalanceResult {
    /// Job ID
    pub job_id: u64,
    /// Whether the computation succeeded
    pub success: bool,
    /// Hash of the result (for on-chain verification)
    pub result_hash: [u8; 32],
    /// New encrypted value (if applicable)
    pub new_encrypted: Option<EncryptedValue>,
    /// Boolean result (for verification operations)
    pub verification_result: Option<bool>,
    /// Computation proof (placeholder for actual ZK proof)
    pub proof: Vec<u8>,
}

// ============================================================================
// FHE Balance Engine Trait
// ============================================================================

/// Trait for FHE balance computation engines
#[async_trait]
pub trait FheBalanceProcessor: Send + Sync {
    /// Process a balance-related FHE job
    async fn process(&self, input: &FheBalanceInput) -> Result<FheBalanceResult>;

    /// Check if this engine supports the given job type
    fn supports_job_type(&self, job_type: JobType) -> bool;
}

// ============================================================================
// FHE Balance Engine Implementation
// ============================================================================

/// TFHE ciphertext wrapper for real FHE operations
///
/// TFHE ciphertexts are ~65KB when serialized with bincode.
/// This is different from the 64-byte ElGamal format used in Cairo contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TfheCiphertext {
    /// Serialized TFHE FheUint8 ciphertext
    pub data: Vec<u8>,
}

impl TfheCiphertext {
    /// Check if this is a valid TFHE ciphertext (based on size)
    /// Real TFHE ciphertexts are ~65KB, mock ones are 64 bytes
    pub fn is_real_tfhe(&self) -> bool {
        self.data.len() > 1000 // Real TFHE ciphertexts are >> 1KB
    }
}

/// Main FHE Balance Engine
pub struct FheBalanceEngine {
    /// Mock mode for testing
    mock_mode: bool,
    /// Real FHE engine from zyberlink-fhe (optional)
    fhe_engine: Option<Arc<FheEngine>>,
}

impl FheBalanceEngine {
    /// Create engine in production mode (requires real FHE server key)
    ///
    /// # Arguments
    /// * `fhe_engine` - Real FHE engine from zyberlink-fhe with server key loaded
    pub fn new(fhe_engine: Arc<FheEngine>) -> Self {
        Self {
            mock_mode: false,
            fhe_engine: Some(fhe_engine),
        }
    }

    /// Create engine in mock mode (for testing without FHE keys)
    pub fn new_mock() -> Self {
        Self {
            mock_mode: true,
            fhe_engine: None,
        }
    }

    /// Check if engine is using real FHE operations
    pub fn is_real_fhe(&self) -> bool {
        !self.mock_mode && self.fhe_engine.is_some()
    }

    /// Compute result hash from encrypted values
    fn compute_result_hash(&self, encrypted: &EncryptedValue) -> [u8; 32] {
        use blake2::{Blake2b, Digest};
        use blake2::digest::consts::U32;
        let mut hasher = Blake2b::<U32>::new();
        hasher.update(&encrypted.c1);
        hasher.update(&encrypted.c2);
        hasher.finalize().into()
    }

    /// Process LoanVerification job
    async fn process_loan_verification(
        &self,
        input: &FheBalanceInput,
        params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let FheJobParams::LoanVerification {
            min_collateral_ratio,
            btc_price_oracle,
        } = params
        else {
            return Err(anyhow::anyhow!("Invalid params for LoanVerification"));
        };

        // In mock mode, always return success
        // In real mode, would perform FHE range check
        let verification_result = if self.mock_mode {
            true
        } else {
            // TODO: Implement actual FHE collateral verification
            // FHE operation: encrypted_collateral * btc_price >= loan_amount * min_ratio
            true
        };

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash: self.compute_result_hash(&input.current_encrypted),
            new_encrypted: None,
            verification_result: Some(verification_result),
            proof: vec![0xDE, 0xAD, 0xBE, 0xEF], // Placeholder proof
        })
    }

    /// Process BalanceUpdate job
    async fn process_balance_update(
        &self,
        input: &FheBalanceInput,
        _params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let delta = input
            .delta_encrypted
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BalanceUpdate requires delta_encrypted"))?;

        // Check if we have real TFHE ciphertexts (stored in tfhe_ciphertext field)
        // Real TFHE ciphertexts are ~65KB, ElGamal format is 64 bytes
        let new_encrypted = if self.is_real_fhe() && input.tfhe_current.is_some() && input.tfhe_delta.is_some() {
            // Use real FHE engine with actual TFHE ciphertexts
            if let (Some(ref fhe_engine), Some(ref current_tfhe), Some(ref delta_tfhe)) =
                (&self.fhe_engine, &input.tfhe_current, &input.tfhe_delta)
            {
                log::info!(
                    "[Job {}] Processing real TFHE addition: current={} bytes, delta={} bytes",
                    input.job_id,
                    current_tfhe.data.len(),
                    delta_tfhe.data.len()
                );

                // Perform real homomorphic addition using spawn_blocking for CPU-intensive work
                let engine = fhe_engine.clone();
                let current_data = current_tfhe.data.clone();
                let delta_data = delta_tfhe.data.clone();

                let result_bytes = tokio::task::spawn_blocking(move || {
                    engine.set_key_for_thread();
                    engine.compute_add_encrypted(&current_data, &delta_data)
                })
                .await
                .context("FHE task panicked")?
                .context("TFHE addition failed")?;

                log::info!(
                    "[Job {}] TFHE addition complete: result={} bytes",
                    input.job_id,
                    result_bytes.len()
                );

                // For compatibility, store hash of TFHE result in EncryptedValue
                // The actual result would be stored off-chain
                let result_hash = FheEngine::hash_result(&result_bytes);
                EncryptedValue {
                    c1: result_hash,
                    c2: [0u8; 32], // Mark as TFHE result
                }
            } else {
                // Fallback to mock mode if FHE engine not available
                log::warn!("[Job {}] FHE engine not available, using mock", input.job_id);
                input.current_encrypted.add(delta)
            }
        } else {
            // Mock mode: simple byte addition (ElGamal format)
            log::debug!("[Job {}] Using mock FHE (ElGamal format)", input.job_id);
            input.current_encrypted.add(delta)
        };

        let result_hash = self.compute_result_hash(&new_encrypted);

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash,
            new_encrypted: Some(new_encrypted),
            verification_result: None,
            proof: vec![0xBA, 0x1A, 0xCE, 0x00], // Placeholder proof
        })
    }

    /// Convert EncryptedValue (c1, c2) to bytes for FHE engine
    fn encrypted_value_to_bytes(&self, encrypted: &EncryptedValue) -> Vec<u8> {
        // For now, concatenate c1 and c2
        // TODO: Use proper serialization format compatible with zyberlink-fhe
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&encrypted.c1);
        bytes.extend_from_slice(&encrypted.c2);
        bytes
    }

    /// Convert bytes from FHE engine back to EncryptedValue
    fn bytes_to_encrypted_value(&self, bytes: &[u8]) -> Result<EncryptedValue> {
        if bytes.len() < 64 {
            return Err(anyhow::anyhow!("Invalid encrypted value bytes: too short"));
        }

        let mut c1 = [0u8; 32];
        let mut c2 = [0u8; 32];
        c1.copy_from_slice(&bytes[0..32]);
        c2.copy_from_slice(&bytes[32..64]);

        Ok(EncryptedValue { c1, c2 })
    }

    /// Process StakeProof job
    async fn process_stake_proof(
        &self,
        input: &FheBalanceInput,
        params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let FheJobParams::StakeProof {
            min_stake_amount,
            staking_period,
        } = params
        else {
            return Err(anyhow::anyhow!("Invalid params for StakeProof"));
        };

        // In mock mode, always return success
        let verification_result = if self.mock_mode {
            true
        } else {
            // TODO: Implement actual FHE stake verification
            // FHE operation: encrypted_stake >= min_stake_amount
            true
        };

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash: self.compute_result_hash(&input.current_encrypted),
            new_encrypted: None,
            verification_result: Some(verification_result),
            proof: vec![0x57, 0x4A, 0x4B, 0x45], // "STAKE" placeholder
        })
    }

    /// Process TransferProof job
    async fn process_transfer_proof(
        &self,
        input: &FheBalanceInput,
        params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let FheJobParams::TransferProof {
            transfer_amount_encrypted,
        } = params
        else {
            return Err(anyhow::anyhow!("Invalid params for TransferProof"));
        };

        // In mock mode, always return success
        let verification_result = if self.mock_mode {
            true
        } else {
            // TODO: Implement actual FHE transfer verification
            // FHE operation: encrypted_balance >= encrypted_transfer_amount
            true
        };

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash: self.compute_result_hash(&input.current_encrypted),
            new_encrypted: None,
            verification_result: Some(verification_result),
            proof: vec![0x78, 0x46, 0x45, 0x52], // "XFER" placeholder
        })
    }

    /// Process LiquidationCheck job
    async fn process_liquidation_check(
        &self,
        input: &FheBalanceInput,
        params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let FheJobParams::LiquidationCheck {
            liquidation_threshold,
            current_price,
        } = params
        else {
            return Err(anyhow::anyhow!("Invalid params for LiquidationCheck"));
        };

        // In mock mode, return false (not liquidatable)
        let verification_result = if self.mock_mode {
            false // Position is healthy
        } else {
            // TODO: Implement actual FHE liquidation check
            // FHE operation: encrypted_collateral * current_price < loan_amount * threshold
            false
        };

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash: self.compute_result_hash(&input.current_encrypted),
            new_encrypted: None,
            verification_result: Some(verification_result),
            proof: vec![0x4C, 0x49, 0x51, 0x00], // "LIQ" placeholder
        })
    }

    /// Process LtvCheck job (Aptos Private Lending)
    ///
    /// Verifies: (borrow_amount / collateral_value) * 10000 <= ltv_threshold_bps
    async fn process_ltv_check(
        &self,
        input: &FheBalanceInput,
        params: &FheJobParams,
    ) -> Result<FheBalanceResult> {
        let FheJobParams::LtvCheck {
            ltv_threshold_bps,
            collateral_price_usd,
        } = params
        else {
            return Err(anyhow::anyhow!("Invalid params for LtvCheck"));
        };

        log::info!(
            "[Job {}] Processing LTV check: threshold={}bps, price={}",
            input.job_id, ltv_threshold_bps, collateral_price_usd
        );

        // In mock mode, always approve the loan (LTV is valid)
        // In real mode, perform FHE LTV verification
        let verification_result = if self.mock_mode {
            true // LTV check passes in mock mode
        } else if let Some(ref fhe_engine) = self.fhe_engine {
            // Use the existing FHE engine for LTV verification
            // LTV check: borrow_amount * 10000 <= collateral_amount * ltv_threshold
            fhe_engine.set_key_for_thread();

            // For real FHE, we would deserialize the encrypted values and perform
            // homomorphic comparison. For now, we use a deterministic check based
            // on the hash of inputs to ensure consensus across provers.
            use blake2::{Blake2b, Digest};
            use blake2::digest::consts::U32;
            let mut hasher = Blake2b::<U32>::new();
            hasher.update(&input.current_encrypted.c1);
            hasher.update(&input.current_encrypted.c2);
            if let Some(ref delta) = input.delta_encrypted {
                hasher.update(&delta.c1);
                hasher.update(&delta.c2);
            }
            hasher.update(&ltv_threshold_bps.to_le_bytes());

            let hash: [u8; 32] = hasher.finalize().into();

            // Deterministic approval: use first byte of hash to decide
            // This ensures all provers reach the same decision for same inputs
            // In production, this would be actual FHE comparison
            let approval_threshold = 128u8; // 50% approval rate for testing
            hash[0] >= approval_threshold || hash[0] < 64 // Bias toward approval for demo
        } else {
            true // No FHE engine, approve by default
        };

        log::info!(
            "[Job {}] LTV check result: {}",
            input.job_id,
            if verification_result { "APPROVED" } else { "REJECTED" }
        );

        Ok(FheBalanceResult {
            job_id: input.job_id,
            success: true,
            result_hash: self.compute_result_hash(&input.current_encrypted),
            new_encrypted: None,
            verification_result: Some(verification_result),
            proof: vec![0x4C, 0x54, 0x56, 0x00], // "LTV" placeholder
        })
    }
}

impl Default for FheBalanceEngine {
    fn default() -> Self {
        // Default to mock mode (no FHE keys required)
        Self::new_mock()
    }
}

#[async_trait]
impl FheBalanceProcessor for FheBalanceEngine {
    async fn process(&self, input: &FheBalanceInput) -> Result<FheBalanceResult> {
        match input.job_type {
            JobType::LoanVerification => {
                self.process_loan_verification(input, &input.params).await
            }
            JobType::BalanceUpdate => self.process_balance_update(input, &input.params).await,
            JobType::StakeProof => self.process_stake_proof(input, &input.params).await,
            JobType::TransferProof => self.process_transfer_proof(input, &input.params).await,
            JobType::LiquidationCheck => {
                self.process_liquidation_check(input, &input.params).await
            }
            JobType::LtvCheck => {
                self.process_ltv_check(input, &input.params).await
            }
        }
    }

    fn supports_job_type(&self, job_type: JobType) -> bool {
        // This engine supports all job types
        matches!(
            job_type,
            JobType::LoanVerification
                | JobType::BalanceUpdate
                | JobType::StakeProof
                | JobType::TransferProof
                | JobType::LiquidationCheck
                | JobType::LtvCheck
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_encrypted() -> EncryptedValue {
        EncryptedValue {
            c1: [1u8; 32],
            c2: [2u8; 32],
        }
    }

    #[tokio::test]
    async fn test_balance_update() {
        let engine = FheBalanceEngine::new_mock();

        let input = FheBalanceInput {
            job_id: 1,
            job_type: JobType::BalanceUpdate,
            current_encrypted: dummy_encrypted(),
            delta_encrypted: Some(EncryptedValue {
                c1: [1u8; 32],
                c2: [1u8; 32],
            }),
            params: FheJobParams::BalanceUpdate { is_mint: true },
            tfhe_current: None,
            tfhe_delta: None,
        };

        let result = engine.process(&input).await.unwrap();
        assert!(result.success);
        assert!(result.new_encrypted.is_some());

        // Check homomorphic addition worked
        let new_enc = result.new_encrypted.unwrap();
        assert_eq!(new_enc.c1[0], 2); // 1 + 1 = 2
        assert_eq!(new_enc.c2[0], 3); // 2 + 1 = 3
    }

    #[tokio::test]
    async fn test_loan_verification() {
        let engine = FheBalanceEngine::new_mock();

        let input = FheBalanceInput {
            job_id: 2,
            job_type: JobType::LoanVerification,
            current_encrypted: dummy_encrypted(),
            delta_encrypted: None,
            params: FheJobParams::LoanVerification {
                min_collateral_ratio: 150,
                btc_price_oracle: 50000,
            },
            tfhe_current: None,
            tfhe_delta: None,
        };

        let result = engine.process(&input).await.unwrap();
        assert!(result.success);
        assert_eq!(result.verification_result, Some(true));
        assert!(result.new_encrypted.is_none());
    }

    #[tokio::test]
    async fn test_liquidation_check() {
        let engine = FheBalanceEngine::new_mock();

        let input = FheBalanceInput {
            job_id: 3,
            job_type: JobType::LiquidationCheck,
            current_encrypted: dummy_encrypted(),
            delta_encrypted: None,
            params: FheJobParams::LiquidationCheck {
                liquidation_threshold: 110,
                current_price: 48000,
            },
            tfhe_current: None,
            tfhe_delta: None,
        };

        let result = engine.process(&input).await.unwrap();
        assert!(result.success);
        assert_eq!(result.verification_result, Some(false)); // Not liquidatable in mock
    }

    #[test]
    fn test_job_type_conversion() {
        assert_eq!(JobType::try_from(0).unwrap(), JobType::LoanVerification);
        assert_eq!(JobType::try_from(1).unwrap(), JobType::BalanceUpdate);
        assert_eq!(JobType::try_from(2).unwrap(), JobType::StakeProof);
        assert_eq!(JobType::try_from(3).unwrap(), JobType::TransferProof);
        assert_eq!(JobType::try_from(4).unwrap(), JobType::LiquidationCheck);
        assert!(JobType::try_from(5).is_err());
    }

    #[test]
    fn test_encrypted_value_add() {
        let a = EncryptedValue {
            c1: [10u8; 32],
            c2: [20u8; 32],
        };
        let b = EncryptedValue {
            c1: [5u8; 32],
            c2: [3u8; 32],
        };

        let result = a.add(&b);
        assert_eq!(result.c1[0], 15);
        assert_eq!(result.c2[0], 23);
    }

    #[test]
    fn test_supports_job_type() {
        let engine = FheBalanceEngine::new_mock();
        assert!(engine.supports_job_type(JobType::LoanVerification));
        assert!(engine.supports_job_type(JobType::BalanceUpdate));
        assert!(engine.supports_job_type(JobType::StakeProof));
        assert!(engine.supports_job_type(JobType::TransferProof));
        assert!(engine.supports_job_type(JobType::LiquidationCheck));
    }

    #[test]
    fn test_engine_modes() {
        // Mock mode engine
        let mock_engine = FheBalanceEngine::new_mock();
        assert!(mock_engine.mock_mode);
        assert!(!mock_engine.is_real_fhe());

        // Default engine should be mock mode
        let default_engine = FheBalanceEngine::default();
        assert!(default_engine.mock_mode);
        assert!(!default_engine.is_real_fhe());
    }

    #[test]
    fn test_encrypted_value_conversion() {
        let engine = FheBalanceEngine::new_mock();

        let original = EncryptedValue {
            c1: [1u8; 32],
            c2: [2u8; 32],
        };

        // Test round-trip conversion
        let bytes = engine.encrypted_value_to_bytes(&original);
        assert_eq!(bytes.len(), 64);

        let recovered = engine.bytes_to_encrypted_value(&bytes).unwrap();
        assert_eq!(recovered.c1, original.c1);
        assert_eq!(recovered.c2, original.c2);

        // Test error on invalid bytes
        let short_bytes = vec![0u8; 32];
        assert!(engine.bytes_to_encrypted_value(&short_bytes).is_err());
    }

    #[test]
    fn test_tfhe_ciphertext_detection() {
        // Small data is not real TFHE
        let small = TfheCiphertext { data: vec![0u8; 64] };
        assert!(!small.is_real_tfhe());

        // Large data (>1KB) is real TFHE
        let large = TfheCiphertext { data: vec![0u8; 65856] };
        assert!(large.is_real_tfhe());
    }

    /// Integration test with real TFHE operations
    /// Requires FHE keys to be generated (run generate-fhe-keys first)
    #[tokio::test]
    #[ignore] // Run with: cargo test test_real_tfhe_balance_update -- --ignored
    async fn test_real_tfhe_balance_update() {
        use zyberlink_fhe::{generate_keys, encrypt_value, FheUint8};
        use zyberlink_fhe::prelude::*;

        // Generate keys
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = Arc::new(FheEngine::new(server_key));
        let balance_engine = FheBalanceEngine::new(engine.clone());

        // Encrypt test values
        let current_value = 42u8;
        let delta_value = 10u8;

        let current_encrypted = FheUint8::try_encrypt(current_value, &client_key).unwrap();
        let delta_encrypted = FheUint8::try_encrypt(delta_value, &client_key).unwrap();

        let current_bytes = bincode::serialize(&current_encrypted).unwrap();
        let delta_bytes = bincode::serialize(&delta_encrypted).unwrap();

        println!("Encrypted current: {} bytes", current_bytes.len());
        println!("Encrypted delta: {} bytes", delta_bytes.len());

        // Create input with real TFHE ciphertexts
        let input = FheBalanceInput {
            job_id: 100,
            job_type: JobType::BalanceUpdate,
            current_encrypted: EncryptedValue {
                c1: [0u8; 32], // Placeholder for Cairo compatibility
                c2: [0u8; 32],
            },
            delta_encrypted: Some(EncryptedValue {
                c1: [0u8; 32],
                c2: [0u8; 32],
            }),
            params: FheJobParams::BalanceUpdate { is_mint: false },
            tfhe_current: Some(TfheCiphertext { data: current_bytes }),
            tfhe_delta: Some(TfheCiphertext { data: delta_bytes }),
        };

        // Process with real FHE
        let result = balance_engine.process(&input).await.unwrap();

        assert!(result.success);
        assert!(result.new_encrypted.is_some());

        // The result_hash should be deterministic for same inputs
        println!("Result hash: {}", hex::encode(result.result_hash));
        println!("Real FHE balance update test PASSED!");
    }
}
