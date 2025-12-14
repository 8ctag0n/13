//! ZK Generator instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum ZkGeneratorInstruction {
    /// Create a new ZK proof job
    CreateJob {
        job_id: u64,
        circuit_type: u8,
        witness_hash: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
    },

    /// Claim a pending job
    ClaimJob,

    /// Submit proof hash for a claimed job
    /// Full proof verification only happens during disputes
    SubmitProof { proof_hash: [u8; 32] },

    /// Dispute a submitted proof by providing the full proof for on-chain verification
    /// Must be called within the dispute window (24 hours after completion)
    DisputeProof {
        /// The full ZK proof bytes (256 bytes for Groth16)
        proof: Vec<u8>,
        /// Public inputs for circuit verification
        public_inputs: Vec<u8>,
    },

    /// Cancel a pending job (creator only)
    CancelJob,

    // === Prover Stake Management ===

    /// Register as a prover with initial stake
    /// Requires minimum 0.5 SOL stake
    RegisterProver {
        /// Initial stake amount in lamports
        stake_amount: u64,
    },

    /// Deposit additional stake
    DepositStake {
        /// Amount to deposit in lamports
        amount: u64,
    },

    /// Withdraw available stake (not locked in jobs)
    WithdrawStake {
        /// Amount to withdraw in lamports
        amount: u64,
    },
}
