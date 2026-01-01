//! Ciphertext Fetcher
//!
//! Fetches encrypted bet amounts from the app server.
//! Ciphertexts are stored off-chain due to their size (~500KB each).

use anyhow::{anyhow, Context, Result};
use fhe_client_sdk::FutarchyFheClient;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Response from the app server when fetching a ciphertext
#[derive(Debug, Serialize, Deserialize)]
pub struct CiphertextResponse {
    /// The ciphertext bytes (base64 encoded in JSON)
    pub ciphertext: String,
    /// Hash of the ciphertext for verification
    pub hash: String,
    /// Market ID this ciphertext belongs to
    pub market_id: String,
    /// Whether this is a pool or a bet
    pub ciphertext_type: CiphertextType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CiphertextType {
    Pool,
    Bet,
}

/// Fetches ciphertexts from the app server
pub struct CiphertextFetcher {
    client: Client,
    base_url: String,
}

impl CiphertextFetcher {
    /// Create a new fetcher with the app server URL
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetch a ciphertext by its hash
    ///
    /// The hash is used as the identifier since it's what's stored on-chain.
    pub fn fetch_by_hash(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        let hash_hex = hex::encode(hash);
        let url = format!("{}/api/fhe/ciphertext/{}", self.base_url, hash_hex);

        log::debug!("Fetching ciphertext from: {}", url);

        let response: CiphertextResponse = self
            .client
            .get(&url)
            .send()
            .context("Failed to send request to app server")?
            .json()
            .context("Failed to parse ciphertext response")?;

        // Decode base64 ciphertext
        let ciphertext = base64_decode(&response.ciphertext)
            .context("Failed to decode base64 ciphertext")?;

        // Verify hash matches
        let actual_hash = FutarchyFheClient::hash_ciphertext(&ciphertext);
        if actual_hash != *hash {
            return Err(anyhow!(
                "Ciphertext hash mismatch! Expected: {}, Got: {}",
                hash_hex,
                hex::encode(actual_hash)
            ));
        }

        log::info!(
            "Fetched ciphertext: {} bytes, hash: {}",
            ciphertext.len(),
            hash_hex
        );

        Ok(ciphertext)
    }

    /// Fetch the current encrypted pool for a market
    pub fn fetch_pool(&self, market_id: &str, side: bool) -> Result<Vec<u8>> {
        let side_str = if side { "yes" } else { "no" };
        let url = format!(
            "{}/api/fhe/markets/{}/pool/{}",
            self.base_url, market_id, side_str
        );

        log::debug!("Fetching pool from: {}", url);

        let response: CiphertextResponse = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch pool")?
            .json()
            .context("Failed to parse pool response")?;

        let ciphertext = base64_decode(&response.ciphertext)
            .context("Failed to decode pool ciphertext")?;

        log::info!(
            "Fetched {} pool for market {}: {} bytes",
            side_str,
            market_id,
            ciphertext.len()
        );

        Ok(ciphertext)
    }

    /// Submit the updated pool back to the app server
    pub fn submit_pool_update(
        &self,
        market_id: &str,
        side: bool,
        new_pool_ciphertext: &[u8],
        job_id: u64,
    ) -> Result<()> {
        let side_str = if side { "yes" } else { "no" };
        let url = format!(
            "{}/api/fhe/markets/{}/pool/{}/update",
            self.base_url, market_id, side_str
        );

        let hash = FutarchyFheClient::hash_ciphertext(new_pool_ciphertext);

        let payload = serde_json::json!({
            "ciphertext": base64_encode(new_pool_ciphertext),
            "hash": hex::encode(hash),
            "job_id": job_id,
        });

        log::debug!("Submitting pool update to: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .context("Failed to submit pool update")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Pool update failed with status {}: {}",
                status,
                body
            ));
        }

        log::info!(
            "Submitted pool update for market {} ({}), job_id: {}",
            market_id,
            side_str,
            job_id
        );

        Ok(())
    }
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode(s)
        .map_err(|e| anyhow!("Base64 decode error: {}", e))
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ciphertext_type_serde() {
        let pool = CiphertextType::Pool;
        let bet = CiphertextType::Bet;

        let pool_json = serde_json::to_string(&pool).unwrap();
        let bet_json = serde_json::to_string(&bet).unwrap();

        assert_eq!(pool_json, "\"pool\"");
        assert_eq!(bet_json, "\"bet\"");
    }
}
