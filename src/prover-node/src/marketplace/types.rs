//! Generic types for multi-chain marketplace operations
//!
//! These types abstract away chain-specific details, allowing prover-node
//! to work with any blockchain that implements the MarketplaceOperations trait.

use zyberlink_types::{CircuitType, JobStatus};

/// Source program for a job
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobSource {
    /// Legacy zyberlink program
    Legacy,
    /// New ZK-Generator program
    ZkGenerator,
    /// New FHE-Generator program
    FheGenerator,
    /// Starknet FHE Jobs contract
    Starknet,
}

/// Starknet-specific job type (maps to Cairo JobType enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StarknetJobType {
    /// pBTCFi: verify encrypted BTC collateral
    LoanVerification = 0,
    /// pLST: update encrypted token balance
    BalanceUpdate = 1,
    /// pLST: prove staking position
    StakeProof = 2,
    /// Generic: prove valid transfer
    TransferProof = 3,
    /// pBTCFi: check liquidation threshold
    LiquidationCheck = 4,
}

impl TryFrom<u8> for StarknetJobType {
    type Error = String;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(StarknetJobType::LoanVerification),
            1 => Ok(StarknetJobType::BalanceUpdate),
            2 => Ok(StarknetJobType::StakeProof),
            3 => Ok(StarknetJobType::TransferProof),
            4 => Ok(StarknetJobType::LiquidationCheck),
            _ => Err(format!("Unknown StarknetJobType: {}", value)),
        }
    }
}

/// Generic job data structure (chain-agnostic)
#[derive(Debug, Clone)]
pub struct JobData {
    /// Unique job identifier
    pub id: u64,
    /// Job creator address (string format, chain-agnostic)
    pub creator: String,
    /// Circuit type for this job
    pub circuit_type: CircuitType,
    /// Hash of witness data
    pub witness_hash: [u8; 32],
    /// Price in base units (lamports for Solana, octas for Aptos, etc.)
    pub price: u64,
    /// Current job status
    pub status: JobStatus,
    /// Assigned prover address (if claimed)
    pub prover: Option<String>,
    /// Unix timestamp of job creation
    pub created_at: i64,
    /// Unix timestamp of job timeout
    pub timeout_at: i64,
    /// Whether this is an FHE job
    pub is_fhe: bool,
    /// Job PDA/address as string (for chain-specific operations)
    pub address: String,
    /// Source program for this job
    pub source: JobSource,
    /// Program ID that owns this job (for new generators)
    pub program_id: Option<String>,

    // === Starknet-specific fields ===
    /// Starknet job type (only set for Starknet jobs)
    pub starknet_job_type: Option<StarknetJobType>,
    /// Encrypted data c1 component (felt252 as bytes)
    pub encrypted_c1: Option<[u8; 32]>,
    /// Encrypted data c2 component (felt252 as bytes)
    pub encrypted_c2: Option<[u8; 32]>,
    /// Payload hash from Cairo contract
    pub payload_hash: Option<[u8; 32]>,
}

/// Generic prover data structure
#[derive(Debug, Clone)]
pub struct ProverData {
    /// Prover authority address
    pub authority: String,
    /// Staked amount in base units
    pub stake: u64,
    /// Encryption public key (for FHE jobs)
    pub encryption_pubkey: Option<[u8; 32]>,
    /// Whether prover is currently active
    pub is_active: bool,
    /// Total jobs completed
    pub jobs_completed: u64,
    /// Total jobs failed
    pub jobs_failed: u64,
    /// Reputation score (0-100)
    pub reputation: u8,
    /// Registration timestamp
    pub registered_at: i64,
}

/// Transaction result (chain-agnostic)
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// Transaction signature/hash as string
    pub signature: String,
    /// Whether transaction succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Block height where transaction was included
    pub block_height: Option<u64>,
}

impl TransactionResult {
    /// Create a successful transaction result
    pub fn success(signature: String) -> Self {
        Self {
            signature,
            success: true,
            error: None,
            block_height: None,
        }
    }

    /// Create a successful transaction result with block height
    pub fn success_with_block(signature: String, block_height: u64) -> Self {
        Self {
            signature,
            success: true,
            error: None,
            block_height: Some(block_height),
        }
    }

    /// Create a failed transaction result
    pub fn failed(signature: String, error: String) -> Self {
        Self {
            signature,
            success: false,
            error: Some(error),
            block_height: None,
        }
    }
}

/// FHE consensus configuration
#[derive(Debug, Clone)]
pub struct FheConsensusConfig {
    /// Number of required provers
    pub required_provers: u8,
    /// Consensus threshold
    pub consensus_threshold: u8,
}

/// Marketplace error types
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Chain error: {0}")]
    ChainError(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Job not found: {0}")]
    JobNotFound(u64),

    #[error("Prover not found: {0}")]
    ProverNotFound(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("Network error: {0}")]
    Network(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Job already claimed")]
    JobAlreadyClaimed,

    #[error("Job expired")]
    JobExpired,

    #[error("Not authorized: {0}")]
    NotAuthorized(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Timeout waiting for confirmation")]
    Timeout,

    #[error("{0}")]
    Other(String),
}

/// Result type for marketplace operations
pub type Result<T> = std::result::Result<T, MarketplaceError>;

// Conversion implementations for common error types
impl From<anyhow::Error> for MarketplaceError {
    fn from(e: anyhow::Error) -> Self {
        MarketplaceError::Other(e.to_string())
    }
}

impl From<std::io::Error> for MarketplaceError {
    fn from(e: std::io::Error) -> Self {
        MarketplaceError::Network(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_result_success() {
        let result = TransactionResult::success("abc123".to_string());
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.signature, "abc123");
    }

    #[test]
    fn test_transaction_result_failed() {
        let result = TransactionResult::failed("abc123".to_string(), "timeout".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("timeout".to_string()));
    }

    #[test]
    fn test_marketplace_error_display() {
        let err = MarketplaceError::JobNotFound(42);
        assert_eq!(err.to_string(), "Job not found: 42");

        let err = MarketplaceError::InsufficientBalance {
            required: 1000,
            available: 500,
        };
        assert!(err.to_string().contains("1000"));
        assert!(err.to_string().contains("500"));
    }
}
