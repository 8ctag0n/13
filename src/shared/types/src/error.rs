use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that can occur in ZyberLink operations
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZyberLinkError {
    /// Invalid instruction data
    #[error("Invalid instruction data")]
    InvalidInstructionData,

    /// Invalid account
    #[error("Invalid account")]
    InvalidAccount,

    /// Account already initialized
    #[error("Account already initialized")]
    AlreadyInitialized,

    /// Account not initialized
    #[error("Account not initialized")]
    NotInitialized,

    /// Insufficient funds for operation
    #[error("Insufficient funds")]
    InsufficientFunds,

    /// Insufficient stake amount
    #[error("Insufficient stake amount")]
    InsufficientStake,

    /// Insufficient reputation score
    #[error("Insufficient reputation score")]
    InsufficientReputation,

    /// Job not found
    #[error("Job not found")]
    JobNotFound,

    /// Job is not in pending status
    #[error("Job is not pending")]
    JobNotPending,

    /// Job is not in claimed status
    #[error("Job is not claimed")]
    JobNotClaimed,

    /// Job has already been completed
    #[error("Job already completed")]
    JobAlreadyCompleted,

    /// Job has timed out
    #[error("Job has timed out")]
    JobTimedOut,

    /// Unauthorized operation
    #[error("Unauthorized")]
    Unauthorized,

    /// Prover is not active
    #[error("Prover is inactive")]
    ProverInactive,

    /// Prover not found
    #[error("Prover not found")]
    ProverNotFound,

    /// Invalid prover for this job
    #[error("Invalid prover for this job")]
    InvalidProver,

    /// Arithmetic overflow
    #[error("Arithmetic overflow")]
    Overflow,

    /// Invalid circuit type
    #[error("Invalid circuit type")]
    InvalidCircuitType,

    /// Invalid proof data
    #[error("Invalid proof data")]
    InvalidProof,

    /// Escrow error
    #[error("Escrow error")]
    EscrowError,

    /// Invalid timestamp
    #[error("Invalid timestamp")]
    InvalidTimestamp,

    /// Serialization error
    #[error("Serialization error")]
    SerializationError,
}

impl From<ZyberLinkError> for ProgramError {
    fn from(e: ZyberLinkError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<std::io::Error> for ZyberLinkError {
    fn from(_: std::io::Error) -> Self {
        ZyberLinkError::SerializationError
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let error = ZyberLinkError::InvalidAccount;
        let program_error: ProgramError = error.into();

        match program_error {
            ProgramError::Custom(code) => {
                assert_eq!(code, ZyberLinkError::InvalidAccount as u32);
            }
            _ => panic!("Expected Custom error"),
        }
    }

    #[test]
    fn test_error_display() {
        let error = ZyberLinkError::InsufficientFunds;
        assert_eq!(error.to_string(), "Insufficient funds");
    }
}
