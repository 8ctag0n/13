//! Aptos blockchain client implementation
//!
//! This module provides a concrete implementation of the ChainClient trait
//! for Aptos blockchain using the REST API directly.

use crate::{ChainClient, ChainClientError, Result, TransactionStatus};
use crate::signature::SignatureVerifier;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "aptos")]
use reqwest::Client;

/// Aptos chain client
///
/// Provides integration with Aptos blockchain via the REST API.
pub struct AptosClient {
    rpc_url: String,
    faucet_url: Option<String>,
    network: String,
    #[cfg(feature = "aptos")]
    client: Client,
}

#[derive(Debug, Deserialize)]
struct LedgerInfo {
    chain_id: u64,
    ledger_version: String,
    ledger_timestamp: String,
    node_role: String,
    block_height: String,
    #[serde(default)]
    git_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountData {
    #[serde(default)]
    sequence_number: String,
    #[serde(default)]
    authentication_key: String,
}

#[derive(Debug, Deserialize)]
struct AccountResource {
    #[serde(rename = "type")]
    resource_type: String,
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct TransactionInfo {
    #[serde(rename = "type")]
    txn_type: String,
    hash: String,
    #[serde(default)]
    success: bool,
    #[serde(default)]
    vm_status: String,
}

impl AptosClient {
    /// Create a new Aptos client
    ///
    /// # Arguments
    /// * `rpc_url` - Aptos REST API endpoint URL
    /// * `faucet_url` - Optional faucet endpoint for testnet/devnet
    #[cfg(feature = "aptos")]
    pub fn new(rpc_url: String, faucet_url: Option<String>) -> Result<Self> {
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else if rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1") {
            "local"
        } else {
            "devnet"
        }
        .to_string();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ChainClientError::Network(e.to_string()))?;

        Ok(Self {
            rpc_url,
            faucet_url,
            network,
            client,
        })
    }

    #[cfg(not(feature = "aptos"))]
    pub fn new(rpc_url: String, faucet_url: Option<String>) -> Result<Self> {
        Ok(Self {
            rpc_url,
            faucet_url,
            network: "unknown".to_string(),
        })
    }

    /// Create an Aptos client for local testnet
    pub fn local() -> Result<Self> {
        Self::new(
            "http://localhost:8080".to_string(),
            Some("http://localhost:9081".to_string()),
        )
    }

    /// Create an Aptos client for devnet
    pub fn devnet() -> Result<Self> {
        Self::new(
            "https://fullnode.devnet.aptoslabs.com/v1".to_string(),
            Some("https://faucet.devnet.aptoslabs.com".to_string()),
        )
    }

    /// Create an Aptos client for testnet
    pub fn testnet() -> Result<Self> {
        Self::new(
            "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
            Some("https://faucet.testnet.aptoslabs.com".to_string()),
        )
    }

    /// Create an Aptos client for mainnet
    pub fn mainnet() -> Result<Self> {
        Self::new(
            "https://fullnode.mainnet.aptoslabs.com/v1".to_string(),
            None,
        )
    }

    #[cfg(feature = "aptos")]
    fn validate_address(&self, address: &str) -> Result<String> {
        // Aptos addresses are 64-char hex strings (32 bytes)
        // Allow 0x prefix or not
        let addr = address.strip_prefix("0x").unwrap_or(address);

        if addr.len() != 64 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ChainClientError::InvalidAddress(
                "Invalid Aptos address format (expected 64 hex chars)".to_string(),
            ));
        }

        Ok(format!("0x{}", addr))
    }

    #[cfg(feature = "aptos")]
    async fn get_account_resource(
        &self,
        address: &str,
        resource_type: &str,
    ) -> Result<serde_json::Value> {
        let addr = self.validate_address(address)?;
        let url = format!(
            "{}/accounts/{}/resource/{}",
            self.rpc_url, addr, resource_type
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ChainClientError::Network(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let resource: AccountResource = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        Ok(resource.data)
    }
}

#[async_trait]
impl ChainClient for AptosClient {
    fn chain_id(&self) -> &str {
        "aptos"
    }

    fn network(&self) -> &str {
        &self.network
    }

    #[cfg(feature = "aptos")]
    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/v1", self.rpc_url.trim_end_matches("/v1"));

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => Ok(true),
            Ok(_) => Ok(false),
            Err(e) => {
                log::warn!("Aptos health check failed: {}", e);
                Ok(false)
            }
        }
    }

    #[cfg(not(feature = "aptos"))]
    async fn health_check(&self) -> Result<bool> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_balance(&self, address: &str) -> Result<u64> {
        // Get APT balance from 0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>
        match self
            .get_account_resource(address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>")
            .await
        {
            Ok(data) => {
                let coin_value = data
                    .get("coin")
                    .and_then(|c| c.get("value"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChainClientError::Deserialization(
                            "Failed to parse balance".to_string(),
                        )
                    })?;

                coin_value
                    .parse::<u64>()
                    .map_err(|e| ChainClientError::Deserialization(e.to_string()))
            }
            Err(_) => {
                // Account doesn't exist or no APT balance
                Ok(0)
            }
        }
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_balance(&self, _address: &str) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_account_data(&self, address: &str) -> Result<Vec<u8>> {
        let addr = self.validate_address(address)?;
        let url = format!("{}/accounts/{}", self.rpc_url, addr);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ChainClientError::Network(format!(
                "HTTP {}: Account not found",
                response.status()
            )));
        }

        let account_data: AccountData = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        serde_json::to_vec(&account_data)
            .map_err(|e| ChainClientError::Serialization(e.to_string()))
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_account_data(&self, _address: &str) -> Result<Vec<u8>> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn account_exists(&self, address: &str) -> Result<bool> {
        let addr = self.validate_address(address)?;
        let url = format!("{}/accounts/{}", self.rpc_url, addr);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(feature = "aptos"))]
    async fn account_exists(&self, _address: &str) -> Result<bool> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn send_transaction(&self, transaction: &[u8]) -> Result<String> {
        let url = format!("{}/transactions", self.rpc_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x.aptos.signed_transaction+bcs")
            .body(transaction.to_vec())
            .send()
            .await
            .map_err(|e| ChainClientError::TransactionFailed(format!("Submit failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChainClientError::TransactionFailed(format!(
                "Transaction submission failed: {}",
                error_text
            )));
        }

        let txn_info: TransactionInfo = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        Ok(txn_info.hash)
    }

    #[cfg(not(feature = "aptos"))]
    async fn send_transaction(&self, _transaction: &[u8]) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn confirm_transaction(&self, hash: &str, timeout_secs: u64) -> Result<bool> {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            match self.get_transaction_status(hash).await? {
                TransactionStatus::Confirmed => return Ok(true),
                TransactionStatus::Failed => return Ok(false),
                TransactionStatus::Pending | TransactionStatus::Unknown => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(false)
    }

    #[cfg(not(feature = "aptos"))]
    async fn confirm_transaction(&self, _hash: &str, _timeout_secs: u64) -> Result<bool> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_transaction_status(&self, hash: &str) -> Result<TransactionStatus> {
        let url = format!("{}/transactions/by_hash/{}", self.rpc_url, hash);

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let txn_info: TransactionInfo = response
                    .json()
                    .await
                    .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

                if txn_info.success {
                    Ok(TransactionStatus::Confirmed)
                } else {
                    Ok(TransactionStatus::Failed)
                }
            }
            Ok(response) if response.status().as_u16() == 404 => {
                // Transaction not found yet
                Ok(TransactionStatus::Pending)
            }
            Ok(response) => Err(ChainClientError::Network(format!(
                "Unexpected status: {}",
                response.status()
            ))),
            Err(e) => Err(ChainClientError::Network(format!(
                "Request failed: {}",
                e
            ))),
        }
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_transaction_status(&self, _hash: &str) -> Result<TransactionStatus> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    async fn transfer(&self, _from: &str, _to: &str, _amount: u64) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Direct transfers require transaction signing - use aptos-sdk or build transaction manually".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn call_contract(
        &self,
        _module_address: &str,
        function_name: &str,
        args: &[u8],
    ) -> Result<Vec<u8>> {
        // View function call
        let url = format!("{}/view", self.rpc_url);

        let args_json: Vec<serde_json::Value> = serde_json::from_slice(args)
            .unwrap_or_else(|_| vec![]);

        let payload = serde_json::json!({
            "function": function_name,
            "type_arguments": [],
            "arguments": args_json
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("View call failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChainClientError::Network(format!(
                "View call error: {}",
                error_text
            )));
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        serde_json::to_vec(&result)
            .map_err(|e| ChainClientError::Serialization(e.to_string()))
    }

    #[cfg(not(feature = "aptos"))]
    async fn call_contract(
        &self,
        _module_address: &str,
        _function_name: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    async fn execute_contract(
        &self,
        _program_address: &str,
        _method: &str,
        _args: &[u8],
        _signer: &str,
    ) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Entry functions require transaction signing - use aptos-sdk or build transaction manually".to_string(),
        ))
    }

    async fn subscribe_account(&self, _address: &str) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Aptos doesn't support WebSocket subscriptions - use polling".to_string(),
        ))
    }

    async fn unsubscribe(&self, _subscription_id: u64) -> Result<()> {
        Err(ChainClientError::NotImplemented(
            "Aptos doesn't support WebSocket subscriptions".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_block_height(&self) -> Result<u64> {
        let url = format!("{}/v1", self.rpc_url.trim_end_matches("/v1"));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("Request failed: {}", e)))?;

        let ledger_info: LedgerInfo = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        ledger_info
            .block_height
            .parse::<u64>()
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_block_height(&self) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_recent_blockhash(&self) -> Result<String> {
        let url = format!("{}/v1", self.rpc_url.trim_end_matches("/v1"));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("Request failed: {}", e)))?;

        let ledger_info: LedgerInfo = response
            .json()
            .await
            .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

        // Return ledger version as "blockhash" equivalent
        Ok(ledger_info.ledger_version)
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_recent_blockhash(&self) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    async fn estimate_fee(&self, _transaction: &[u8]) -> Result<u64> {
        // Aptos uses gas units, typical transaction costs ~2000 gas units
        // At 100 gas units per APT octa, this is ~0.002 APT
        Ok(2000)
    }
}

#[cfg(all(test, feature = "aptos"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aptos_client_creation() {
        let client = AptosClient::local();
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.chain_id(), "aptos");
        assert_eq!(client.network(), "local");
    }

    #[tokio::test]
    async fn test_address_validation() {
        let client = AptosClient::local().unwrap();

        // Valid address with 0x prefix
        let valid = client.validate_address("0x0000000000000000000000000000000000000000000000000000000000000001");
        assert!(valid.is_ok());

        // Valid address without 0x prefix
        let valid = client.validate_address("0000000000000000000000000000000000000000000000000000000000000001");
        assert!(valid.is_ok());

        // Invalid - too short
        let invalid = client.validate_address("0x01");
        assert!(invalid.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = AptosClient::local().unwrap();
        // This will fail if local node is not running, which is expected in CI
        let _ = client.health_check().await;
    }

    #[tokio::test]
    async fn test_get_block_height() {
        let client = AptosClient::local().unwrap();
        // This will fail if local node is not running
        let _ = client.get_block_height().await;
    }
}

impl SignatureVerifier for AptosClient {
    fn verify_signature(
        &self,
        public_key: &str,
        signature: &str,
        message: &[u8],
    ) -> Result<bool> {
        use ed25519_dalek::{Signature, VerifyingKey, Verifier};

        // Decode hex public key
        let pubkey_bytes = hex::decode(public_key)
            .map_err(|e| ChainClientError::InvalidAddress(
                format!("Invalid hex pubkey: {}", e)
            ))?;

        // Decode hex signature
        let sig_bytes = hex::decode(signature)
            .map_err(|e| ChainClientError::InvalidSignature(
                format!("Invalid hex signature: {}", e)
            ))?;

        // Verify using ed25519
        let verifying_key = VerifyingKey::from_bytes(
            pubkey_bytes.as_slice().try_into()
                .map_err(|_| ChainClientError::InvalidAddress(
                    "Invalid pubkey length (expected 32 bytes)".to_string()
                ))?
        ).map_err(|e| ChainClientError::InvalidAddress(
            format!("Invalid pubkey: {}", e)
        ))?;

        let sig = Signature::from_bytes(
            sig_bytes.as_slice().try_into()
                .map_err(|_| ChainClientError::InvalidSignature(
                    "Invalid signature length (expected 64 bytes)".to_string()
                ))?
        );

        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    fn signature_encoding(&self) -> &str {
        "hex"
    }

    fn signature_algorithm(&self) -> &str {
        "ed25519"
    }
}
