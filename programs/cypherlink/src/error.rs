use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Program-specific errors
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CypherLinkProgramError {
    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Invalid account")]
    InvalidAccount,

    #[error("Account already initialized")]
    AlreadyInitialized,

    #[error("Account not initialized")]
    NotInitialized,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Insufficient stake")]
    InsufficientStake,

    #[error("Insufficient reputation")]
    InsufficientReputation,

    #[error("Job not pending")]
    JobNotPending,

    #[error("Job not claimed")]
    JobNotClaimed,

    #[error("Job already completed")]
    JobAlreadyCompleted,

    #[error("Job timed out")]
    JobTimedOut,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Prover inactive")]
    ProverInactive,

    #[error("Invalid prover")]
    InvalidProver,

    #[error("Arithmetic overflow")]
    Overflow,

    #[error("Invalid circuit type")]
    InvalidCircuitType,

    #[error("Invalid proof")]
    InvalidProof,

    #[error("Escrow error")]
    EscrowError,

    #[error("Invalid timestamp")]
    InvalidTimestamp,

    #[error("Marketplace paused")]
    MarketplacePaused,
}

impl From<CypherLinkProgramError> for ProgramError {
    fn from(e: CypherLinkProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub type ProgramResult<T = ()> = Result<T, ProgramError>;
