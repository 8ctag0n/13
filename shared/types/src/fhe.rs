use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// Predicate for conditional FHE operations
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FhePredicate {
    /// Check if encrypted value equals constant
    Equals(u8),

    /// Check if encrypted value > constant
    GreaterThan(u8),

    /// Check if encrypted value < constant
    LessThan(u8),

    /// Check if encrypted value is in range [min, max]
    InRange { min: u8, max: u8 },

    /// Check if encrypted value != constant
    NotEquals(u8),
}

/// Histogram bin definition for distribution counting
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct HistogramBin {
    /// Minimum value (inclusive)
    pub min: u8,

    /// Maximum value (inclusive)
    pub max: u8,

    /// Human-readable label for this bin
    pub label: String,
}

impl HistogramBin {
    pub fn new(min: u8, max: u8, label: impl Into<String>) -> Self {
        Self {
            min,
            max,
            label: label.into(),
        }
    }
}

/// FHE operation to perform on encrypted data
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FheOperation {
    /// Add a constant to encrypted value
    /// Example: encrypt(42) + 10 = encrypt(52)
    Add(u8),

    /// Multiply encrypted value by constant
    /// Example: encrypt(5) * 3 = encrypt(15)
    Multiply(u8),

    // TRACK A - Foundation Layer (Simple Operations)

    /// Sum multiple encrypted values
    /// Used for census counting and population statistics
    /// Example: encrypt(1) + encrypt(1) + ... = encrypt(N)
    Sum { expected_count: u16 },

    /// Check if encrypted value meets threshold condition
    /// Used for age verification in zk-passport
    /// Example: encrypt(age) >= 18 returns encrypt(1) if true, encrypt(0) if false
    Threshold { threshold: u8, greater_or_equal: bool },

    /// Verify encrypted value is within valid range
    /// Used for passport age bounds checking
    /// Example: 18 <= encrypt(age) <= 65
    RangeCheck { min: u8, max: u8 },

    // TRACK B - Extension Layer (Composite Operations)

    /// Compute average of multiple encrypted values
    /// Returns (encrypted_sum, count) for client-side division
    /// Used for demographic analysis
    /// Example: avg([encrypt(25), encrypt(30), encrypt(35)]) = (encrypt(90), 3) -> 30
    Average { expected_count: u16 },

    /// Count how many encrypted values satisfy a predicate
    /// Used for conditional census and filtering
    /// Example: count_if([encrypt(15), encrypt(25), encrypt(30)], GreaterThan(18)) = encrypt(2)
    CountIf { predicate: FhePredicate, expected_count: u16 },

    /// Compute distribution histogram across bins
    /// Used for private voting and demographic distribution
    /// Example: histogram([votes...], [bin1, bin2, bin3]) = [encrypt(30), encrypt(20), encrypt(10)]
    Histogram { bins: Vec<HistogramBin> },
}

impl FheOperation {
    /// Get a human-readable name for this operation
    pub fn name(&self) -> &str {
        match self {
            FheOperation::Add(_) => "Add",
            FheOperation::Multiply(_) => "Multiply",
            FheOperation::Sum { .. } => "Sum",
            FheOperation::Threshold { .. } => "Threshold",
            FheOperation::RangeCheck { .. } => "RangeCheck",
            FheOperation::Average { .. } => "Average",
            FheOperation::CountIf { .. } => "CountIf",
            FheOperation::Histogram { .. } => "Histogram",
        }
    }

    /// Estimate typical computation time in milliseconds for this operation
    /// Based on spike-fhe-prover benchmarks
    pub fn estimated_compute_time_ms(&self) -> u32 {
        match self {
            FheOperation::Add(_) => 150,      // ~150ms from spike
            FheOperation::Multiply(_) => 200, // Estimate, slightly higher
            FheOperation::Sum { expected_count } => 150 + (*expected_count as u32 * 50),
            FheOperation::Threshold { .. } => 300,
            FheOperation::RangeCheck { .. } => 400,
            FheOperation::Average { expected_count } => 150 + (*expected_count as u32 * 50), // Same as Sum
            FheOperation::CountIf { expected_count, .. } => 300 + (*expected_count as u32 * 100),
            FheOperation::Histogram { bins } => {
                let bin_count = bins.len() as u32;
                500 + (bin_count * 3000) // ~3s per bin for 100 inputs
            },
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

    // TRACK A - Foundation Layer Serialization Tests

    #[test]
    fn test_sum_serialization() {
        let op = FheOperation::Sum { expected_count: 100 };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_threshold_serialization() {
        let op = FheOperation::Threshold {
            threshold: 18,
            greater_or_equal: true,
        };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_threshold_less_than_serialization() {
        let op = FheOperation::Threshold {
            threshold: 65,
            greater_or_equal: false,
        };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_range_check_serialization() {
        let op = FheOperation::RangeCheck { min: 18, max: 65 };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_sum_operation_name() {
        let op = FheOperation::Sum { expected_count: 100 };
        assert_eq!(op.name(), "Sum");
    }

    #[test]
    fn test_threshold_operation_name() {
        let op = FheOperation::Threshold {
            threshold: 18,
            greater_or_equal: true,
        };
        assert_eq!(op.name(), "Threshold");
    }

    #[test]
    fn test_range_check_operation_name() {
        let op = FheOperation::RangeCheck { min: 18, max: 65 };
        assert_eq!(op.name(), "RangeCheck");
    }

    #[test]
    fn test_sum_compute_time_estimation() {
        let op = FheOperation::Sum { expected_count: 10 };
        assert_eq!(op.estimated_compute_time_ms(), 150 + (10 * 50)); // 650ms
    }

    #[test]
    fn test_threshold_compute_time_estimation() {
        let op = FheOperation::Threshold {
            threshold: 18,
            greater_or_equal: true,
        };
        assert_eq!(op.estimated_compute_time_ms(), 300);
    }

    #[test]
    fn test_range_check_compute_time_estimation() {
        let op = FheOperation::RangeCheck { min: 18, max: 65 };
        assert_eq!(op.estimated_compute_time_ms(), 400);
    }

    // TRACK B - Extension Layer Serialization Tests

    #[test]
    fn test_fhe_predicate_serialization() {
        let predicates = vec![
            FhePredicate::Equals(42),
            FhePredicate::GreaterThan(18),
            FhePredicate::LessThan(100),
            FhePredicate::InRange { min: 18, max: 65 },
            FhePredicate::NotEquals(0),
        ];

        for pred in predicates {
            let serialized = borsh::to_vec(&pred).unwrap();
            let deserialized: FhePredicate = borsh::from_slice(&serialized).unwrap();
            assert_eq!(pred, deserialized);
        }
    }

    #[test]
    fn test_histogram_bin_creation() {
        let bin = HistogramBin::new(0, 10, "Range 0-10");
        assert_eq!(bin.min, 0);
        assert_eq!(bin.max, 10);
        assert_eq!(bin.label, "Range 0-10");
    }

    #[test]
    fn test_histogram_bin_serialization() {
        let bin = HistogramBin::new(0, 10, "Range 0-10");
        let serialized = borsh::to_vec(&bin).unwrap();
        let deserialized: HistogramBin = borsh::from_slice(&serialized).unwrap();
        assert_eq!(bin, deserialized);
    }

    #[test]
    fn test_average_serialization() {
        let op = FheOperation::Average { expected_count: 100 };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_count_if_serialization() {
        let op = FheOperation::CountIf {
            predicate: FhePredicate::GreaterThan(18),
            expected_count: 50,
        };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_histogram_serialization() {
        let op = FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 10, "0-10"),
                HistogramBin::new(11, 20, "11-20"),
            ],
        };
        let serialized = borsh::to_vec(&op).unwrap();
        let deserialized: FheOperation = borsh::from_slice(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_average_operation_name() {
        let op = FheOperation::Average { expected_count: 100 };
        assert_eq!(op.name(), "Average");
    }

    #[test]
    fn test_count_if_operation_name() {
        let op = FheOperation::CountIf {
            predicate: FhePredicate::Equals(1),
            expected_count: 50,
        };
        assert_eq!(op.name(), "CountIf");
    }

    #[test]
    fn test_histogram_operation_name() {
        let op = FheOperation::Histogram {
            bins: vec![HistogramBin::new(0, 10, "bin1")],
        };
        assert_eq!(op.name(), "Histogram");
    }

    #[test]
    fn test_average_compute_time_estimation() {
        let op = FheOperation::Average { expected_count: 10 };
        assert_eq!(op.estimated_compute_time_ms(), 150 + (10 * 50)); // 650ms (same as Sum)
    }

    #[test]
    fn test_count_if_compute_time_estimation() {
        let op = FheOperation::CountIf {
            predicate: FhePredicate::GreaterThan(18),
            expected_count: 10,
        };
        assert_eq!(op.estimated_compute_time_ms(), 300 + (10 * 100)); // 1300ms
    }

    #[test]
    fn test_histogram_compute_time_estimation() {
        let op = FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 10, "bin1"),
                HistogramBin::new(11, 20, "bin2"),
                HistogramBin::new(21, 30, "bin3"),
            ],
        };
        // 500 + (3 bins * 3000ms) = 9500ms
        assert_eq!(op.estimated_compute_time_ms(), 500 + (3 * 3000));
    }
}
