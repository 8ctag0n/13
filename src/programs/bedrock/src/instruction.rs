//! Bedrock instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::state::ValidatorRegion;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum BedrockInstruction {
    /// Initialize the bedrock config
    ///
    /// Accounts:
    /// 0. `[signer]` Admin (authority)
    /// 1. `[writable]` Config PDA
    /// 2. `[]` System program
    Initialize {
        zk_generator_program: Pubkey,
        fhe_generator_program: Pubkey,
    },

    /// Register a new prover with stake
    ///
    /// Accounts:
    /// 0. `[signer]` Prover wallet
    /// 1. `[writable]` Prover account PDA
    /// 2. `[]` Config
    /// 3. `[]` System program
    RegisterProver { stake_lamports: u64 },

    /// Slash a prover (called by generators via CPI)
    ///
    /// Accounts:
    /// 0. `[signer]` Generator program (must be registered)
    /// 1. `[writable]` Prover account
    /// 2. `[]` Config
    SlashProver { amount: u64, reason: SlashReason },

    /// Update prover stats after job completion (CPI from generators)
    ///
    /// Accounts:
    /// 0. `[signer]` Generator program
    /// 1. `[writable]` Prover account
    /// 2. `[]` Config
    UpdateProverStats { job_completed: bool, job_failed: bool },

    /// Register a new validator for threshold encryption
    ///
    /// Accounts:
    /// 0. `[signer]` Validator wallet
    /// 1. `[writable]` Validator account PDA
    /// 2. `[writable]` Config
    /// 3. `[]` System program
    RegisterValidator {
        endpoint: String,
        region: ValidatorRegion,
        stake: u64,
    },

    /// Slash a validator (admin only)
    ///
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` Validator account
    /// 2. `[]` Config
    SlashValidator { amount: u64, reason: SlashReason },
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub enum SlashReason {
    Timeout,
    InvalidResult,
    ConsensusMismatch,
    InvalidKeyShare,
}
