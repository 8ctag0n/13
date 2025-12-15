//! Solana blockchain client implementation
//!
//! This module provides a concrete implementation of the ChainClient trait
//! for Solana blockchain using the official Solana SDK.

use crate::{ChainClient, ChainClientError, Result, TransactionStatus};
use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;

/// Solana chain client
///
/// Wraps the Solana RPC client and implements the ChainClient trait.
///
/// # Example
///
/// ```rust,no_run
/// use zyberlink_chain_client::{SolanaClient, ChainClient};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = SolanaClient::new("https://api.devnet.solana.com")?;
///
///     let balance = client.get_balance("your_address_here").await?;
///     println!("Balance: {} lamports", balance);
///
///     Ok(())
/// }
/// ```
pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    network: String,
}

impl SolanaClient {
    /// Create a new Solana client with RPC endpoint
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint URL
    ///
    /// # Example
    /// ```rust,no_run
    /// # use zyberlink_chain_client::SolanaClient;
    /// let client = SolanaClient::new("https://api.devnet.solana.com")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        // Determine network from URL
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else if rpc_url.contains("devnet") {
            "devnet"
        } else {
            "localnet"
        }
        .to_string();

        Ok(Self {
            rpc_client: Arc::new(rpc_client),
            network,
        })
    }

    /// Create a Solana client for devnet
    pub fn devnet() -> Result<Self> {
        Self::new("https://api.devnet.solana.com")
    }

    /// Create a Solana client for mainnet
    pub fn mainnet() -> Result<Self> {
        Self::new("https://api.mainnet-beta.solana.com")
    }

    /// Create a Solana client for localnet
    pub fn localnet() -> Result<Self> {
        Self::new("http://localhost:8899")
    }

    /// Parse Solana address string to Pubkey
    fn parse_address(address: &str) -> Result<Pubkey> {
        Pubkey::from_str(address)
            .map_err(|e| ChainClientError::InvalidAddress(format!("{}: {}", address, e)))
    }

    /// Parse Solana signature string
    fn parse_signature(signature: &str) -> Result<Signature> {
        Signature::from_str(signature)
            .map_err(|e| ChainClientError::InvalidSignature(format!("{}: {}", signature, e)))
    }
}

#[async_trait]
impl ChainClient for SolanaClient {
    fn chain_id(&self) -> &str {
        "solana"
    }

    fn network(&self) -> &str {
        &self.network
    }

    async fn health_check(&self) -> Result<bool> {
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            // Try to get version - if it works, node is healthy
            rpc_client
                .get_version()
                .map(|_| true)
                .map_err(|e| ChainClientError::Network(e.to_string()))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn get_balance(&self, address: &str) -> Result<u64> {
        let pubkey = Self::parse_address(address)?;
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            rpc_client
                .get_balance(&pubkey)
                .map_err(|e| ChainClientError::Network(e.to_string()))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn get_account_data(&self, address: &str) -> Result<Vec<u8>> {
        let pubkey = Self::parse_address(address)?;
        let rpc_client = Arc::clone(&self.rpc_client);
        let address_str = address.to_string();

        tokio::task::spawn_blocking(move || {
            rpc_client
                .get_account_data(&pubkey)
                .map_err(|e| ChainClientError::AccountNotFound(format!("{}: {}", address_str, e)))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn account_exists(&self, address: &str) -> Result<bool> {
        let pubkey = Self::parse_address(address)?;
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            rpc_client
                .get_account(&pubkey)
                .map(|_| true)
                .or_else(|_| Ok(false))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn send_transaction(&self, transaction: &[u8]) -> Result<String> {
        let tx_bytes = transaction.to_vec();
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            // Deserialize transaction
            let tx: Transaction = bincode::deserialize(&tx_bytes)
                .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

            // Send transaction
            let signature = rpc_client
                .send_transaction(&tx)
                .map_err(|e| ChainClientError::TransactionFailed(e.to_string()))?;

            Ok(signature.to_string())
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn confirm_transaction(&self, signature: &str, timeout_secs: u64) -> Result<bool> {
        let sig = Self::parse_signature(signature)?;
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            use std::time::{Duration, Instant};

            let start = Instant::now();
            let timeout = Duration::from_secs(timeout_secs);

            loop {
                match rpc_client.get_signature_status(&sig) {
                    Ok(Some(result)) => {
                        return match result {
                            Ok(_) => Ok(true),
                            Err(e) => Err(ChainClientError::TransactionFailed(e.to_string())),
                        };
                    }
                    Ok(None) => {
                        // Transaction not found yet, continue polling
                    }
                    Err(e) => {
                        return Err(ChainClientError::Network(e.to_string()));
                    }
                }

                if start.elapsed() > timeout {
                    return Err(ChainClientError::TransactionTimeout(timeout_secs));
                }

                // Sleep for 500ms before next poll
                std::thread::sleep(Duration::from_millis(500));
            }
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn get_transaction_status(&self, signature: &str) -> Result<TransactionStatus> {
        let sig = Self::parse_signature(signature)?;
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            match rpc_client.get_signature_status(&sig) {
                Ok(Some(result)) => match result {
                    Ok(_) => Ok(TransactionStatus::Confirmed),
                    Err(_) => Ok(TransactionStatus::Failed),
                },
                Ok(None) => Ok(TransactionStatus::Pending),
                Err(_) => Ok(TransactionStatus::Unknown),
            }
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn transfer(&self, _from: &str, _to: &str, _amount: u64) -> Result<String> {
        // Transfer requires signing, which needs a keypair
        // For now, this is not implemented as it requires wallet integration
        Err(ChainClientError::NotImplemented(
            "transfer requires wallet/keypair integration - use Rust SDK or wallet adapter"
                .to_string(),
        ))
    }

    async fn call_contract(
        &self,
        program_address: &str,
        _method: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>> {
        // On Solana, "calling" a program means reading account data
        // For actual program invocation, use execute_contract
        self.get_account_data(program_address).await
    }

    async fn execute_contract(
        &self,
        _program_address: &str,
        _method: &str,
        _args: &[u8],
        _signer: &str,
    ) -> Result<String> {
        // Execute contract requires building and signing transactions
        // This is complex and requires instruction building logic
        Err(ChainClientError::NotImplemented(
            "execute_contract requires instruction building - use Rust SDK or Anchor client"
                .to_string(),
        ))
    }

    async fn subscribe_account(&self, _address: &str) -> Result<u64> {
        // Account subscriptions require WebSocket connection
        Err(ChainClientError::NotImplemented(
            "subscribe_account requires WebSocket - use solana-client PubsubClient".to_string(),
        ))
    }

    async fn unsubscribe(&self, _subscription_id: u64) -> Result<()> {
        Err(ChainClientError::NotImplemented(
            "unsubscribe requires WebSocket - use solana-client PubsubClient".to_string(),
        ))
    }

    async fn get_block_height(&self) -> Result<u64> {
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            rpc_client
                .get_block_height()
                .map_err(|e| ChainClientError::Network(e.to_string()))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn get_recent_blockhash(&self) -> Result<String> {
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            rpc_client
                .get_latest_blockhash()
                .map(|hash| hash.to_string())
                .map_err(|e| ChainClientError::Network(e.to_string()))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }

    async fn estimate_fee(&self, transaction: &[u8]) -> Result<u64> {
        let tx_bytes = transaction.to_vec();
        let rpc_client = Arc::clone(&self.rpc_client);

        tokio::task::spawn_blocking(move || {
            // Deserialize transaction
            let tx: Transaction = bincode::deserialize(&tx_bytes)
                .map_err(|e| ChainClientError::Deserialization(e.to_string()))?;

            // Get fee for transaction
            rpc_client
                .get_fee_for_message(tx.message())
                .map_err(|e| ChainClientError::Network(e.to_string()))
        })
        .await
        .map_err(|e| ChainClientError::Generic(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_client_creation() {
        let client = SolanaClient::devnet().unwrap();
        assert_eq!(client.chain_id(), "solana");
        assert_eq!(client.network(), "devnet");
    }

    #[test]
    fn test_parse_valid_address() {
        let valid_address = "11111111111111111111111111111111";
        let result = SolanaClient::parse_address(valid_address);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_address() {
        let invalid_address = "invalid_address";
        let result = SolanaClient::parse_address(invalid_address);
        assert!(result.is_err());
    }

    // Note: Integration tests requiring actual RPC connection
    // should be in a separate integration test file
}
