//! FHE Generator instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum FheGeneratorInstruction {
    /// Create a new FHE computation job
    CreateJob {
        job_id: u64,
        circuit_type: u8,
        witness_hash: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        required_provers: u8,
        consensus_threshold: u8,
        /// Packed operation parameters
        operation_param1: u16,
        operation_param2: u8,
        operation_param3: u8,
    },

    /// Claim a job as a prover (multi-prover, up to required_provers)
    ClaimJob,

    /// Submit result for a claimed job
    SubmitResult { result_hash: [u8; 32] },

    /// Finalize job after all results submitted (check consensus, distribute payments)
    FinalizeJob,

    /// Cancel a pending job (creator only)
    CancelJob,
}
