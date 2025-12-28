//! ZK Generator error types

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ZkGeneratorError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Job not found")]
    JobNotFound,

    #[error("Job already claimed")]
    JobAlreadyClaimed,

    #[error("Job not claimed")]
    JobNotClaimed,

    #[error("Invalid prover")]
    InvalidProver,

    #[error("Job expired")]
    JobExpired,

    #[error("Invalid proof")]
    InvalidProof,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Job not pending")]
    JobNotPending,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Arithmetic overflow")]
    Overflow,

    #[error("Invalid escrow account")]
    InvalidEscrow,

    #[error("Dispute window expired")]
    DisputeWindowExpired,

    #[error("Job already disputed")]
    AlreadyDisputed,

    #[error("Proof hash mismatch")]
    ProofHashMismatch,
}

impl From<ZkGeneratorError> for ProgramError {
    fn from(e: ZkGeneratorError) -> Self {
        ProgramError::Custom(e as u32 + 100)
    }
}
