use zyberlink_sdk::FheConsensusData;
use zyberlink_types::{CircuitType, FheOperation, HistogramBin};

// Circuit type ID constants (must match program)
const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
const CIRCUIT_CREDENTIAL: u8 = 3;
const CIRCUIT_FHE_ADD: u8 = 4;
const CIRCUIT_FHE_MULTIPLY: u8 = 5;
const CIRCUIT_FHE_SUM: u8 = 6;
const CIRCUIT_FHE_THRESHOLD: u8 = 7;
const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
const CIRCUIT_FHE_AVERAGE: u8 = 9;
const CIRCUIT_FHE_COUNT_IF: u8 = 10;
const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

/// Circuit category for routing to appropriate proof engine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitCategory {
    /// Halo2 Orchard proofs (circuits 0-3)
    Halo2Orchard,
    /// FHE computation proofs (circuits 4-9)
    FheComputation,
    /// Snarkjs Groth16 proofs (circuits 10-49)
    SnarkjsGroth16,
}

/// Registry for circuit type information and routing
pub struct CircuitRegistry;

impl CircuitRegistry {
    /// Get circuit category from circuit ID
    pub fn circuit_category(circuit_type: u8) -> CircuitCategory {
        match circuit_type {
            0..=3 => CircuitCategory::Halo2Orchard,
            4..=9 => CircuitCategory::FheComputation,
            10..=49 => CircuitCategory::SnarkjsGroth16,
            _ => CircuitCategory::SnarkjsGroth16, // Default for unknown circuits
        }
    }

    /// Check if circuit is FHE type
    pub fn is_fhe_circuit(circuit_type: u8) -> bool {
        circuit_type >= 4 && circuit_type <= 11
    }

    /// Check if circuit is ZK type (Snarkjs Groth16)
    pub fn is_zk_circuit(circuit_type: u8) -> bool {
        circuit_type >= 10 && circuit_type <= 49
    }

    /// Convert circuit_type u8 to CircuitType enum
    /// For FHE jobs, also requires FheConsensusData to reconstruct the FheOperation
    pub fn get_circuit_type(
        circuit_type: u8,
        fhe_data: Option<&FheConsensusData>,
    ) -> CircuitType {
        match circuit_type {
            CIRCUIT_ZCASH_ORCHARD => CircuitType::ZcashOrchard,
            1 => CircuitType::ZcashOrchard, // ZcashSapling not yet supported
            CIRCUIT_ANONYMOUS_VOTE => CircuitType::AnonymousVote,
            CIRCUIT_CREDENTIAL => CircuitType::Credential,
            CIRCUIT_FHE_ADD => {
                let param = fhe_data.map(|d| d.operation_param1 as u8).unwrap_or(0);
                CircuitType::FheComputation(FheOperation::Add(param))
            }
            CIRCUIT_FHE_MULTIPLY => {
                let param = fhe_data.map(|d| d.operation_param1 as u8).unwrap_or(0);
                CircuitType::FheComputation(FheOperation::Multiply(param))
            }
            CIRCUIT_FHE_SUM => {
                let count = fhe_data.map(|d| d.operation_param1).unwrap_or(0);
                CircuitType::FheComputation(FheOperation::Sum {
                    expected_count: count,
                })
            }
            CIRCUIT_FHE_THRESHOLD => {
                let threshold = fhe_data.map(|d| d.operation_param1 as u8).unwrap_or(0);
                let greater_or_equal = fhe_data.map(|d| d.operation_param2 != 0).unwrap_or(true);
                CircuitType::FheComputation(FheOperation::Threshold {
                    threshold,
                    greater_or_equal,
                })
            }
            CIRCUIT_FHE_RANGE_CHECK => {
                let min = fhe_data.map(|d| d.operation_param2).unwrap_or(0);
                let max = fhe_data.map(|d| d.operation_param3).unwrap_or(255);
                CircuitType::FheComputation(FheOperation::RangeCheck { min, max })
            }
            CIRCUIT_FHE_AVERAGE => {
                let count = fhe_data.map(|d| d.operation_param1).unwrap_or(0);
                CircuitType::FheComputation(FheOperation::Average {
                    expected_count: count,
                })
            }
            CIRCUIT_FHE_COUNT_IF => {
                let count = fhe_data.map(|d| d.operation_param1).unwrap_or(0);
                // param2 = predicate type (0=Equals, 1=GreaterThan, 2=LessThan, 3=InRange, 4=NotEquals)
                let predicate_type = fhe_data.map(|d| d.operation_param2).unwrap_or(0);
                // param3 = predicate value (threshold)
                let predicate_value = fhe_data.map(|d| d.operation_param3).unwrap_or(0);

                // Reconstruct predicate from packed params
                let predicate = match predicate_type {
                    1 => zyberlink_types::fhe::FhePredicate::GreaterThan(predicate_value),
                    2 => zyberlink_types::fhe::FhePredicate::LessThan(predicate_value),
                    3 => zyberlink_types::fhe::FhePredicate::InRange {
                        min: predicate_value,
                        max: predicate_value,
                    }, // Note: max not fully stored, limitation of packed format
                    4 => zyberlink_types::fhe::FhePredicate::NotEquals(predicate_value),
                    _ => zyberlink_types::fhe::FhePredicate::Equals(predicate_value),
                };

                CircuitType::FheComputation(FheOperation::CountIf {
                    predicate,
                    expected_count: count,
                })
            }
            CIRCUIT_FHE_HISTOGRAM => {
                let bins_count = fhe_data.map(|d| d.operation_param1 as usize).unwrap_or(4);
                // Default histogram bins, actual bins need to come from FheConsensusData extension
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
            _ => CircuitType::Custom(format!("Unknown({})", circuit_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_category() {
        assert_eq!(
            CircuitRegistry::circuit_category(0),
            CircuitCategory::Halo2Orchard
        );
        assert_eq!(
            CircuitRegistry::circuit_category(3),
            CircuitCategory::Halo2Orchard
        );
        assert_eq!(
            CircuitRegistry::circuit_category(4),
            CircuitCategory::FheComputation
        );
        assert_eq!(
            CircuitRegistry::circuit_category(9),
            CircuitCategory::FheComputation
        );
        assert_eq!(
            CircuitRegistry::circuit_category(10),
            CircuitCategory::SnarkjsGroth16
        );
        assert_eq!(
            CircuitRegistry::circuit_category(49),
            CircuitCategory::SnarkjsGroth16
        );
    }

    #[test]
    fn test_is_fhe_circuit() {
        assert!(!CircuitRegistry::is_fhe_circuit(3));
        assert!(CircuitRegistry::is_fhe_circuit(4));
        assert!(CircuitRegistry::is_fhe_circuit(11));
        assert!(!CircuitRegistry::is_fhe_circuit(12));
    }

    #[test]
    fn test_is_zk_circuit() {
        assert!(!CircuitRegistry::is_zk_circuit(9));
        assert!(CircuitRegistry::is_zk_circuit(10));
        assert!(CircuitRegistry::is_zk_circuit(49));
        assert!(!CircuitRegistry::is_zk_circuit(50));
    }

    #[test]
    fn test_get_circuit_type_basic() {
        // Test basic circuit types without FHE data
        assert_eq!(
            CircuitRegistry::get_circuit_type(CIRCUIT_ZCASH_ORCHARD, None),
            CircuitType::ZcashOrchard
        );
        assert_eq!(
            CircuitRegistry::get_circuit_type(CIRCUIT_ANONYMOUS_VOTE, None),
            CircuitType::AnonymousVote
        );
        assert_eq!(
            CircuitRegistry::get_circuit_type(CIRCUIT_CREDENTIAL, None),
            CircuitType::Credential
        );
    }

    #[test]
    fn test_get_circuit_type_fhe() {
        use solana_sdk::pubkey::Pubkey;

        // Test FHE circuit types - create full FheConsensusData structure
        let fhe_data = FheConsensusData {
            job_id: 1,
            operation_type: 4,
            operation_param1: 10,
            operation_param2: 5,
            operation_param3: 100,
            required_provers: 3,
            consensus_threshold: 2,
            submission_timeout: 3600,
            claimed_provers: [Pubkey::default(); 5],
            claimed_count: 0,
            result_hashes: [[0u8; 32]; 5],
            result_submitted: [false; 5],
            results_count: 0,
            consensus_hash: None,
            bump: 0,
        };

        // Test FHE Add
        match CircuitRegistry::get_circuit_type(CIRCUIT_FHE_ADD, Some(&fhe_data)) {
            CircuitType::FheComputation(FheOperation::Add(param)) => {
                assert_eq!(param, 10);
            }
            _ => panic!("Expected FheComputation::Add"),
        }

        // Test FHE Sum
        match CircuitRegistry::get_circuit_type(CIRCUIT_FHE_SUM, Some(&fhe_data)) {
            CircuitType::FheComputation(FheOperation::Sum { expected_count }) => {
                assert_eq!(expected_count, 10);
            }
            _ => panic!("Expected FheComputation::Sum"),
        }

        // Test FHE RangeCheck
        match CircuitRegistry::get_circuit_type(CIRCUIT_FHE_RANGE_CHECK, Some(&fhe_data)) {
            CircuitType::FheComputation(FheOperation::RangeCheck { min, max }) => {
                assert_eq!(min, 5);
                assert_eq!(max, 100);
            }
            _ => panic!("Expected FheComputation::RangeCheck"),
        }
    }

    #[test]
    fn test_get_circuit_type_unknown() {
        // Test unknown circuit type
        match CircuitRegistry::get_circuit_type(99, None) {
            CircuitType::Custom(s) => {
                assert_eq!(s, "Unknown(99)");
            }
            _ => panic!("Expected Custom circuit type"),
        }
    }
}
