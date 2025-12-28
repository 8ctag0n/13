//! Error types for the Unified SDK

use thiserror::Error;
use solana_sdk::pubkey::Pubkey;

/// Result type alias for the Unified SDK
pub type Result<T> = std::result::Result<T, UnifiedError>;

/// Unified error type for all SDK operations
#[derive(Debug, Error)]
pub enum UnifiedError {
    /// Solana RPC client error
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    /// Job not found at the specified address
    #[error("Job not found: {0}")]
    JobNotFound(Pubkey),

    /// Invalid job type for the requested operation
    #[error("Invalid job type for operation: {0}")]
    InvalidJobType(String),

    /// Prover is not eligible to claim this job
    #[error("Prover not eligible: {0}")]
    ProverNotEligible(String),

    /// Job account deserialization failed
    #[error("Failed to deserialize job account: {0}")]
    DeserializationError(String),

    /// Unknown program owner
    #[error("Unknown program owner: {0}")]
    UnknownProgram(Pubkey),

    /// Invalid account data
    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    /// Job is in invalid state for operation
    #[error("Invalid job state: {0}")]
    InvalidJobState(String),

    /// Generic I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Borsh serialization/deserialization error
    #[error("Borsh error: {0}")]
    BorshError(String),

    /// Custom error with message
    #[error("{0}")]
    Custom(String),
}
