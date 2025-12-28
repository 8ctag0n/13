//! Error types for chain client operations

use thiserror::Error;

/// Result type alias using ChainClientError
pub type Result<T> = std::result::Result<T, ChainClientError>;

/// Error types for chain client operations
#[derive(Error, Debug)]
pub enum ChainClientError {
    /// Network connection error
    #[error("Network error: {0}")]
    Network(String),

    /// Invalid address format
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Account not found on-chain
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Transaction timeout
    #[error("Transaction timeout after {0} seconds")]
    TransactionTimeout(u64),

    /// Insufficient balance
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    /// Contract/program call error
    #[error("Contract call failed: {0}")]
    ContractCallFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Feature not implemented for this chain
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),

    /// Subscription error
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("{0}")]
    Generic(String),
}

impl From<anyhow::Error> for ChainClientError {
    fn from(err: anyhow::Error) -> Self {
        ChainClientError::Generic(err.to_string())
    }
}

impl From<serde_json::Error> for ChainClientError {
    fn from(err: serde_json::Error) -> Self {
        ChainClientError::Serialization(err.to_string())
    }
}
