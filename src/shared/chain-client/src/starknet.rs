//! Starknet blockchain client implementation
//!
//! This module provides a concrete implementation of the ChainClient trait
//! for Starknet blockchain.
//!
//! **Status**: Placeholder - awaiting Starknet SDK integration

use crate::{ChainClient, ChainClientError, Result, TransactionStatus};
use crate::signature::SignatureVerifier;
use async_trait::async_trait;

/// Starknet chain client
///
/// Lightweight RPC client for Starknet using reqwest.
/// Implements minimal functionality needed for event polling.
pub struct StarknetClient {
    rpc_url: String,
    network: String,
    http_client: reqwest::Client,
}

impl StarknetClient {
    /// Create a new Starknet client
    ///
    /// # Arguments
    /// * `rpc_url` - Starknet RPC endpoint URL
    pub fn new(rpc_url: &str) -> Result<Self> {
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else {
            "devnet"
        }
        .to_string();

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ChainClientError::Generic(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            rpc_url: rpc_url.to_string(),
            network,
            http_client,
        })
    }

    /// Make a JSON-RPC call to Starknet
    pub(crate) async fn rpc_call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let response = self.http_client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChainClientError::Network(format!("RPC request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ChainClientError::Network(format!(
                "RPC returned status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let rpc_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ChainClientError::Generic(format!("Failed to parse response: {}", e)))?;

        // Check for RPC error
        if let Some(error) = rpc_response.get("error") {
            return Err(ChainClientError::Generic(format!("RPC error: {}", error)));
        }

        // Extract result
        let result = rpc_response
            .get("result")
            .ok_or_else(|| ChainClientError::Generic("No result in RPC response".to_string()))?;

        serde_json::from_value(result.clone())
            .map_err(|e| ChainClientError::Generic(format!("Failed to deserialize result: {}", e)))
    }

    /// Create a Starknet client for testnet
    pub fn testnet() -> Result<Self> {
        Self::new("https://alpha4.starknet.io")
    }
}

#[async_trait]
impl ChainClient for StarknetClient {
    fn chain_id(&self) -> &str {
        "starknet"
    }

    fn network(&self) -> &str {
        &self.network
    }

    async fn health_check(&self) -> Result<bool> {
        // Try to get block number as health check
        match self.rpc_call::<u64>("starknet_blockNumber", serde_json::json!([])).await {
            Ok(_) => Ok(true),
            Err(e) => {
                log::warn!("Starknet health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn get_balance(&self, _address: &str) -> Result<u64> {
        // For Starknet, balance is typically stored in an ERC-20 contract (STRK token)
        // For MVP, we'll return NotImplemented as this requires knowing the token contract
        // In production, this would query the balance of STRK token at the address
        Err(ChainClientError::NotImplemented(
            "get_balance requires STRK token contract address - use call_contract instead".to_string(),
        ))
    }

    async fn get_account_data(&self, _address: &str) -> Result<Vec<u8>> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn account_exists(&self, address: &str) -> Result<bool> {
        // Check if contract/account exists by getting class hash
        let params = serde_json::json!({
            "block_id": "latest",
            "contract_address": address
        });

        match self.rpc_call::<String>("starknet_getClassHashAt", params).await {
            Ok(_) => Ok(true),
            Err(ChainClientError::Generic(msg)) if msg.contains("Contract not found") => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn send_transaction(&self, _transaction: &[u8]) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn confirm_transaction(&self, _signature: &str, _timeout_secs: u64) -> Result<bool> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn get_transaction_status(&self, signature: &str) -> Result<TransactionStatus> {
        #[derive(serde::Deserialize)]
        struct TxReceipt {
            execution_status: Option<String>,
            finality_status: Option<String>,
        }

        let params = serde_json::json!({
            "transaction_hash": signature
        });

        let receipt: TxReceipt = self.rpc_call("starknet_getTransactionReceipt", params).await?;

        // Map Starknet status to our TransactionStatus
        match (receipt.execution_status.as_deref(), receipt.finality_status.as_deref()) {
            (Some("SUCCEEDED"), Some("ACCEPTED_ON_L2" | "ACCEPTED_ON_L1")) => Ok(TransactionStatus::Confirmed),
            (Some("REVERTED"), _) => Ok(TransactionStatus::Failed),
            (_, Some("NOT_RECEIVED" | "RECEIVED")) => Ok(TransactionStatus::Pending),
            _ => Ok(TransactionStatus::Unknown),
        }
    }

    async fn transfer(&self, _from: &str, _to: &str, _amount: u64) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn call_contract(
        &self,
        program_address: &str,
        method: &str,
        args: &[u8],
    ) -> Result<Vec<u8>> {
        // Deserialize args as JSON array
        let calldata: Vec<String> = serde_json::from_slice(args)
            .map_err(|e| ChainClientError::Deserialization(format!("Invalid calldata: {}", e)))?;

        let params = serde_json::json!({
            "request": {
                "contract_address": program_address,
                "entry_point_selector": method,
                "calldata": calldata
            },
            "block_id": "latest"
        });

        let result: Vec<String> = self.rpc_call("starknet_call", params).await?;

        // Serialize result back to bytes
        serde_json::to_vec(&result)
            .map_err(|e| ChainClientError::Generic(format!("Failed to serialize result: {}", e)))
    }

    async fn execute_contract(
        &self,
        _program_address: &str,
        _method: &str,
        _args: &[u8],
        _signer: &str,
    ) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn subscribe_account(&self, _address: &str) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn unsubscribe(&self, _subscription_id: u64) -> Result<()> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn get_block_height(&self) -> Result<u64> {
        self.rpc_call::<u64>("starknet_blockNumber", serde_json::json!([])).await
    }

    async fn get_recent_blockhash(&self) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }

    async fn estimate_fee(&self, _transaction: &[u8]) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Starknet client not yet implemented - awaiting Phase 2-6".to_string(),
        ))
    }
}

impl SignatureVerifier for StarknetClient {
    fn verify_signature(
        &self,
        _public_key: &str,
        _signature: &str,
        _message: &[u8],
    ) -> Result<bool> {
        // Starknet uses ECDSA over STARK curve
        // Full implementation requires starknet-crypto crate
        Err(ChainClientError::NotImplemented(
            "Starknet signature verification requires starknet-crypto dependency".to_string()
        ))
    }

    fn signature_encoding(&self) -> &str {
        "hex"
    }

    fn signature_algorithm(&self) -> &str {
        "ecdsa-stark"
    }
}
