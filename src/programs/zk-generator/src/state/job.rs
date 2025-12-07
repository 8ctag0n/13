//! ZK Job state - uses JobCommon from zyberlink-jobs

use borsh::{BorshDeserialize, BorshSerialize};
use zyberlink_jobs::JobCommon;

/// Seeds for ZK job PDA
pub const ZK_JOB_SEED: &[u8] = b"zk_job";

/// ZK Circuit types (0-3)
pub const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
pub const CIRCUIT_ZCASH_SAPLING: u8 = 1;
pub const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
pub const CIRCUIT_CREDENTIAL: u8 = 3;

/// ZK Job Account
///
/// Composes JobCommon with ZK-specific fields.
///
/// PDA: ["zk_job", creator, job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ZkJob {
    /// Common job fields (200 bytes)
    pub common: JobCommon,

    /// Circuit type (0-3)
    pub circuit_type: u8,
}

impl ZkJob {
    /// Size: JobCommon::SIZE (200) + 1 = 201 bytes
    pub const SIZE: usize = JobCommon::SIZE + 1;

    /// Check if circuit type is valid for ZK
    pub fn is_valid_circuit(circuit_type: u8) -> bool {
        circuit_type <= CIRCUIT_CREDENTIAL
    }
}
