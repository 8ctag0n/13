//! Marketplace factory for chain selection
//!
//! Creates the appropriate marketplace implementation based on configuration.

use super::{AptosMarketplace, MarketplaceOperations, SolanaMarketplace, StarknetMarketplace};
use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::sync::Arc;
use zyberlink_chain_client::{AptosClient, StarknetClient};

/// Supported blockchain types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainType {
    Solana,
    Aptos,
    Starknet,
}

impl ChainType {
    /// Parse chain type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solana" | "sol" => Some(ChainType::Solana),
            "aptos" | "apt" => Some(ChainType::Aptos),
            "starknet" | "strk" => Some(ChainType::Starknet),
            _ => None,
        }
    }

    /// Get chain type as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainType::Solana => "solana",
            ChainType::Aptos => "aptos",
            ChainType::Starknet => "starknet",
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
    /// Program/contract address (legacy zyberlink program)
    pub program_address: String,
    /// Prover address (chain-specific format)
    pub prover_address: Option<String>,
    /// ZK Generator program ID (optional, for new architecture)
    pub zk_generator_program: Option<String>,
    /// FHE Generator program ID (optional, for new architecture)
    pub fhe_generator_program: Option<String>,
    /// Starknet private key for signing transactions (hex format)
    pub starknet_private_key: Option<String>,
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
            prover_address: None, // Will be derived from keypair
            zk_generator_program: None,
            fhe_generator_program: None,
            starknet_private_key: None,
        }
    }

    /// Set ZK and FHE generator program IDs (builder pattern)
    pub fn with_generators(mut self, zk_program: Option<&str>, fhe_program: Option<&str>) -> Self {
        self.zk_generator_program = zk_program.map(|s| s.to_string());
        self.fhe_generator_program = fhe_program.map(|s| s.to_string());
        self
    }

    /// Create config for Aptos
    pub fn aptos(rpc_url: &str, module_address: &str, prover_address: &str) -> Self {
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else {
            "devnet"
        };

        Self {
            chain: ChainType::Aptos,
            rpc_url: rpc_url.to_string(),
            network: network.to_string(),
            program_address: module_address.to_string(),
            prover_address: Some(prover_address.to_string()),
            zk_generator_program: None,
            fhe_generator_program: None,
            starknet_private_key: None,
        }
    }

    /// Create config for Starknet
    pub fn starknet(rpc_url: &str, contract_address: &str, prover_address: &str) -> Self {
        let network = if rpc_url.contains("mainnet") {
            "mainnet"
        } else if rpc_url.contains("testnet") {
            "testnet"
        } else {
            "devnet"
        };

        Self {
            chain: ChainType::Starknet,
            rpc_url: rpc_url.to_string(),
            network: network.to_string(),
            program_address: contract_address.to_string(),
            prover_address: Some(prover_address.to_string()),
            zk_generator_program: None,
            fhe_generator_program: None,
            starknet_private_key: None,
        }
    }

    /// Set Starknet private key for signing transactions (builder pattern)
    pub fn with_starknet_signer(mut self, private_key: &str) -> Self {
        self.starknet_private_key = Some(private_key.to_string());
        self
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

                // Parse optional generator program IDs
                let zk_generator = config.zk_generator_program
                    .as_ref()
                    .map(|s| s.parse::<Pubkey>())
                    .transpose()
                    .context("Invalid ZK Generator program ID")?;

                let fhe_generator = config.fhe_generator_program
                    .as_ref()
                    .map(|s| s.parse::<Pubkey>())
                    .transpose()
                    .context("Invalid FHE Generator program ID")?;

                // Use new_with_generators if generators are configured
                let marketplace = if zk_generator.is_some() || fhe_generator.is_some() {
                    log::info!(
                        "Creating SolanaMarketplace with generators: ZK={:?}, FHE={:?}",
                        zk_generator, fhe_generator
                    );
                    SolanaMarketplace::new_with_generators(
                        &config.rpc_url, program_id, keypair, zk_generator, fhe_generator
                    )
                } else {
                    SolanaMarketplace::new(&config.rpc_url, program_id, keypair)
                }.map_err(|e| anyhow::anyhow!("Failed to create SolanaMarketplace: {}", e))?;

                Ok(Arc::new(marketplace))
            }
            ChainType::Aptos => {
                let prover_address = config
                    .prover_address
                    .clone()
                    .context("Prover address required for Aptos")?;

                let client = AptosClient::new(config.rpc_url.clone(), None)
                    .map_err(|e| anyhow::anyhow!("Failed to create AptosClient: {}", e))?;

                let marketplace = AptosMarketplace::new(
                    Arc::new(client),
                    config.program_address.clone(),
                    prover_address,
                );

                Ok(Arc::new(marketplace))
            }
            ChainType::Starknet => {
                let prover_address = config
                    .prover_address
                    .clone()
                    .context("Prover address required for Starknet")?;

                let client = StarknetClient::new(&config.rpc_url)
                    .map_err(|e| anyhow::anyhow!("Failed to create StarknetClient: {}", e))?;

                // Use new_with_signer if private key is provided
                let marketplace = if let Some(ref private_key) = config.starknet_private_key {
                    log::info!(
                        "Creating StarknetMarketplace with signer for prover {}",
                        prover_address
                    );
                    StarknetMarketplace::new_with_signer(
                        Arc::new(client),
                        config.program_address.clone(),
                        prover_address,
                        private_key.clone(),
                    )
                } else {
                    log::warn!(
                        "Creating StarknetMarketplace without signer (read-only mode)"
                    );
                    StarknetMarketplace::new(
                        Arc::new(client),
                        config.program_address.clone(),
                        prover_address,
                    )
                };

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

    /// Create a Starknet marketplace client directly (for Starknet-specific operations)
    ///
    /// # Arguments
    /// * `rpc_url` - Starknet RPC endpoint URL
    /// * `contract_address` - PbtcfiJobs contract address
    /// * `prover_address` - Prover account address
    /// * `private_key` - Optional private key for signing transactions
    pub fn create_starknet(
        rpc_url: &str,
        contract_address: &str,
        prover_address: &str,
        private_key: Option<&str>,
    ) -> Result<Arc<StarknetMarketplace>> {
        let client = StarknetClient::new(rpc_url)
            .map_err(|e| anyhow::anyhow!("Failed to create StarknetClient: {}", e))?;

        let marketplace = if let Some(pk) = private_key {
            StarknetMarketplace::new_with_signer(
                Arc::new(client),
                contract_address.to_string(),
                prover_address.to_string(),
                pk.to_string(),
            )
        } else {
            StarknetMarketplace::new(
                Arc::new(client),
                contract_address.to_string(),
                prover_address.to_string(),
            )
        };

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
        assert_eq!(ChainType::from_str("aptos"), Some(ChainType::Aptos));
        assert_eq!(ChainType::from_str("APT"), Some(ChainType::Aptos));
        assert_eq!(ChainType::from_str("starknet"), Some(ChainType::Starknet));
        assert_eq!(ChainType::from_str("STRK"), Some(ChainType::Starknet));
        assert_eq!(ChainType::from_str("unknown"), None);
    }

    #[test]
    fn test_chain_type_as_str() {
        assert_eq!(ChainType::Solana.as_str(), "solana");
        assert_eq!(ChainType::Aptos.as_str(), "aptos");
        assert_eq!(ChainType::Starknet.as_str(), "starknet");
    }

    #[test]
    fn test_marketplace_config_solana() {
        let config = MarketplaceConfig::solana(
            "https://api.devnet.solana.com",
            "11111111111111111111111111111111",
        );
        assert_eq!(config.chain, ChainType::Solana);
        assert_eq!(config.network, "devnet");
        assert_eq!(config.prover_address, None);
    }

    #[test]
    fn test_marketplace_config_aptos() {
        let config = MarketplaceConfig::aptos(
            "https://fullnode.testnet.aptoslabs.com",
            "0x123abc",
            "0x456def",
        );
        assert_eq!(config.chain, ChainType::Aptos);
        assert_eq!(config.network, "testnet");
        assert_eq!(config.prover_address, Some("0x456def".to_string()));
    }

    #[test]
    fn test_marketplace_config_starknet() {
        let config = MarketplaceConfig::starknet(
            "https://starknet-mainnet.public.blastapi.io",
            "0x789ghi",
            "0xabcdef",
        );
        assert_eq!(config.chain, ChainType::Starknet);
        assert_eq!(config.network, "mainnet");
        assert_eq!(config.prover_address, Some("0xabcdef".to_string()));
    }
}
