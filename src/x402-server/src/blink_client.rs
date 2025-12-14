//! HTTP Client for internal communication with blink-server

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Response from /internal/prover/{pubkey}/status
#[derive(Debug, Deserialize)]
pub struct ProverStatus {
    pub registered: bool,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub reputation: Option<i32>,
}

pub struct BlinkClient {
    client: Client,
    base_url: String,
}

impl BlinkClient {
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

    /// Proxy a request to blink-server
    pub async fn proxy_post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
        payment_token: Option<&str>,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url).json(body);

        if let Some(token) = payment_token {
            request = request.header("X-Payment-Token", token);
        }

        request.send().await?.json().await
    }

    /// Proxy a GET request to blink-server
    pub async fn proxy_get<R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        self.client.get(&url).send().await?.json().await
    }

    /// Health check for blink-server
    pub async fn health_check(&self) -> Result<bool, reqwest::Error> {
        let url = format!("{}/internal/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// GET /internal/witness/{hash}
    ///
    /// Download witness data from blink-server (binary)
    pub async fn get_witness(&self, hash: &str) -> Result<Vec<u8>, reqwest::Error> {
        let url = format!("{}/internal/witness/{}", self.base_url, hash);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        response.bytes().await.map(|b| b.to_vec())
    }

    /// POST /internal/zk/{job_id}/submit-proof
    ///
    /// Submit a ZK proof to blink-server
    pub async fn submit_zk_proof<T: Serialize>(
        &self,
        job_id: i64,
        prover: &str,
        body: &T,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/internal/zk/{}/submit-proof", self.base_url, job_id);
        let response = self.client
            .post(&url)
            .header("X-Prover-Pubkey", prover)
            .json(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(())
    }

    /// POST /internal/fhe-result?job_id={}&prover={}
    ///
    /// Submit FHE result to blink-server (binary data)
    pub async fn submit_fhe_result(
        &self,
        job_id: i64,
        prover: &str,
        data: &[u8],
    ) -> Result<(), reqwest::Error> {
        let url = format!(
            "{}/internal/fhe-result?job_id={}&prover={}",
            self.base_url, job_id, prover
        );
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        Ok(())
    }

    /// GET /internal/prover/{pubkey}/status
    ///
    /// Check if a prover is registered and active in blink-server.
    /// Used for authorization in x402-gateway.
    pub async fn check_prover_status(&self, pubkey: &str) -> Result<ProverStatus, reqwest::Error> {
        let url = format!("{}/internal/prover/{}/status", self.base_url, pubkey);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err());
        }

        response.json::<ProverStatus>().await
    }
}
