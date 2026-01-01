//! Bedrock error types

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum BedrockError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Account not initialized")]
    UninitializedAccount,

    #[error("Account already initialized")]
    AlreadyInitialized,

    #[error("Invalid authority")]
    InvalidAuthority,

    #[error("Prover not found")]
    ProverNotFound,

    #[error("Prover already registered")]
    ProverAlreadyRegistered,

    #[error("Insufficient stake")]
    InsufficientStake,

    #[error("Arithmetic overflow")]
    ArithmeticOverflow,

    #[error("Validator not found")]
    ValidatorNotFound,

    #[error("Validator already registered")]
    ValidatorAlreadyRegistered,

    #[error("Endpoint too long")]
    EndpointTooLong,

    #[error("Validator not active")]
    ValidatorNotActive,
}

impl From<BedrockError> for ProgramError {
    fn from(e: BedrockError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
