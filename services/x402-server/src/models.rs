//! x402 Data Models

use serde::{Deserialize, Serialize};

// =============================================================================
// Chain Identifier
// =============================================================================

/// Supported chains for x402 gateway
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    #[default]
    Solana,
    Starknet,
    Aptos,
}

impl Chain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Chain::Solana => "solana",
            Chain::Starknet => "starknet",
            Chain::Aptos => "aptos",
        }
    }
}

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub circuit_type: u8,
    pub payer: String,
    #[serde(default)]
    pub chain: Chain,
}

#[derive(Debug, Deserialize)]
pub struct EstimateRequest {
    pub circuit_type: u8,
    #[serde(default)]
    pub chain: Chain,
}

#[derive(Debug, Deserialize)]
pub struct BuildPaymentRequest {
    pub quote_id: String,
    pub payer: String,
    pub amount: String,
    pub recipient: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmPaymentRequest {
    pub quote_id: String,
    pub signed_transaction: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateTokenRequest {
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub circuit_type: u8,
    pub witness_commitment: String,
    pub witness_size: u32,
    pub timeout_seconds: i64,
    pub payer: String,
    #[serde(default)]
    pub chain: Chain,
}

/// Request for creating a pBTCFi loan (multi-chain)
/// Includes ciphertext + signed deposit TX
#[derive(Debug, Deserialize)]
pub struct CreateLoanRequest {
    /// Target chain
    #[serde(default)]
    pub chain: Chain,
    /// Borrower address (format depends on chain)
    pub borrower: String,
    /// TFHE encrypted BTC amount (base64 encoded)
    pub ciphertext: String,
    /// Signed transaction for wBTC deposit (hex or base64 depending on chain)
    pub signed_deposit_tx: String,
    /// Optional: server key hash if already uploaded
    pub server_key_hash: Option<String>,
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

#[derive(Debug, Serialize)]
pub struct EstimateResponse {
    pub price_lamports: u64,
    pub price_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct BuildPaymentResponse {
    pub instruction: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ConfirmPaymentResponse {
    pub token_id: String,
    pub amount_paid: String,
    pub tx_signature: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ValidateTokenResponse {
    pub valid: bool,
    pub circuit_type: Option<u8>,
    pub used: bool,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TokenStatusResponse {
    pub valid: bool,
    pub used: bool,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct WitnessUploadResponse {
    pub commitment: String,
}

#[derive(Debug, Serialize)]
pub struct CreateJobResponse {
    pub job_id: i64,
    pub unsigned_transaction: String,
}

/// Response for pBTCFi loan creation
#[derive(Debug, Serialize)]
pub struct CreateLoanResponse {
    pub loan_id: String,
    pub chain: String,
    pub witness_commitment: String,
    pub deposit_tx_hash: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}
