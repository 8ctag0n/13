use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a temporary job data entry stored in the database
/// before on-chain confirmation
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TempJobData {
    pub id: i64,
    pub job_id: i64,
    pub creator_pubkey: String,
    pub encrypted_data: Vec<u8>,
    pub server_key: Vec<u8>,
    pub operation: String,
    pub operation_value: i16,
    pub price_lamports: i64,
    pub required_provers: i16,
    pub consensus_threshold: i16,
    pub status: String,
    pub payment_method: String,             // "SOL" or "wZEC"
    pub payment_token_mint: Option<String>, // SPL token mint for wZEC
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Enum representing the different states a job can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Job data validated, waiting for on-chain transaction
    PendingTx,
    /// Job created on-chain and active for provers
    Active,
    /// Job has been claimed by a prover
    Claimed,
    /// Prover is computing the result
    Computing,
    /// Job completed successfully
    Completed,
    /// Job failed (timeout, invalid computation, etc.)
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::PendingTx => "pending_tx",
            JobStatus::Active => "active",
            JobStatus::Claimed => "claimed",
            JobStatus::Computing => "computing",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_tx" => Some(JobStatus::PendingTx),
            "active" => Some(JobStatus::Active),
            "claimed" => Some(JobStatus::Claimed),
            "computing" => Some(JobStatus::Computing),
            "completed" => Some(JobStatus::Completed),
            "failed" => Some(JobStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a used nonce for anti-replay protection
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct UsedNonce {
    pub nonce: String,
    pub used_at: DateTime<Utc>,
}

/// Data needed to insert a new pending job
#[derive(Debug, Clone)]
pub struct InsertJobData {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub encrypted_data: Vec<u8>,
    pub server_key: Vec<u8>,
    pub operation: String,
    pub operation_value: i16,
    pub price_lamports: i64,
    pub required_provers: i16,
    pub consensus_threshold: i16,
    pub payment_method: String,             // "SOL" or "wZEC"
    pub payment_token_mint: Option<String>, // SPL token mint for wZEC payments
}
