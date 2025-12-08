//! x402 Data Models

use serde::{Deserialize, Serialize};

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub circuit_type: u8,
    pub payer: String,
}

#[derive(Debug, Deserialize)]
pub struct EstimateRequest {
    pub circuit_type: u8,
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

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}
