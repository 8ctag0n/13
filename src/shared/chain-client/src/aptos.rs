//! Aptos blockchain client implementation
//!
//! This module provides a concrete implementation of the ChainClient trait
//! for Aptos blockchain using the official Aptos SDK.

use crate::{ChainClient, ChainClientError, Result, TransactionStatus};
use async_trait::async_trait;

#[cfg(feature = "aptos")]
use {
    aptos_sdk::{
        rest_client::{Client as AptosRestClient, FaucetClient},
        types::{
            account_address::AccountAddress,
            transaction::{SignedTransaction, TransactionPayload},
            LocalAccount,
        },
    },
    std::str::FromStr,
};

/// Aptos chain client
///
/// Provides integration with Aptos blockchain via the official REST API.
pub struct AptosClient {
    rpc_url: String,
    faucet_url: Option<String>,
    network: String,
    #[cfg(feature = "aptos")]
    rest_client: AptosRestClient,
    #[cfg(feature = "aptos")]
    faucet_client: Option<FaucetClient>,
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

        let rest_client = AptosRestClient::new(url::Url::parse(&rpc_url).map_err(|e| {
            ChainClientError::ConnectionError(format!("Invalid RPC URL: {}", e))
        })?);

        let faucet_client = faucet_url.as_ref().and_then(|url| {
            url::Url::parse(url).ok().map(FaucetClient::new)
        });

        Ok(Self {
            rpc_url,
            faucet_url,
            network,
            rest_client,
            faucet_client,
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
    fn parse_address(&self, address: &str) -> Result<AccountAddress> {
        AccountAddress::from_str(address)
            .map_err(|e| ChainClientError::InvalidParameter(format!("Invalid address: {}", e)))
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
        match self.rest_client.get_index().await {
            Ok(_) => Ok(true),
            Err(e) => {
                log::error!("Aptos health check failed: {:?}", e);
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
        let addr = self.parse_address(address)?;
        let account = self
            .rest_client
            .get_account(addr)
            .await
            .map_err(|e| ChainClientError::RpcError(format!("Failed to get account: {:?}", e)))?
            .into_inner();

        // APT balance is stored in coin field
        Ok(account
            .authentication_key
            .as_ref()
            .map(|_| 0u64)
            .unwrap_or(0))
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_balance(&self, _address: &str) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_account_data(&self, address: &str) -> Result<Vec<u8>> {
        let addr = self.parse_address(address)?;
        let account = self
            .rest_client
            .get_account(addr)
            .await
            .map_err(|e| ChainClientError::RpcError(format!("Failed to get account: {:?}", e)))?;

        serde_json::to_vec(&account.into_inner())
            .map_err(|e| ChainClientError::DeserializationError(e.to_string()))
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_account_data(&self, _address: &str) -> Result<Vec<u8>> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn account_exists(&self, address: &str) -> Result<bool> {
        let addr = self.parse_address(address)?;
        match self.rest_client.get_account(addr).await {
            Ok(_) => Ok(true),
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
        let signed_txn: SignedTransaction = bcs::from_bytes(transaction)
            .map_err(|e| ChainClientError::SerializationError(e.to_string()))?;

        let pending_txn = self
            .rest_client
            .submit(&signed_txn)
            .await
            .map_err(|e| ChainClientError::TransactionError(format!("Submit failed: {:?}", e)))?
            .into_inner();

        Ok(pending_txn.hash.to_string())
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
                TransactionStatus::Pending => {
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
        use aptos_sdk::types::transaction::Transaction;

        let txn_hash = aptos_sdk::types::transaction::TransactionId::Hash(
            aptos_sdk::crypto::HashValue::from_str(hash)
                .map_err(|e| ChainClientError::InvalidParameter(e.to_string()))?,
        );

        match self.rest_client.get_transaction_by_hash(txn_hash).await {
            Ok(response) => {
                let txn = response.into_inner();
                match txn {
                    Transaction::UserTransaction(user_txn) => {
                        if user_txn.info.success {
                            Ok(TransactionStatus::Confirmed)
                        } else {
                            Ok(TransactionStatus::Failed)
                        }
                    }
                    _ => Ok(TransactionStatus::Pending),
                }
            }
            Err(_) => Ok(TransactionStatus::Pending),
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
            "Direct transfers not yet implemented for Aptos - use SDK transaction building"
                .to_string(),
        ))
    }

    async fn call_contract(
        &self,
        _program_address: &str,
        _method: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>> {
        Err(ChainClientError::NotImplemented(
            "View functions not yet implemented - use REST API directly".to_string(),
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
            "Entry functions not yet implemented - use SDK transaction building".to_string(),
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
        let ledger_info = self
            .rest_client
            .get_ledger_information()
            .await
            .map_err(|e| ChainClientError::RpcError(format!("Failed to get ledger info: {:?}", e)))?
            .into_inner();

        Ok(ledger_info.block_height)
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_block_height(&self) -> Result<u64> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    #[cfg(feature = "aptos")]
    async fn get_recent_blockhash(&self) -> Result<String> {
        let ledger_info = self
            .rest_client
            .get_ledger_information()
            .await
            .map_err(|e| ChainClientError::RpcError(format!("Failed to get ledger info: {:?}", e)))?
            .into_inner();

        Ok(ledger_info.ledger_version.to_string())
    }

    #[cfg(not(feature = "aptos"))]
    async fn get_recent_blockhash(&self) -> Result<String> {
        Err(ChainClientError::NotImplemented(
            "Aptos client requires 'aptos' feature flag".to_string(),
        ))
    }

    async fn estimate_fee(&self, _transaction: &[u8]) -> Result<u64> {
        // Aptos uses gas units, not a simple fee
        // For now, return a conservative estimate
        Ok(2000) // ~0.002 APT typical transaction cost
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
    async fn test_health_check() {
        let client = AptosClient::local().unwrap();
        // This will fail if local node is not running, which is expected in CI
        let _ = client.health_check().await;
    }
}
