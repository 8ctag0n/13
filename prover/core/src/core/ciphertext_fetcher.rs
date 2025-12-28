//! Async Ciphertext Fetcher for off-chain TFHE storage
//!
//! Fetches encrypted data from blink-server using payload_hash as key.
//! TFHE ciphertexts are ~65KB and don't fit in on-chain events.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::Deserialize;
use tiny_keccak::{Hasher, Keccak};

use crate::engines::TfheCiphertext;

/// Response from blink-server for ciphertext fetch
#[derive(Debug, Deserialize)]
struct CiphertextResponse {
    /// Base64-encoded ciphertext bytes
    ciphertext: String,
    /// Hash of the ciphertext (hex)
    hash: String,
    /// Size in bytes
    #[allow(dead_code)]
    size_bytes: usize,
}

/// Async fetcher for TFHE ciphertexts from off-chain storage
pub struct CiphertextFetcher {
    client: Client,
    base_url: String,
}

impl CiphertextFetcher {
    /// Create a new async fetcher with the blink-server URL
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch a TFHE ciphertext by its hash from blink-server
    ///
    /// Validates that the fetched data matches the expected hash.
    pub async fn fetch_by_hash(&self, hash: &[u8; 32]) -> Result<TfheCiphertext> {
        let hash_hex = hex::encode(hash);
        let url = format!("{}/api/fhe/ciphertext/{}", self.base_url, hash_hex);

        log::debug!("[CiphertextFetcher] Fetching from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to blink-server")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Blink-server returned {}: {}",
                status,
                body
            ));
        }

        let resp: CiphertextResponse = response
            .json()
            .await
            .context("Failed to parse ciphertext response")?;

        // Decode base64 ciphertext
        let data = STANDARD
            .decode(&resp.ciphertext)
            .context("Failed to decode base64 ciphertext")?;

        // Verify hash matches (using Keccak256 like blink-server)
        let actual_hash = {
            let mut hasher = Keccak::v256();
            hasher.update(&data);
            let mut output = [0u8; 32];
            hasher.finalize(&mut output);
            output
        };

        if actual_hash != *hash {
            return Err(anyhow!(
                "Ciphertext hash mismatch! Expected: {}, Got: {}",
                hash_hex,
                hex::encode(actual_hash)
            ));
        }

        log::info!(
            "[CiphertextFetcher] Fetched ciphertext: {} bytes, hash: {}",
            data.len(),
            &hash_hex[..16]
        );

        Ok(TfheCiphertext { data })
    }

    /// Fetch ciphertext with retry logic
    pub async fn fetch_with_retry(
        &self,
        hash: &[u8; 32],
        max_retries: u32,
    ) -> Result<TfheCiphertext> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self.fetch_by_hash(hash).await {
                Ok(ct) => return Ok(ct),
                Err(e) => {
                    log::warn!(
                        "[CiphertextFetcher] Attempt {}/{} failed: {}",
                        attempt + 1,
                        max_retries,
                        e
                    );
                    last_error = Some(e);

                    if attempt + 1 < max_retries {
                        // Exponential backoff: 100ms, 200ms, 400ms...
                        let delay = std::time::Duration::from_millis(100 * (1 << attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("No retry attempts made")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetcher_url_formatting() {
        let fetcher = CiphertextFetcher::new("http://localhost:8080/".to_string());
        assert_eq!(fetcher.base_url, "http://localhost:8080");

        let fetcher2 = CiphertextFetcher::new("http://localhost:8080".to_string());
        assert_eq!(fetcher2.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_hash_encoding() {
        let hash = [0xab; 32];
        let hex = hex::encode(&hash);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
    }
}
