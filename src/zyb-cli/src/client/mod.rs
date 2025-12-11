//! HTTP client for communicating with blink-server
//!
//! Provides a simple interface to interact with ZK job endpoints.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_SERVER_URL: &str = "http://localhost:3000";

/// ZK Job client for blink-server API
pub struct ZkClient {
    base_url: String,
    client: reqwest::Client,
}

impl ZkClient {
    /// Create a new ZK client with default URL
    pub fn new() -> Self {
        Self::with_url(DEFAULT_SERVER_URL)
    }

    /// Create a new ZK client with custom URL
    pub fn with_url(url: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get a price quote for a ZK circuit
    pub async fn get_quote(&self, circuit_type: u8, payer: &str) -> Result<QuoteResponse> {
        let url = format!(
            "{}/api/quote?circuit_type={}&payer={}",
            self.base_url, circuit_type, payer
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send quote request to server")?;

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
            .context("Failed to parse quote response from server")
    }

    /// Create a new ZK proof job (legacy method - uses validate-and-build endpoint)
    pub async fn create_job(&self, req: CreateJobRequest) -> Result<CreateJobResponse> {
        let url = format!("{}/api/jobs/zk/validate-and-build", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send request to server")?;

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
            .context("Failed to parse response from server")
    }

    /// Create a new ZK proof job with payment signature
    pub async fn create_job_with_payment(
        &self,
        req: CreateJobRequest,
        payment_signature: &str,
    ) -> Result<CreateJobResponse> {
        let url = format!("{}/api/jobs/zk/create", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("X-Payment-Signature", payment_signature)
            .json(&req)
            .send()
            .await
            .context("Failed to send request to server")?;

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
            .context("Failed to parse response from server")
    }

    /// Get status of a ZK job
    pub async fn get_job_status(&self, job_id: i64) -> Result<JobStatusResponse> {
        let url = format!("{}/api/jobs/zk/{}/status", self.base_url, job_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to server")?;

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
            .context("Failed to parse response from server")
    }

    /// Get full details of a ZK job
    pub async fn get_job_details(&self, job_id: i64) -> Result<JobDetailsResponse> {
        let url = format!("{}/api/jobs/zk/{}", self.base_url, job_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to server")?;

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
            .context("Failed to parse response from server")
    }

    /// List ZK jobs
    pub async fn list_jobs(&self, status: Option<&str>) -> Result<ListJobsResponse> {
        let mut url = format!("{}/api/jobs/zk", self.base_url);

        if let Some(s) = status {
            url.push_str(&format!("?status={}", s));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to server")?;

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
            .context("Failed to parse response from server")
    }
}

impl Default for ZkClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

#[derive(Debug, Serialize)]
pub struct CreateJobRequest {
    pub circuit_type: u8,
    pub witness_commitment: String,
    pub public_inputs: Vec<String>,
    pub creator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobResponse {
    pub job_id: i64,
    pub circuit_type: u8,
    pub transaction: String,
    pub status: String,
    pub price_lamports: i64,
}

#[derive(Debug, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: i64,
    pub status: String,
    pub circuit_type: i16,
    pub witness_commitment: String,
    pub proof_hash: Option<String>,
    pub prover_pubkey: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct JobDetailsResponse {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub circuit_type: i16,
    pub circuit_category: String,
    pub witness_commitment: String,
    pub public_inputs: serde_json::Value,
    pub proof_hash: Option<String>,
    pub status: String,
    pub price_lamports: i64,
    pub x402_token_id: Option<String>,
    pub create_tx_signature: Option<String>,
    pub confirm_tx_signature: Option<String>,
    pub prover_pubkey: Option<String>,
    pub claimed_at: Option<String>,
    pub timeout_seconds: i32,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobSummary>,
    pub count: usize,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    #[serde(rename = "type")]
    pub job_type: String,
}

#[derive(Debug, Deserialize)]
pub struct JobSummary {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub circuit_type: i16,
    pub status: String,
    pub price_lamports: i64,
    pub created_at: String,
}

// =============================================================================
// Circuit Information
// =============================================================================

/// Get circuit information by type
pub fn get_circuit_info(circuit_type: u8) -> Option<CircuitInfo> {
    match circuit_type {
        10 => Some(CircuitInfo {
            id: 10,
            name: "ProofOfInnocence",
            category: "Core",
            description: "Proof that a prover is not part of a blacklist",
            path: "circuits/poi/",
        }),
        20 => Some(CircuitInfo {
            id: 20,
            name: "PrivateVote",
            category: "Voting",
            description: "Private voting without PoI check",
            path: "circuits/vote/private_vote",
        }),
        21 => Some(CircuitInfo {
            id: 21,
            name: "PrivateVoteWithPoI",
            category: "Voting",
            description: "Private voting with PoI verification",
            path: "circuits/vote/private_vote_poi",
        }),
        30 => Some(CircuitInfo {
            id: 30,
            name: "MarketBet",
            category: "Market",
            description: "Private market bet placement",
            path: "circuits/market/market_bet",
        }),
        31 => Some(CircuitInfo {
            id: 31,
            name: "MarketBetWithPoI",
            category: "Market",
            description: "Market bet with PoI verification",
            path: "circuits/market/market_bet_poi",
        }),
        32 => Some(CircuitInfo {
            id: 32,
            name: "MarketClaim",
            category: "Market",
            description: "Claim market bet winnings",
            path: "circuits/market/market_claim",
        }),
        40 => Some(CircuitInfo {
            id: 40,
            name: "PortfolioCompliance",
            category: "Portfolio",
            description: "Verify portfolio compliance with rules",
            path: "circuits/portfolio/compliance",
        }),
        41 => Some(CircuitInfo {
            id: 41,
            name: "PortfolioNetWorth",
            category: "Portfolio",
            description: "Prove net worth range without revealing exact amount",
            path: "circuits/portfolio/net_worth",
        }),
        _ => None,
    }
}

/// Get all available circuits
pub fn list_all_circuits() -> Vec<CircuitInfo> {
    vec![
        get_circuit_info(10).unwrap(),
        get_circuit_info(20).unwrap(),
        get_circuit_info(21).unwrap(),
        get_circuit_info(30).unwrap(),
        get_circuit_info(31).unwrap(),
        get_circuit_info(32).unwrap(),
        get_circuit_info(40).unwrap(),
        get_circuit_info(41).unwrap(),
    ]
}

#[derive(Debug, Clone)]
pub struct CircuitInfo {
    pub id: u8,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub path: &'static str,
}
