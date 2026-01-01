//! HTTP client for Futarchy FHE endpoints on blink-server
//!
//! Provides interface for encrypted bet placement using FHE.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};

const DEFAULT_FUTARCHY_URL: &str = "http://localhost:9000";

/// Futarchy FHE client for encrypted betting
pub struct FutarchyClient {
    base_url: String,
    client: reqwest::Client,
}

impl FutarchyClient {
    /// Create client with default URL (localhost:9000)
    pub fn new() -> Self {
        Self::with_url(DEFAULT_FUTARCHY_URL)
    }

    /// Create client with custom URL
    pub fn with_url(url: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Prepare an encrypted bet transaction (step 1 of 2)
    ///
    /// Returns an unsigned transaction that the client must sign.
    pub async fn prepare_bet(&self, req: PrepareBetRequest) -> Result<PrepareBetResponse> {
        let url = format!("{}/api/futarchy/bet/prepare", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send prepare_bet request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Server returned error {}: {}",
                status,
                error_text
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse prepare_bet response")
    }

    /// Submit signed transaction with ciphertext (step 2 of 2)
    ///
    /// Sends the signed TX and full ciphertext to the server.
    /// Server will confirm TX on-chain and store ciphertext for FHE processing.
    pub async fn submit_bet(&self, req: SubmitBetRequest) -> Result<SubmitBetResponse> {
        let url = format!("{}/api/futarchy/bet/submit", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send submit_bet request")?;

        let status = response.status();
        let raw_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        // Check for error in response body (server may return 200 with error JSON)
        if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&raw_body) {
            if error_obj.get("error").is_some() {
                let error_msg = error_obj["error"].as_str().unwrap_or("Unknown error");
                return Err(anyhow!("Transaction failed: {}", error_msg));
            }
        }

        if !status.is_success() {
            return Err(anyhow!(
                "Server returned error {}: {}",
                status,
                raw_body
            ));
        }

        serde_json::from_str(&raw_body)
            .context("Failed to parse submit_bet response")
    }

    /// Get market details
    pub async fn get_market(&self, market_id: &str) -> Result<MarketResponse> {
        let url = format!("{}/api/futarchy/markets/{}", self.base_url, market_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch market")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Server returned error {}: {}",
                status,
                error_text
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse market response")
    }

    /// List all markets
    pub async fn list_markets(&self) -> Result<Vec<MarketResponse>> {
        let url = format!("{}/api/futarchy/markets", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list markets")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Server returned error {}: {}",
                status,
                error_text
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse markets list")
    }

    /// Check API health
    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/api/futarchy/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to check health")?;

        if !response.status().is_success() {
            return Err(anyhow!("Health check failed with status {}", response.status()));
        }

        response
            .json()
            .await
            .context("Failed to parse health response")
    }
}

impl Default for FutarchyClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PrepareBetRequest {
    pub market_id: String,
    pub bettor: String,
    pub side: bool,  // true = YES, false = NO
    pub amount_lamports: u64,
    pub ciphertext_hash: String,  // SHA256/Keccak256 hex of ciphertext
    pub proof: String,  // ZK proof (base64)
    pub public_inputs: String,  // Public inputs (base64)
    pub circuit_type: u8,
}

#[derive(Debug, Serialize)]
pub struct SubmitBetRequest {
    pub signed_tx: String,  // Base64 encoded signed transaction
    pub ciphertext: String,  // Base64 encoded FHE ciphertext
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_key: Option<String>,  // Base64 encoded FHE server key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<u64>,  // Market ID for FHE job creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<bool>,  // true = YES, false = NO for FHE job
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PrepareBetResponse {
    pub unsigned_transaction: String,  // Base64 encoded
    pub market_id: String,
    pub signers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitBetResponse {
    pub tx_signature: String,
    pub status: String,
    pub ciphertext_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct MarketResponse {
    pub id: String,
    pub question: String,
    pub creator: String,
    pub oracle: String,
    pub status: String,
    pub yes_pool_lamports: i64,
    pub no_pool_lamports: i64,
    pub outcome: Option<bool>,
    pub created_at: String,
    pub ends_at: Option<String>,
    pub settled_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub markets_count: i64,
}

// =============================================================================
// Helper functions
// =============================================================================

/// Encode bytes to base64 for API requests
pub fn encode_base64(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

/// Decode base64 string to bytes
pub fn decode_base64(s: &str) -> Result<Vec<u8>> {
    BASE64.decode(s).context("Failed to decode base64")
}
