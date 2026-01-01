//! Futarchy API Client
//!
//! HTTP client for fetching FHE job ciphertexts from blink-server.
//! This enables a hybrid flow: job discovery on-chain + ciphertext fetch via API.

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use log::{debug, info};

/// FHE job data with ciphertexts (from /api/futarchy/fhe-jobs/{id}/data endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FheJobData {
    /// Job ID (database primary key)
    pub job_id: i32,
    /// Market ID
    pub market_id: String,
    /// Bet side as string ("yes" or "no")
    pub side: String,
    /// Pool ciphertext hash (hex, null if first bet)
    pub pool_ciphertext_hash: Option<String>,
    /// Encrypted pool ciphertext (base64, null if first bet)
    pub pool_ciphertext: Option<String>,
    /// Bet ciphertext hash (hex)
    pub bet_ciphertext_hash: String,
    /// Encrypted bet ciphertext (base64)
    pub bet_ciphertext: Option<String>,
    /// Job status
    pub status: String,
    /// Created timestamp
    pub created_at: String,
}

/// Pending FHE job from API (from /api/futarchy/fhe-jobs/pending endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFheJob {
    /// Job ID (database primary key)
    pub id: i32,
    /// On-chain FHE Generator job ID (may be null for legacy jobs)
    pub job_id: Option<i64>,
    /// Market ID
    pub market_id: String,
    /// Bet side as string ("yes" or "no")
    pub side: String,
    /// Pool ciphertext hash
    pub pool_ciphertext_hash: String,
    /// Bet ciphertext hash
    pub bet_ciphertext_hash: String,
    /// Result ciphertext hash (null if not processed)
    pub result_ciphertext_hash: Option<String>,
    /// Job status
    pub status: String,
    /// Created timestamp
    pub created_at: String,
    /// Processed timestamp
    pub processed_at: Option<String>,
}

impl ApiFheJob {
    /// Convert side string to bool (true = yes, false = no)
    pub fn side_bool(&self) -> bool {
        self.side.to_lowercase() == "yes"
    }
}

/// Result submission payload
#[derive(Debug, Serialize)]
pub struct ResultSubmission {
    /// Base64-encoded result ciphertext (new pool)
    pub result_ciphertext: String,
    /// Prover pubkey (optional, for tracking)
    pub prover_pubkey: Option<String>,
}

/// Response from result submission
#[derive(Debug, Deserialize)]
pub struct ResultSubmissionResponse {
    pub message: String,
    pub job_id: i32,
    pub result_hash: String,
    pub market_id: String,
    pub side: String,
    pub version: i32,
}

/// Response wrapper for pending jobs list
#[derive(Debug, Deserialize)]
struct PendingJobsResponse {
    pub count: i32,
    pub jobs: Vec<ApiFheJob>,
}

/// Client for Futarchy blink-server API
pub struct FutarchyApiClient {
    client: Client,
    base_url: String,
}

impl FutarchyApiClient {
    /// Create a new API client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch pending FHE jobs from API
    pub fn fetch_pending_jobs(&self) -> Result<Vec<ApiFheJob>> {
        let url = format!("{}/api/futarchy/fhe-jobs/pending", self.base_url);

        debug!("Fetching pending jobs from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch pending jobs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "API returned error {}: {}",
                status,
                body
            ));
        }

        let response_data: PendingJobsResponse = response
            .json()
            .context("Failed to parse pending jobs response")?;

        info!("Fetched {} pending jobs from API", response_data.jobs.len());

        Ok(response_data.jobs)
    }

    /// Fetch job data (ciphertexts) by database job ID
    pub fn fetch_job_data(&self, db_job_id: i32) -> Result<FheJobData> {
        let url = format!("{}/api/futarchy/fhe-jobs/{}/data", self.base_url, db_job_id);

        debug!("Fetching job data from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to fetch job data for db_job_id {}", db_job_id))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "API returned error {} for job {}: {}",
                status,
                db_job_id,
                body
            ));
        }

        let job_data: FheJobData = response
            .json()
            .with_context(|| format!("Failed to parse job data for db_job_id {}", db_job_id))?;

        info!(
            "Fetched job data for db_job_id {}: market={}, side={}",
            db_job_id,
            job_data.market_id,
            job_data.side
        );

        Ok(job_data)
    }

    /// Submit job result to API
    pub fn submit_result(
        &self,
        db_job_id: i32,
        new_pool_ciphertext: &[u8],
        prover_pubkey: Option<String>,
    ) -> Result<ResultSubmissionResponse> {
        let url = format!("{}/api/futarchy/fhe-jobs/{}/result", self.base_url, db_job_id);

        let submission = ResultSubmission {
            result_ciphertext: base64_encode(new_pool_ciphertext),
            prover_pubkey,
        };

        debug!("Submitting result to: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&submission)
            .send()
            .with_context(|| format!("Failed to submit result for db_job_id {}", db_job_id))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "API returned error {} when submitting result for job {}: {}",
                status,
                db_job_id,
                body
            ));
        }

        let result: ResultSubmissionResponse = response
            .json()
            .with_context(|| format!("Failed to parse result submission response for db_job_id {}", db_job_id))?;

        info!(
            "Successfully submitted result for db_job_id {}: hash={}",
            db_job_id,
            &result.result_hash[..16.min(result.result_hash.len())]
        );

        Ok(result)
    }

    /// Decode base64 ciphertext
    pub fn decode_ciphertext(base64_str: &str) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD
            .decode(base64_str)
            .map_err(|e| anyhow!("Base64 decode error: {}", e))
    }

    /// Decode hex hash
    pub fn decode_hash(hex_str: &str) -> Result<[u8; 32]> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| anyhow!("Hex decode error: {}", e))?;

        if bytes.len() != 32 {
            return Err(anyhow!("Hash must be 32 bytes, got {}", bytes.len()));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hash() {
        let hex = "a".repeat(64); // 32 bytes in hex
        let result = FutarchyApiClient::decode_hash(&hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_decode_hash_invalid_length() {
        let hex = "aa";
        let result = FutarchyApiClient::decode_hash(&hex);
        assert!(result.is_err());
    }
}
