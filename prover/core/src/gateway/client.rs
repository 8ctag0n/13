use std::sync::Arc;
use solana_sdk::signature::{Keypair, Signer};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::auth::sign_gateway_request;

pub struct GatewayClient {
    gateway_url: String,
    keypair: Arc<Keypair>,
    http: reqwest::Client,
}

impl GatewayClient {
    pub fn new(gateway_url: String, keypair: Arc<Keypair>) -> Self {
        Self {
            gateway_url,
            keypair,
            http: reqwest::Client::new(),
        }
    }

    /// GET /gateway/prover/witness/{hash}
    pub async fn download_witness(&self, hash: &str) -> Result<Vec<u8>> {
        let path = format!("/gateway/prover/witness/{}", hash);
        let url = format!("{}{}", self.gateway_url, path);

        let auth = sign_gateway_request(&self.keypair, "GET", &path, &[]);

        let response = auth.apply_to_request(self.http.get(&url))
            .send()
            .await
            .context("Failed to send witness download request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Witness download failed: {} - {}", status, body));
        }

        let bytes = response.bytes().await.context("Failed to read witness bytes")?;
        Ok(bytes.to_vec())
    }

    /// POST /gateway/prover/zk/{job_id}/submit
    pub async fn submit_zk_proof(&self, job_id: u64, proof: &str, public_inputs: &[String]) -> Result<()> {
        let path = format!("/gateway/prover/zk/{}/submit", job_id);
        let url = format!("{}{}", self.gateway_url, path);

        let body = serde_json::json!({
            "proof": proof,
            "public_inputs": public_inputs,
            "prover_pubkey": self.keypair.pubkey().to_string()
        });
        let body_bytes = serde_json::to_vec(&body)?;

        let auth = sign_gateway_request(&self.keypair, "POST", &path, &body_bytes);

        let response = auth.apply_to_request(self.http.post(&url))
            .header("Content-Type", "application/json")
            .body(body_bytes)
            .send()
            .await
            .context("Failed to send ZK proof submit request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("ZK proof submit failed: {} - {}", status, body));
        }

        Ok(())
    }

    /// POST /gateway/prover/fhe/{job_id}/submit
    pub async fn submit_fhe_result(&self, job_id: u64, encrypted_result: &[u8]) -> Result<String> {
        let path = format!("/gateway/prover/fhe/{}/submit", job_id);
        let url = format!("{}{}", self.gateway_url, path);

        let auth = sign_gateway_request(&self.keypair, "POST", &path, encrypted_result);

        let response = auth.apply_to_request(self.http.post(&url))
            .header("Content-Type", "application/octet-stream")
            .body(encrypted_result.to_vec())
            .send()
            .await
            .context("Failed to send FHE result submit request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("FHE result submit failed: {} - {}", status, body));
        }

        #[derive(Deserialize)]
        struct FheResponse {
            success: bool,
            job_id: i64,
            prover: String,
            result_size: usize,
        }

        let resp: FheResponse = response.json().await.context("Failed to parse FHE response")?;
        Ok(format!("job_id={}, size={}", resp.job_id, resp.result_size))
    }
}
