//! Threshold program instructions

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum ThresholdInstruction {
    /// Request key shares from validators for a ZK job
    ///
    /// Accounts expected:
    /// 0. `[signer]` Prover (must match job's claimer)
    /// 1. `[writable]` Key share request PDA
    /// 2. `[]` ZK job account
    /// 3. `[]` Prover account (from bedrock)
    /// 4. `[]` System program
    RequestKeyShare {
        /// IPFS CID of the encrypted witness data
        encrypted_witness_cid: String,
    },

    /// Submit a key share response as a validator
    ///
    /// Accounts expected:
    /// 0. `[signer]` Validator authority
    /// 1. `[writable]` Key share response PDA
    /// 2. `[writable]` Key share request
    /// 3. `[]` Validator account (from bedrock)
    /// 4. `[]` System program
    /// 5. `[]` Clock sysvar
    SubmitKeyShare {
        /// Encrypted key share for the prover
        encrypted_share: Vec<u8>,
    },
}
