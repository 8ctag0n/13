//! Starknet chain client implementation
//!
//! Wraps zyberlink-chain-client StarknetClient into the simplified ChainClient trait.

use super::ChainClient;
use anyhow::Result;
use async_trait::async_trait;

/// Starknet blockchain client
///
/// This client uses the shared zyberlink-chain-client library for Starknet operations.
pub struct StarknetChainClient {
    rpc_url: String,
    account_address: Option<String>,
    private_key: Option<String>,
}

impl StarknetChainClient {
    /// Create a new Starknet chain client
    ///
    /// # Arguments
    /// * `rpc_url` - Starknet RPC endpoint
    /// * `account_address` - Optional account address (for signing operations)
    /// * `private_key` - Optional private key (for signing operations)
    pub fn new(
        rpc_url: String,
        account_address: Option<String>,
        private_key: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            rpc_url,
            account_address,
            private_key,
        })
    }

    /// Get the inner StarknetClient from zyberlink-chain-client
    #[cfg(feature = "starknet")]
    fn inner_client(&self) -> Result<zyberlink_chain_client::StarknetClient> {
        use anyhow::Context;
        zyberlink_chain_client::StarknetClient::new(&self.rpc_url)
            .context("Failed to create Starknet client")
    }

    /// Ensure account credentials are available
    fn ensure_credentials(&self) -> Result<(&str, &str)> {
        let account = self.account_address.as_ref()
            .ok_or_else(|| anyhow::anyhow!(
                "Starknet account address required. Set --starknet-account or STARKNET_ACCOUNT"
            ))?;

        let private_key = self.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!(
                "Starknet private key required. Set --starknet-private-key or STARKNET_PRIVATE_KEY"
            ))?;

        Ok((account, private_key))
    }
}

#[async_trait]
impl ChainClient for StarknetChainClient {
    fn chain_id(&self) -> &str {
        "starknet"
    }

    async fn get_balance(&self, address: &str) -> Result<u64> {
        #[cfg(not(feature = "starknet"))]
        {
            let _ = address;
            anyhow::bail!(
                "Starknet support not enabled. Rebuild with --features starknet"
            );
        }

        #[cfg(feature = "starknet")]
        {
            use zyberlink_chain_client::ChainClient as InnerChainClient;
            use anyhow::Context;

            let client = self.inner_client()?;
            client.get_balance(address).await
                .context("Failed to get Starknet balance")
        }
    }

    async fn send_payment(&self, to: &str, amount: u64) -> Result<String> {
        #[cfg(not(feature = "starknet"))]
        {
            let _ = (to, amount);
            anyhow::bail!(
                "Starknet support not enabled. Rebuild with --features starknet"
            );
        }

        #[cfg(feature = "starknet")]
        {
            use zyberlink_chain_client::ChainClient as InnerChainClient;
            use anyhow::Context;

            let (account, private_key) = self.ensure_credentials()?;
            let client = self.inner_client()?;

            // For Starknet, we need to call the ETH transfer function
            // This is a simplified version - in production, you'd use the transfer method
            // from the inner client or build a proper transfer transaction

            // Use execute_contract to call the ETH transfer function
            // ETH contract address on Starknet (standard across networks)
            let eth_contract = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";

            // transfer function selector (starknet_keccak("transfer"))
            let transfer_selector = "0x83afd3f4caedc6eebf44246fe54e38c95e3179a5ec9ea81740eca5b482d12e";

            // Build calldata: [recipient, amount_low, amount_high]
            // For u64, high part is 0
            let calldata = serde_json::json!([
                to,
                format!("{:#x}", amount),
                "0x0"
            ]);

            let args = serde_json::json!({
                "calldata": calldata.as_array().unwrap(),
                "private_key": private_key
            });

            let args_bytes = serde_json::to_vec(&args)?;

            client.execute_contract(
                eth_contract,
                transfer_selector,
                &args_bytes,
                account,
            ).await
                .context("Failed to send Starknet payment")
        }
    }

    fn native_symbol(&self) -> &str {
        "ETH"
    }

    fn format_amount(&self, amount: u64) -> String {
        let eth = wei_to_eth(amount);
        format!("{:.18} ETH", eth)
    }

    async fn health_check(&self) -> Result<bool> {
        #[cfg(not(feature = "starknet"))]
        {
            Ok(false)
        }

        #[cfg(feature = "starknet")]
        {
            use zyberlink_chain_client::ChainClient as InnerChainClient;
            use anyhow::Context;

            let client = self.inner_client()?;
            client.health_check().await
                .context("Starknet health check failed")
        }
    }
}

/// Convert wei to ETH (18 decimals)
pub fn wei_to_eth(wei: u64) -> f64 {
    wei as f64 / 1_000_000_000_000_000_000.0
}

/// Convert ETH to wei
pub fn eth_to_wei(eth: f64) -> u64 {
    (eth * 1_000_000_000_000_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wei_conversion() {
        assert_eq!(wei_to_eth(1_000_000_000_000_000_000), 1.0);
        assert_eq!(wei_to_eth(500_000_000_000_000_000), 0.5);
        assert_eq!(eth_to_wei(1.0), 1_000_000_000_000_000_000);
        assert_eq!(eth_to_wei(0.5), 500_000_000_000_000_000);
    }

    #[test]
    fn test_client_creation() {
        let client = StarknetChainClient::new(
            "http://localhost:5050".to_string(),
            None,
            None,
        );
        assert!(client.is_ok());
        assert_eq!(client.unwrap().chain_id(), "starknet");
    }
}
