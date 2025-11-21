use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// Cost configuration for an FHE operation
/// Defines pricing and timeout based on computational complexity
#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct OperationCostConfig {
    /// Minimum payment required in lamports
    pub min_payment_lamports: u64,

    /// Timeout for this operation in seconds
    pub timeout_seconds: i64,

    /// Complexity tier (1-5, where 5 is most complex)
    pub complexity_tier: u8,
}

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

    /// Get dynamic cost configuration based on computational complexity
    ///
    /// Pricing tiers:
    /// - Tier 1 (O(1) arithmetic): Add, Multiply
    /// - Tier 2 (O(n) aggregation): Sum
    /// - Tier 3 (O(1) + bootstrap): Threshold, RangeCheck
    /// - Tier 4 (O(n) + predicates): Average, CountIf
    /// - Tier 5 (O(n×m) exponential): Histogram
    pub fn get_cost_config(&self) -> OperationCostConfig {
        const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

        match self {
            // Tier 1: Simple arithmetic (O(1))
            // Cost: 0.001 SOL, Timeout: 60s
            FheOperation::Add(_) | FheOperation::Multiply(_) => OperationCostConfig {
                min_payment_lamports: LAMPORTS_PER_SOL / 1000, // 0.001 SOL
                timeout_seconds: 60,
                complexity_tier: 1,
            },

            // Tier 2: Linear aggregation (O(n))
            // Cost: 0.001 + (n × 0.0001) SOL
            // Timeout: 60 + (n × 2)s
            FheOperation::Sum { expected_count } => {
                let n = *expected_count as u64;
                let base_cost = LAMPORTS_PER_SOL / 1000; // 0.001 SOL
                let per_item_cost = LAMPORTS_PER_SOL / 10000; // 0.0001 SOL per item

                OperationCostConfig {
                    min_payment_lamports: base_cost + (n * per_item_cost),
                    timeout_seconds: 60 + (n as i64 * 2),
                    complexity_tier: 2,
                }
            },

            // Tier 3: Bootstrap operations (O(1) + expensive bootstrapping)
            // Cost: 0.005 SOL, Timeout: 300s
            FheOperation::Threshold { .. } | FheOperation::RangeCheck { .. } => {
                OperationCostConfig {
                    min_payment_lamports: LAMPORTS_PER_SOL / 200, // 0.005 SOL
                    timeout_seconds: 300, // 5 minutes
                    complexity_tier: 3,
                }
            },

            // Tier 4: Composite operations (O(n) + predicates/division)
            FheOperation::Average { expected_count } => {
                let n = *expected_count as u64;
                let base_cost = LAMPORTS_PER_SOL / 1000; // 0.001 SOL
                let per_item_cost = LAMPORTS_PER_SOL / 5000; // 0.0002 SOL per item

                OperationCostConfig {
                    min_payment_lamports: base_cost + (n * per_item_cost),
                    timeout_seconds: 60 + (n as i64 * 3),
                    complexity_tier: 4,
                }
            },

            FheOperation::CountIf { expected_count, .. } => {
                let n = *expected_count as u64;
                let base_cost = LAMPORTS_PER_SOL / 200; // 0.005 SOL
                let per_item_cost = (LAMPORTS_PER_SOL * 3) / 10_000; // 0.0003 SOL per item (300_000 lamports)

                OperationCostConfig {
                    min_payment_lamports: base_cost + (n * per_item_cost),
                    timeout_seconds: 300 + (n as i64 * 5),
                    complexity_tier: 4,
                }
            },

            // Tier 5: Exponential complexity (O(n×m))
            // Most expensive due to combinatorial explosion
            // Cost: 0.02 × (bins^1.5) SOL + base of 0.1 SOL
            // Timeout: 600 + (bins × 100)s
            FheOperation::Histogram { bins } => {
                let m = bins.len() as u64;

                // Base cost for histogram operation
                let base_cost = LAMPORTS_PER_SOL / 10; // 0.1 SOL base

                // Exponential cost scales with bins^1.5
                // Using m^1.5 = sqrt(m^3) for integer math
                let m_cubed = m * m * m;
                let m_power_1_5 = (m_cubed as f64).sqrt() as u64;
                let exponential_cost = (LAMPORTS_PER_SOL / 50) * m_power_1_5; // 0.02 SOL × m^1.5

                OperationCostConfig {
                    min_payment_lamports: base_cost + exponential_cost,
                    timeout_seconds: 600 + (m as i64 * 100), // 10 minutes + 100s per bin
                    complexity_tier: 5,
                }
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

    // DYNAMIC PRICING TESTS

    #[test]
    fn test_tier1_add_pricing() {
        let op = FheOperation::Add(42);
        let config = op.get_cost_config();

        assert_eq!(config.complexity_tier, 1);
        assert_eq!(config.min_payment_lamports, 1_000_000); // 0.001 SOL
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_tier1_multiply_pricing() {
        let op = FheOperation::Multiply(7);
        let config = op.get_cost_config();

        assert_eq!(config.complexity_tier, 1);
        assert_eq!(config.min_payment_lamports, 1_000_000); // 0.001 SOL
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_tier2_sum_pricing_scales_with_count() {
        // Test small count
        let op_small = FheOperation::Sum { expected_count: 10 };
        let config_small = op_small.get_cost_config();

        assert_eq!(config_small.complexity_tier, 2);
        // 0.001 + (10 × 0.0001) = 0.002 SOL = 2_000_000 lamports
        assert_eq!(config_small.min_payment_lamports, 2_000_000);
        // 60 + (10 × 2) = 80s
        assert_eq!(config_small.timeout_seconds, 80);

        // Test large count
        let op_large = FheOperation::Sum { expected_count: 1000 };
        let config_large = op_large.get_cost_config();

        // 0.001 + (1000 × 0.0001) = 0.101 SOL = 101_000_000 lamports
        assert_eq!(config_large.min_payment_lamports, 101_000_000);
        // 60 + (1000 × 2) = 2060s
        assert_eq!(config_large.timeout_seconds, 2060);
    }

    #[test]
    fn test_tier3_threshold_pricing() {
        let op = FheOperation::Threshold {
            threshold: 18,
            greater_or_equal: true,
        };
        let config = op.get_cost_config();

        assert_eq!(config.complexity_tier, 3);
        assert_eq!(config.min_payment_lamports, 5_000_000); // 0.005 SOL
        assert_eq!(config.timeout_seconds, 300); // 5 minutes
    }

    #[test]
    fn test_tier3_range_check_pricing() {
        let op = FheOperation::RangeCheck { min: 18, max: 65 };
        let config = op.get_cost_config();

        assert_eq!(config.complexity_tier, 3);
        assert_eq!(config.min_payment_lamports, 5_000_000); // 0.005 SOL
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_tier4_average_pricing_scales() {
        // Small average
        let op_small = FheOperation::Average { expected_count: 10 };
        let config_small = op_small.get_cost_config();

        assert_eq!(config_small.complexity_tier, 4);
        // 0.001 + (10 × 0.0002) = 0.003 SOL = 3_000_000 lamports
        assert_eq!(config_small.min_payment_lamports, 3_000_000);
        // 60 + (10 × 3) = 90s
        assert_eq!(config_small.timeout_seconds, 90);

        // Large average
        let op_large = FheOperation::Average { expected_count: 500 };
        let config_large = op_large.get_cost_config();

        // 0.001 + (500 × 0.0002) = 0.101 SOL = 101_000_000 lamports
        assert_eq!(config_large.min_payment_lamports, 101_000_000);
        // 60 + (500 × 3) = 1560s
        assert_eq!(config_large.timeout_seconds, 1560);
    }

    #[test]
    fn test_tier4_count_if_pricing_scales() {
        let op = FheOperation::CountIf {
            predicate: FhePredicate::GreaterThan(18),
            expected_count: 100,
        };
        let config = op.get_cost_config();

        assert_eq!(config.complexity_tier, 4);
        // 0.005 + (100 × 0.0003) = 0.035 SOL = 35_000_000 lamports
        assert_eq!(config.min_payment_lamports, 35_000_000);
        // 300 + (100 × 5) = 800s
        assert_eq!(config.timeout_seconds, 800);
    }

    #[test]
    fn test_tier5_histogram_exponential_pricing() {
        // Small histogram (3 bins)
        let op_small = FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 10, "bin1"),
                HistogramBin::new(11, 20, "bin2"),
                HistogramBin::new(21, 30, "bin3"),
            ],
        };
        let config_small = op_small.get_cost_config();

        assert_eq!(config_small.complexity_tier, 5);
        // Base: 0.1 SOL + (0.02 × 3^1.5) ≈ 0.1 + 0.104 = 0.204 SOL
        // Should be >= 0.2 SOL for 3 bins
        assert!(config_small.min_payment_lamports >= 200_000_000);
        // 600 + (3 × 100) = 900s
        assert_eq!(config_small.timeout_seconds, 900);

        // Large histogram (10 bins)
        let op_large = FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 10, "bin1"),
                HistogramBin::new(11, 20, "bin2"),
                HistogramBin::new(21, 30, "bin3"),
                HistogramBin::new(31, 40, "bin4"),
                HistogramBin::new(41, 50, "bin5"),
                HistogramBin::new(51, 60, "bin6"),
                HistogramBin::new(61, 70, "bin7"),
                HistogramBin::new(71, 80, "bin8"),
                HistogramBin::new(81, 90, "bin9"),
                HistogramBin::new(91, 100, "bin10"),
            ],
        };
        let config_large = op_large.get_cost_config();

        // Base: 0.1 SOL + (0.02 × 10^1.5) ≈ 0.1 + 0.632 = 0.732 SOL
        // Should be > 0.7 SOL for 10 bins
        assert!(config_large.min_payment_lamports > 700_000_000);
        // Should be MUCH more expensive than small histogram (exponential scaling)
        assert!(config_large.min_payment_lamports > config_small.min_payment_lamports * 3);
        // 600 + (10 × 100) = 1600s
        assert_eq!(config_large.timeout_seconds, 1600);
    }

    #[test]
    fn test_pricing_increases_with_complexity_tier() {
        let tier1 = FheOperation::Add(1).get_cost_config();
        let tier2 = FheOperation::Sum { expected_count: 1 }.get_cost_config();
        let tier3 = FheOperation::Threshold { threshold: 18, greater_or_equal: true }.get_cost_config();
        let tier4 = FheOperation::Average { expected_count: 1 }.get_cost_config();
        let tier5 = FheOperation::Histogram { bins: vec![HistogramBin::new(0, 10, "bin1")] }.get_cost_config();

        // Verify tier ordering
        assert!(tier1.min_payment_lamports < tier3.min_payment_lamports);
        assert!(tier3.min_payment_lamports < tier5.min_payment_lamports);

        // Verify timeout ordering
        assert!(tier1.timeout_seconds < tier3.timeout_seconds);
        assert!(tier3.timeout_seconds < tier5.timeout_seconds);
    }

    #[test]
    fn test_operation_cost_config_serialization() {
        let config = OperationCostConfig {
            min_payment_lamports: 5_000_000,
            timeout_seconds: 300,
            complexity_tier: 3,
        };

        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: OperationCostConfig = borsh::from_slice(&serialized).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_cost_config_realistic_scenarios() {
        // Census count of 10,000 people
        let census = FheOperation::Sum { expected_count: 10_000 };
        let census_config = census.get_cost_config();
        // 0.001 + (10_000 × 0.0001) = 1.001 SOL
        assert!(census_config.min_payment_lamports > 1_000_000_000);
        assert!(census_config.min_payment_lamports < 1_100_000_000);

        // Age verification
        let age_check = FheOperation::Threshold {
            threshold: 18,
            greater_or_equal: true,
        };
        let age_config = age_check.get_cost_config();
        assert_eq!(age_config.min_payment_lamports, 5_000_000); // 0.005 SOL

        // Voting histogram (5 candidates)
        let voting = FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 0, "Candidate A"),
                HistogramBin::new(1, 1, "Candidate B"),
                HistogramBin::new(2, 2, "Candidate C"),
                HistogramBin::new(3, 3, "Candidate D"),
                HistogramBin::new(4, 4, "Candidate E"),
            ],
        };
        let voting_config = voting.get_cost_config();
        // Base: 0.1 SOL + (0.02 × 5^1.5) ≈ 0.1 + 0.224 = 0.324 SOL
        // Voting should be expensive (> 0.3 SOL)
        assert!(voting_config.min_payment_lamports > 300_000_000);
        assert!(voting_config.min_payment_lamports < 400_000_000);
    }
}
