//! FHE Generator error types

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum FheGeneratorError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Job not found")]
    JobNotFound,

    #[error("Job already fully claimed")]
    JobFullyClaimed,

    #[error("Prover already claimed this job")]
    ProverAlreadyClaimed,

    #[error("Prover not in this job")]
    ProverNotInJob,

    #[error("Already submitted result")]
    AlreadySubmitted,

    #[error("Job expired")]
    JobExpired,

    #[error("Consensus not reached")]
    ConsensusNotReached,

    #[error("Invalid FHE operation")]
    InvalidOperation,

    #[error("Unauthorized")]
    Unauthorized,
}

impl From<FheGeneratorError> for ProgramError {
    fn from(e: FheGeneratorError) -> Self {
        ProgramError::Custom(e as u32 + 200)
    }
}
