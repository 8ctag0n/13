//! ZK Generator instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum ZkGeneratorInstruction {
    /// Create a new ZK proof job
    CreateJob {
        circuit_type: u8,
        witness_hash: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
    },

    /// Claim a pending job
    ClaimJob,

    /// Submit proof for a claimed job
    SubmitProof { proof_hash: [u8; 32] },

    /// Cancel a pending job (creator only)
    CancelJob,
}
