use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Program-specific errors
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZyberLinkProgramError {
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

    #[error("Marketplace not initialized")]
    MarketplaceNotInitialized,

    #[error("Invalid price")]
    InvalidPrice,

    // FHE-specific errors
    #[error("Missing FHE consensus config for FHE job")]
    MissingFheConfig,

    #[error("Invalid FHE consensus configuration")]
    InvalidFheConfig,

    #[error("Unexpected FHE config for non-FHE job")]
    UnexpectedFheConfig,

    #[error("FHE job already fully claimed")]
    FheJobFullyClaimed,

    #[error("Prover already claimed this FHE job")]
    ProverAlreadyClaimed,

    #[error("Not an FHE job")]
    NotFheJob,

    #[error("Prover did not claim this FHE job")]
    ProverNotClaimed,

    #[error("Result already submitted by this prover")]
    ResultAlreadySubmitted,

    #[error("Insufficient FHE results for finalization")]
    InsufficientFheResults,

    #[error("Job already finalized")]
    AlreadyFinalized,

    #[error("Invalid job status for this operation")]
    InvalidJobStatus,

    #[error("Missing FHE consensus account")]
    MissingFheConsensusAccount,
}

impl From<ZyberLinkProgramError> for ProgramError {
    fn from(e: ZyberLinkProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub type ProgramResult<T = ()> = Result<T, ProgramError>;
