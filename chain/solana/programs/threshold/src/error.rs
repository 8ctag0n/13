//! Threshold program error types

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ThresholdError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Invalid prover")]
    InvalidProver,

    #[error("Invalid validator")]
    InvalidValidator,

    #[error("Key share request not found")]
    RequestNotFound,

    #[error("Key share request expired")]
    RequestExpired,

    #[error("Validator already responded to this request")]
    AlreadyResponded,

    #[error("Insufficient key share responses")]
    InsufficientResponses,

    #[error("Invalid job")]
    InvalidJob,

    #[error("Encrypted share too large")]
    ShareTooLarge,

    #[error("Request already completed")]
    RequestCompleted,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Job not claimed by prover")]
    JobNotClaimed,

    #[error("Validator not active")]
    ValidatorNotActive,

    #[error("Invalid CID format")]
    InvalidCid,

    #[error("Arithmetic overflow")]
    Overflow,
}

impl From<ThresholdError> for ProgramError {
    fn from(e: ThresholdError) -> Self {
        ProgramError::Custom(e as u32 + 300)
    }
}
