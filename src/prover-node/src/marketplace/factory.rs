//! Marketplace factory for chain selection
//!
//! Creates the appropriate marketplace implementation based on configuration.

use super::{MarketplaceOperations, SolanaMarketplace};
use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::sync::Arc;

/// Supported blockchain types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainType {
    Solana,
    // Future: Aptos, Starknet
}

impl ChainType {
    /// Parse chain type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solana" | "sol" => Some(ChainType::Solana),
            // "aptos" | "apt" => Some(ChainType::Aptos),
            // "starknet" | "strk" => Some(ChainType::Starknet),
            _ => None,
        }
    }

    /// Get chain type as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainType::Solana => "solana",
        }
    }
}

impl Default for ChainType {
    fn default() -> Self {
        ChainType::Solana
    }
}

/// Configuration for creating a marketplace client
#[derive(Debug, Clone)]
pub struct MarketplaceConfig {
    /// Chain type
    pub chain: ChainType,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Network name (mainnet, devnet, testnet, localnet)
    pub network: String,
    /// Program/contract address
    pub program_address: String,
}

impl MarketplaceConfig {
    /// Create config for Solana
    pub fn solana(rpc_url: &str, program_id: &str) -> Self {
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("devnet") {
            "devnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else {
            "localnet"
        };

        Self {
            chain: ChainType::Solana,
            rpc_url: rpc_url.to_string(),
            network: network.to_string(),
            program_address: program_id.to_string(),
        }
    }
}

/// Factory for creating marketplace clients
pub struct MarketplaceFactory;

impl MarketplaceFactory {
    /// Create a marketplace client based on configuration
    ///
    /// Returns a trait object that can be used polymorphically,
    /// or the concrete type for chain-specific operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = MarketplaceConfig::solana(
    ///     "https://api.devnet.solana.com",
    ///     "YourProgramId...",
    /// );
    /// let marketplace = MarketplaceFactory::create(&config, keypair)?;
    /// ```
    pub fn create(
        config: &MarketplaceConfig,
        keypair: Arc<Keypair>,
    ) -> Result<Arc<dyn MarketplaceOperations>> {
        match config.chain {
            ChainType::Solana => {
                let program_id: Pubkey = config
                    .program_address
                    .parse()
                    .context("Invalid Solana program ID")?;

                let marketplace =
                    SolanaMarketplace::new(&config.rpc_url, program_id, keypair)
                        .map_err(|e| anyhow::anyhow!("Failed to create SolanaMarketplace: {}", e))?;

                Ok(Arc::new(marketplace))
            }
        }
    }

    /// Create a Solana marketplace client directly (for Solana-specific operations)
    ///
    /// Use this when you need access to Solana-specific methods via `.inner()`.
    pub fn create_solana(
        rpc_url: &str,
        program_id: Pubkey,
        keypair: Arc<Keypair>,
    ) -> Result<Arc<SolanaMarketplace>> {
        let marketplace = SolanaMarketplace::new(rpc_url, program_id, keypair)
            .map_err(|e| anyhow::anyhow!("Failed to create SolanaMarketplace: {}", e))?;

        Ok(Arc::new(marketplace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_type_from_str() {
        assert_eq!(ChainType::from_str("solana"), Some(ChainType::Solana));
        assert_eq!(ChainType::from_str("SOL"), Some(ChainType::Solana));
        assert_eq!(ChainType::from_str("unknown"), None);
    }

    #[test]
    fn test_marketplace_config_solana() {
        let config = MarketplaceConfig::solana(
            "https://api.devnet.solana.com",
            "11111111111111111111111111111111",
        );
        assert_eq!(config.chain, ChainType::Solana);
        assert_eq!(config.network, "devnet");
    }
}
