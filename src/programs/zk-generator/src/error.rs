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
}

impl From<ZkGeneratorError> for ProgramError {
    fn from(e: ZkGeneratorError) -> Self {
        ProgramError::Custom(e as u32 + 100)
    }
}
