//! FHE Generator SDK
//!
//! Client SDK for interacting with the ZyberLink FHE Generator program.
//! Provides instruction builders and PDA derivation helpers for FHE job operations.

use solana_sdk::pubkey::Pubkey;

pub mod instructions;

// Re-export types from fhe-generator
pub use fhe_generator::{
    FheConsensusData, FheGeneratorInstruction, FheJob,
    CIRCUIT_FHE_ADD, CIRCUIT_FHE_AVERAGE, CIRCUIT_FHE_COUNT_IF, CIRCUIT_FHE_HISTOGRAM,
    CIRCUIT_FHE_MULTIPLY, CIRCUIT_FHE_RANGE_CHECK, CIRCUIT_FHE_SUM, CIRCUIT_FHE_THRESHOLD,
};

/// Seeds for PDA derivation
pub const FHE_JOB_SEED: &[u8] = b"fhe_job";
pub const FHE_CONSENSUS_SEED: &[u8] = b"fhe_consensus";
pub const ESCROW_SEED: &[u8] = b"fhe_escrow";

/// Derive FHE job PDA
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `creator` - Job creator wallet
/// * `job_id` - Job ID (u64)
///
/// # Returns
/// Tuple of (PDA address, bump seed)
pub fn derive_job_pda(program_id: &Pubkey, creator: &Pubkey, job_id: u64) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(
        &[FHE_JOB_SEED, creator.as_ref(), &job_id_bytes],
        program_id,
    )
}

/// Derive FHE consensus PDA
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `job_id` - Job ID (u64)
///
/// # Returns
/// Tuple of (PDA address, bump seed)
pub fn derive_consensus_pda(program_id: &Pubkey, job_id: u64) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(&[FHE_CONSENSUS_SEED, &job_id_bytes], program_id)
}

/// Derive escrow PDA
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `job_pda` - Job PDA address
///
/// # Returns
/// Tuple of (PDA address, bump seed)
pub fn derive_escrow_pda(program_id: &Pubkey, job_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id)
}
