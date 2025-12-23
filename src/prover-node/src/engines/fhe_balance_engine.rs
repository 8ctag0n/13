use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
    /// Current encrypted balance (or collateral)
    pub current_encrypted: EncryptedValue,
    /// Delta to add (for BalanceUpdate) or amount to verify (for others)
    pub delta_encrypted: Option<EncryptedValue>,
    /// Additional parameters
    pub params: FheJobParams,
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

/// Main FHE Balance Engine
pub struct FheBalanceEngine {
    /// Mock mode for testing
    mock_mode: bool,
}

impl FheBalanceEngine {
    pub fn new() -> Self {
        Self { mock_mode: false }
    }

    pub fn new_mock() -> Self {
        Self { mock_mode: true }
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

        // Perform homomorphic addition
        let new_encrypted = input.current_encrypted.add(delta);
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
}

impl Default for FheBalanceEngine {
    fn default() -> Self {
        Self::new()
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
        let engine = FheBalanceEngine::new();
        assert!(engine.supports_job_type(JobType::LoanVerification));
        assert!(engine.supports_job_type(JobType::BalanceUpdate));
        assert!(engine.supports_job_type(JobType::StakeProof));
        assert!(engine.supports_job_type(JobType::TransferProof));
        assert!(engine.supports_job_type(JobType::LiquidationCheck));
    }
}
