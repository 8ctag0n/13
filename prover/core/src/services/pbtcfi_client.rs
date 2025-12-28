//! pBTCFi Job Client
//!
//! HTTP client for interacting with blink-server's pBTCFi endpoints.
//! Allows provers to discover, claim, and complete FHE verification jobs.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Pending FHE job from blink-server
#[derive(Debug, Clone, Deserialize)]
pub struct PbtcfiPendingJob {
    pub loan_id: String,
    pub borrower: String,
    pub btc_encrypted_c1: String,
    pub btc_encrypted_c2: String,
    pub btc_commitment: String,
    pub created_at: i64,
}

/// Request to claim a job
#[derive(Debug, Serialize)]
struct ClaimJobRequest {
    loan_id: String,
    prover_id: String,
}

/// Request to complete a job
#[derive(Debug, Serialize)]
struct CompleteJobRequest {
    loan_id: String,
    collateral_value_c1: String,
    collateral_value_c2: String,
    plst_amount_c1: String,
    plst_amount_c2: String,
}

/// Response from claim/complete operations
#[derive(Debug, Deserialize)]
pub struct JobOperationResponse {
    pub success: bool,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Client for pBTCFi job operations
pub struct PbtcfiJobClient {
    client: Client,
    base_url: String,
    prover_id: String,
}

impl PbtcfiJobClient {
    /// Create a new pBTCFi job client
    ///
    /// # Arguments
    /// * `base_url` - Base URL for blink-server (e.g., "http://localhost:8080")
    /// * `prover_id` - Unique identifier for this prover
    pub fn new(base_url: &str, prover_id: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            prover_id: prover_id.to_string(),
        }
    }

    /// Get pending FHE jobs for pBTCFi loans
    ///
    /// Returns a list of loans that need FHE verification.
    pub async fn get_pending_jobs(&self, limit: u32) -> Result<Vec<PbtcfiPendingJob>> {
        let url = format!(
            "{}/internal/pbtcfi/pending-jobs?limit={}",
            self.base_url, limit
        );

        log::debug!("Fetching pending pBTCFi jobs from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch pending jobs: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to fetch pending jobs: {} - {}",
                status,
                body
            ));
        }

        let jobs: Vec<PbtcfiPendingJob> = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse pending jobs response: {}", e))?;

        log::debug!("Found {} pending pBTCFi jobs", jobs.len());
        Ok(jobs)
    }

    /// Claim a job for processing
    ///
    /// Atomically claims a job to prevent other provers from working on it.
    /// Returns true if claim was successful, false if job was already claimed.
    pub async fn claim_job(&self, loan_id: &str) -> Result<bool> {
        let url = format!("{}/internal/pbtcfi/claim-job", self.base_url);

        log::info!("Claiming pBTCFi job: {}", loan_id);

        let request = ClaimJobRequest {
            loan_id: loan_id.to_string(),
            prover_id: self.prover_id.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to claim job: {}", e))?;

        let status = response.status();

        if status.is_success() {
            let result: JobOperationResponse = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse claim response: {}", e))?;

            log::info!("Job {} claim result: success={}", loan_id, result.success);
            Ok(result.success)
        } else if status == reqwest::StatusCode::CONFLICT {
            // Job already claimed by another prover
            log::warn!("Job {} already claimed by another prover", loan_id);
            Ok(false)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(anyhow!("Failed to claim job: {} - {}", status, body))
        }
    }

    /// Complete a job with FHE computation results
    ///
    /// Submits the encrypted pLST amount after FHE computation.
    pub async fn complete_job(
        &self,
        loan_id: &str,
        collateral_value_c1: &str,
        collateral_value_c2: &str,
        plst_amount_c1: &str,
        plst_amount_c2: &str,
    ) -> Result<()> {
        let url = format!("{}/internal/pbtcfi/complete-job", self.base_url);

        log::info!("Completing pBTCFi job: {}", loan_id);

        let request = CompleteJobRequest {
            loan_id: loan_id.to_string(),
            collateral_value_c1: collateral_value_c1.to_string(),
            collateral_value_c2: collateral_value_c2.to_string(),
            plst_amount_c1: plst_amount_c1.to_string(),
            plst_amount_c2: plst_amount_c2.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to complete job: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to complete job: {} - {}", status, body));
        }

        let result: JobOperationResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse complete response: {}", e))?;

        if result.success {
            log::info!("Job {} completed successfully", loan_id);
            Ok(())
        } else {
            Err(anyhow!(
                "Job completion failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    /// Fail a job with error message
    ///
    /// Reports that FHE computation failed for this job.
    pub async fn fail_job(&self, loan_id: &str, error: &str) -> Result<()> {
        let url = format!("{}/internal/pbtcfi/fail-job", self.base_url);

        log::warn!("Failing pBTCFi job {}: {}", loan_id, error);

        let request = serde_json::json!({
            "loan_id": loan_id,
            "error": error,
        });

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to report job failure: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("Failed to report job failure: {} - {}", status, body);
            // Don't return error - failing to report failure is not critical
        }

        Ok(())
    }

    /// Get the prover ID
    pub fn prover_id(&self) -> &str {
        &self.prover_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PbtcfiJobClient::new("http://localhost:8080", "test-prover-123");
        assert_eq!(client.prover_id(), "test-prover-123");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_url_normalization() {
        let client = PbtcfiJobClient::new("http://localhost:8080/", "test-prover");
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
