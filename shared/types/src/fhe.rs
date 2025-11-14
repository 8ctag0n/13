use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// FHE operation to perform on encrypted data
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FheOperation {
    /// Add a constant to encrypted value
    /// Example: encrypt(42) + 10 = encrypt(52)
    Add(u8),

    /// Multiply encrypted value by constant
    /// Example: encrypt(5) * 3 = encrypt(15)
    Multiply(u8),
}

impl FheOperation {
    /// Get a human-readable name for this operation
    pub fn name(&self) -> &str {
        match self {
            FheOperation::Add(_) => "Add",
            FheOperation::Multiply(_) => "Multiply",
        }
    }

    /// Estimate typical computation time in milliseconds for this operation
    /// Based on spike-fhe-prover benchmarks
    pub fn estimated_compute_time_ms(&self) -> u32 {
        match self {
            FheOperation::Add(_) => 150,      // ~150ms from spike
            FheOperation::Multiply(_) => 200, // Estimate, slightly higher
        }
    }
}

/// Configuration for FHE consensus mechanism
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FheConsensusConfig {
    /// Total number of provers required
    pub required_provers: u8,

    /// Minimum matching results for consensus (e.g., 2 out of 3)
    pub consensus_threshold: u8,

    /// Timeout for prover submission (seconds)
    pub submission_timeout_secs: i64,

    /// FHE operation to perform
    pub operation: FheOperation,
}

impl FheConsensusConfig {
    /// Validate consensus configuration
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.required_provers < 2 {
            return Err("FHE jobs require at least 2 provers");
        }
        if self.required_provers > 10 {
            return Err("FHE jobs cannot exceed 10 provers");
        }
        if self.consensus_threshold < 2 {
            return Err("Consensus threshold must be at least 2");
        }
        if self.consensus_threshold > self.required_provers {
            return Err("Consensus threshold cannot exceed required provers");
        }
        if self.submission_timeout_secs < 60 {
            return Err("Timeout must be at least 60 seconds");
        }
        Ok(())
    }

    /// Create a standard 3-of-3 consensus configuration with 2/3 threshold
    pub fn default_3_of_3(operation: FheOperation) -> Self {
        Self {
            required_provers: 3,
            consensus_threshold: 2,
            submission_timeout_secs: 300, // 5 minutes
            operation,
        }
    }
}

/// Result submitted by a prover for an FHE job
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FheJobResult {
    /// Prover who submitted this result
    pub prover: Pubkey,

    /// Hash of encrypted result (for consensus)
    /// This is SHA3_256(result_data) for matching
    pub result_hash: [u8; 32],

    /// Timestamp of submission
    pub submitted_at: i64,
}

impl FheJobResult {
    /// Create a new FHE job result
    pub fn new(prover: Pubkey, result_hash: [u8; 32], submitted_at: i64) -> Self {
        Self {
            prover,
            result_hash,
            submitted_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhe_operation_serialization() {
        let op = FheOperation::Add(42);
        let bytes = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&bytes).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_fhe_operation_multiply_serialization() {
        let op = FheOperation::Multiply(7);
        let bytes = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&bytes).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_fhe_consensus_config_validation() {
        let config = FheConsensusConfig {
            required_provers: 3,
            consensus_threshold: 2,
            submission_timeout_secs: 300,
            operation: FheOperation::Add(10),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_fhe_consensus_config_invalid_threshold() {
        let config = FheConsensusConfig {
            required_provers: 3,
            consensus_threshold: 5, // > required_provers
            submission_timeout_secs: 300,
            operation: FheOperation::Add(10),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_fhe_consensus_config_too_few_provers() {
        let config = FheConsensusConfig {
            required_provers: 1, // < 2
            consensus_threshold: 1,
            submission_timeout_secs: 300,
            operation: FheOperation::Add(10),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_fhe_job_result_creation() {
        let prover = Pubkey::new_unique();
        let hash = [42u8; 32];
        let result = FheJobResult::new(prover, hash, 1000);

        assert_eq!(result.prover, prover);
        assert_eq!(result.result_hash, hash);
        assert_eq!(result.submitted_at, 1000);
    }

    #[test]
    fn test_fhe_job_result_serialization() {
        let result = FheJobResult {
            prover: Pubkey::new_unique(),
            result_hash: [1u8; 32],
            submitted_at: 1234567890,
        };

        let serialized = borsh::to_vec(&result).unwrap();
        let deserialized: FheJobResult = borsh::from_slice(&serialized).unwrap();

        assert_eq!(result, deserialized);
    }

    #[test]
    fn test_default_3_of_3_config() {
        let config = FheConsensusConfig::default_3_of_3(FheOperation::Multiply(5));

        assert_eq!(config.required_provers, 3);
        assert_eq!(config.consensus_threshold, 2);
        assert_eq!(config.submission_timeout_secs, 300);
        assert!(config.validate().is_ok());
    }
}
