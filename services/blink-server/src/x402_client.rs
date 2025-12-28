//! x402 Client for communicating with x402-server
//!
//! This module provides HTTP client functionality to validate payment tokens
//! against the external x402-server.

use anyhow::Result;
use serde::{Deserialize, Serialize};

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct ValidateTokenRequest {
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateTokenResponse {
    pub valid: bool,
    pub circuit_type: Option<u8>,
    pub used: bool,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MarkUsedRequest {
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

#[derive(Debug, Deserialize)]
pub struct EstimateResponse {
    pub price_lamports: u64,
    pub price_sol: f64,
}

// =============================================================================
// x402 Client
// =============================================================================

/// HTTP client for x402-server
pub struct X402Client {
    base_url: String,
    client: reqwest::Client,
}

impl X402Client {
    /// Create a new x402 client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Validate a payment token
    ///
    /// Returns token validity, circuit_type, and usage status.
    pub async fn validate_token(&self, token_id: &str) -> Result<ValidateTokenResponse> {
        let url = format!("{}/api/validate", self.base_url);

        let response = self.client
            .post(&url)
            .json(&ValidateTokenRequest {
                token_id: token_id.to_string(),
            })
            .send()
            .await?
            .json::<ValidateTokenResponse>()
            .await?;

        Ok(response)
    }

    /// Mark a token as used
    pub async fn mark_token_used(&self, token_id: &str) -> Result<()> {
        let url = format!("{}/api/mark-used", self.base_url);

        self.client
            .post(&url)
            .json(&MarkUsedRequest {
                token_id: token_id.to_string(),
            })
            .send()
            .await?;

        Ok(())
    }

    /// Get a price quote
    pub async fn get_quote(&self, circuit_type: u8, payer: &str) -> Result<QuoteResponse> {
        let url = format!("{}/api/quote", self.base_url);

        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "circuit_type": circuit_type,
                "payer": payer
            }))
            .send()
            .await?
            .json::<QuoteResponse>()
            .await?;

        Ok(response)
    }

    /// Estimate price without creating a quote
    pub async fn estimate_price(&self, circuit_type: u8) -> Result<EstimateResponse> {
        let url = format!("{}/api/estimate", self.base_url);

        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "circuit_type": circuit_type
            }))
            .send()
            .await?
            .json::<EstimateResponse>()
            .await?;

        Ok(response)
    }

    /// Check if x402-server is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
