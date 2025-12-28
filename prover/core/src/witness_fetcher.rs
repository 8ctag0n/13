use anyhow::{Context, Result};
use log::{debug, info};
use reqwest::Client;

/// Client for interacting with witness storage backend
pub struct WitnessFetcher {
    backend_url: String,
    client: Client,
}

impl WitnessFetcher {
    pub fn new(backend_url: String) -> Self {
        Self {
            backend_url,
            client: Client::new(),
        }
    }

    /// Upload encrypted witness to backend
    /// Returns the commitment (Blake2b hash) of the witness
    pub async fn upload_witness(&self, encrypted_witness: &[u8]) -> Result<[u8; 32]> {
        let url = format!("{}/witness", self.backend_url);

        debug!("Uploading {} bytes to {}", encrypted_witness.len(), url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .body(encrypted_witness.to_vec())
            .send()
            .await
            .context("Failed to upload witness to backend")?;

        if !response.status().is_success() {
            anyhow::bail!("Upload failed with status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        let commitment_hex = json["commitment"]
            .as_str()
            .context("Missing commitment in response")?;

        let commitment_bytes =
            hex::decode(commitment_hex).context("Invalid commitment hex format")?;

        if commitment_bytes.len() != 32 {
            anyhow::bail!("Invalid commitment length: {}", commitment_bytes.len());
        }

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&commitment_bytes);

        info!("Uploaded witness, commitment: {}", commitment_hex);
        Ok(commitment)
    }

    /// Download encrypted witness from backend using commitment
    /// Retries on 404 with exponential backoff (chain sync may not have run yet)
    pub async fn download_witness(&self, commitment: &[u8; 32]) -> Result<Vec<u8>> {
        let commitment_hex = hex::encode(commitment);
        let url = format!("{}/witness/{}", self.backend_url, commitment_hex);

        // Retry configuration: wait for chain sync to populate witness_hash
        const MAX_RETRIES: u32 = 5;
        const INITIAL_DELAY_MS: u64 = 2000; // 2 seconds

        for attempt in 0..MAX_RETRIES {
            debug!("Downloading witness from {} (attempt {}/{})", url, attempt + 1, MAX_RETRIES);

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to download witness from backend")?;

            if response.status() == 404 {
                if attempt < MAX_RETRIES - 1 {
                    let delay = INITIAL_DELAY_MS * (1 << attempt); // Exponential backoff
                    info!(
                        "Witness not found (attempt {}/{}), retrying in {}ms (waiting for chain sync)...",
                        attempt + 1, MAX_RETRIES, delay
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    continue;
                }
                anyhow::bail!("Witness not found for commitment: {} (after {} retries)", commitment_hex, MAX_RETRIES);
            }

            if !response.status().is_success() {
                anyhow::bail!("Download failed with status: {}", response.status());
            }

            let encrypted_witness = response
                .bytes()
                .await
                .context("Failed to read witness bytes")?
                .to_vec();

            info!(
                "Downloaded witness: {} bytes (commitment: {})",
                encrypted_witness.len(),
                commitment_hex
            );

            return Ok(encrypted_witness);
        }

        anyhow::bail!("Witness download failed after {} retries", MAX_RETRIES);
    }

    /// Check if backend is healthy
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let url = format!("{}/health", self.backend_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to reach backend")?;

        if !response.status().is_success() {
            anyhow::bail!("Health check failed: {}", response.status());
        }

        let json = response
            .json()
            .await
            .context("Failed to parse health response")?;

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires witness-storage backend running
    async fn test_upload_download_roundtrip() {
        let fetcher = WitnessFetcher::new("http://localhost:8080".to_string());

        // Upload
        let witness_data = b"test encrypted witness data";
        let commitment = fetcher
            .upload_witness(witness_data)
            .await
            .expect("Upload should succeed");

        // Download
        let downloaded = fetcher
            .download_witness(&commitment)
            .await
            .expect("Download should succeed");

        assert_eq!(downloaded, witness_data);
    }

    #[tokio::test]
    #[ignore] // Requires witness-storage backend running
    async fn test_download_nonexistent() {
        let fetcher = WitnessFetcher::new("http://localhost:8080".to_string());

        let fake_commitment = [0u8; 32];
        let result = fetcher.download_witness(&fake_commitment).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Witness not found"));
    }

    #[tokio::test]
    #[ignore] // Requires witness-storage backend running
    async fn test_health_check() {
        let fetcher = WitnessFetcher::new("http://localhost:8080".to_string());

        let health = fetcher.health_check().await.expect("Health check failed");

        assert_eq!(health["status"], "ok");
    }
}
