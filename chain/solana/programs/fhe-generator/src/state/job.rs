//! FHE Job state - uses JobCommon from zyberlink-jobs

use borsh::{BorshDeserialize, BorshSerialize};
use zyberlink_jobs::JobCommon;

/// Seeds for FHE job PDA
pub const FHE_JOB_SEED: &[u8] = b"fhe_job";

/// FHE Operation types (4-11)
pub const CIRCUIT_FHE_ADD: u8 = 4;
pub const CIRCUIT_FHE_MULTIPLY: u8 = 5;
pub const CIRCUIT_FHE_SUM: u8 = 6;
pub const CIRCUIT_FHE_THRESHOLD: u8 = 7;
pub const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
pub const CIRCUIT_FHE_AVERAGE: u8 = 9;
pub const CIRCUIT_FHE_COUNT_IF: u8 = 10;
pub const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

/// FHE Job Account
///
/// Composes JobCommon with FHE-specific fields.
/// Has an associated FheConsensusData PDA for multi-prover consensus.
///
/// PDA: ["fhe_job", creator, job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FheJob {
    /// Common job fields (200 bytes)
    pub common: JobCommon,

    /// Circuit type (4-11)
    pub circuit_type: u8,

    /// Bump of the associated FheConsensusData PDA
    pub fhe_consensus_bump: u8,
}

impl FheJob {
    /// Size: JobCommon::SIZE (200) + 1 + 1 = 202 bytes
    pub const SIZE: usize = JobCommon::SIZE + 2;

    /// Check if circuit type is valid for FHE
    pub fn is_valid_circuit(circuit_type: u8) -> bool {
        circuit_type >= CIRCUIT_FHE_ADD && circuit_type <= CIRCUIT_FHE_HISTOGRAM
    }
}
