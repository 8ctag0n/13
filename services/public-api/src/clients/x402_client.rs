use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct X402Client {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct QuoteRequest {
    pub circuit_type: u8,
    pub payer: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateTokenResponse {
    pub valid: bool,
    pub circuit_type: Option<u8>,
    pub used: bool,
    pub expires_at: Option<i64>,
}

impl X402Client {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Get base URL for proxying
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get HTTP client for proxying
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub async fn get_quote(&self, circuit_type: u8, payer: &str) -> Result<QuoteResponse, reqwest::Error> {
        let url = format!("{}/api/quote", self.base_url);
        self.client
            .post(&url)
            .json(&QuoteRequest {
                circuit_type,
                payer: payer.to_string(),
            })
            .send()
            .await?
            .json()
            .await
    }

    pub async fn validate_token(&self, token_id: &str) -> Result<ValidateTokenResponse, reqwest::Error> {
        let url = format!("{}/api/validate", self.base_url);
        self.client
            .post(&url)
            .json(&serde_json::json!({"token_id": token_id}))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn health_check(&self) -> Result<bool, reqwest::Error> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
