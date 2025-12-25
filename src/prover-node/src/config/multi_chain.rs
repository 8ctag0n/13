//! Multi-chain configuration support via TOML files
//!
//! This module provides TOML-based configuration for running the prover-node
//! across multiple blockchain networks simultaneously.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::path::Path;

use crate::roi_calculator::OperationMode;

/// Top-level prover configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfigFile {
    /// General prover settings
    pub prover: ProverSettings,

    /// Witness service configuration
    #[serde(default)]
    pub witness: WitnessSettings,

    /// Chain-specific configurations
    pub chains: HashMap<String, ChainConfig>,
}

impl ProverConfigFile {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .context("Failed to read config file")?;

        let mut config: Self = toml::from_str(&contents)
            .context("Failed to parse TOML config")?;

        // Expand ~ in paths
        config.expand_paths()?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Expand ~ to home directory in all path fields
    fn expand_paths(&mut self) -> Result<()> {
        let home = std::env::var("HOME").unwrap_or_default();

        for chain_config in self.chains.values_mut() {
            if let ChainConfig::Solana(ref mut sol) = chain_config {
                if let Some(ref mut path) = sol.keypair_path {
                    *path = path.replace("~", &home);
                }
            }
        }

        Ok(())
    }

    /// Validate configuration for common errors
    fn validate(&self) -> Result<()> {
        // Check at least one chain is enabled
        let enabled_chains: Vec<_> = self.chains.iter()
            .filter(|(_, config)| config.is_enabled())
            .collect();

        if enabled_chains.is_empty() {
            anyhow::bail!("No chains enabled in configuration");
        }

        // Validate each chain config
        for (chain_name, chain_config) in &self.chains {
            chain_config.validate()
                .with_context(|| format!("Invalid config for chain '{}'", chain_name))?;
        }

        Ok(())
    }

    /// Get list of enabled chains
    pub fn enabled_chains(&self) -> Vec<(String, &ChainConfig)> {
        self.chains.iter()
            .filter(|(_, config)| config.is_enabled())
            .map(|(name, config)| (name.clone(), config))
            .collect()
    }
}

/// General prover settings (chain-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverSettings {
    /// Maximum concurrent jobs across all chains
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Poll interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    /// Operation mode: "profit", "contributor", or "subsidize"
    /// - profit: Only accept jobs with ROI >= min_roi_threshold
    /// - contributor: Accept jobs that at least break even (ROI >= 0%)
    /// - subsidize: Accept all jobs regardless of profitability
    #[serde(default)]
    pub mode: OperationMode,

    /// Minimum ROI threshold (percentage) - only used in "profit" mode
    #[serde(default = "default_min_roi")]
    pub min_roi_threshold: f64,

    /// Cost multiplier for overhead calculation
    #[serde(default = "default_cost_multiplier")]
    pub cost_multiplier: f64,

    /// Mock proving time for testing (seconds)
    #[serde(default = "default_mock_proving_time")]
    pub mock_proving_time_secs: u64,

    /// ZK circuits directory path
    #[serde(default = "default_zk_circuits_path")]
    pub zk_circuits_path: String,

    /// FHE server key path (optional)
    pub fhe_server_key_path: Option<String>,
}

/// Witness service settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WitnessSettings {
    /// Base URL for witness storage and proof submission
    #[serde(default = "default_witness_base_url")]
    pub base_url: String,

    /// Blink backend URL for ZK job coordination
    #[serde(default = "default_blink_backend_url")]
    pub blink_backend_url: String,
}

/// Chain-specific configuration (enum for different chains)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChainConfig {
    Solana(SolanaChainConfig),
    Starknet(StarknetChainConfig),
    Aptos(AptosChainConfig),
}

impl ChainConfig {
    /// Check if this chain is enabled
    pub fn is_enabled(&self) -> bool {
        match self {
            ChainConfig::Solana(c) => c.enabled,
            ChainConfig::Starknet(c) => c.enabled,
            ChainConfig::Aptos(c) => c.enabled,
        }
    }

    /// Validate chain-specific configuration
    fn validate(&self) -> Result<()> {
        match self {
            ChainConfig::Solana(c) => c.validate(),
            ChainConfig::Starknet(c) => c.validate(),
            ChainConfig::Aptos(c) => c.validate(),
        }
    }

    /// Get RPC URL for this chain
    pub fn rpc_url(&self) -> &str {
        match self {
            ChainConfig::Solana(c) => &c.rpc_url,
            ChainConfig::Starknet(c) => &c.rpc_url,
            ChainConfig::Aptos(c) => &c.rpc_url,
        }
    }
}

/// Solana chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaChainConfig {
    /// Enable this chain
    #[serde(default)]
    pub enabled: bool,

    /// Solana RPC URL
    pub rpc_url: String,

    /// Solana WebSocket URL (optional)
    pub ws_url: Option<String>,

    /// Path to Solana keypair file
    pub keypair_path: Option<String>,

    /// Legacy ZyberLink program ID
    pub program_id: Option<String>,

    /// ZK Generator program ID (new architecture)
    pub zk_generator_program: Option<String>,

    /// FHE Generator program ID (new architecture)
    pub fhe_generator_program: Option<String>,
}

impl SolanaChainConfig {
    fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Validate RPC URL
        if self.rpc_url.is_empty() {
            anyhow::bail!("rpc_url is required for Solana");
        }

        // Validate program IDs (at least one should be set)
        if self.program_id.is_none()
            && self.zk_generator_program.is_none()
            && self.fhe_generator_program.is_none()
        {
            anyhow::bail!("At least one program ID must be configured (program_id, zk_generator_program, or fhe_generator_program)");
        }

        // Validate program ID formats if provided
        if let Some(ref pid) = self.program_id {
            pid.parse::<Pubkey>()
                .context("Invalid program_id format")?;
        }

        if let Some(ref pid) = self.zk_generator_program {
            pid.parse::<Pubkey>()
                .context("Invalid zk_generator_program format")?;
        }

        if let Some(ref pid) = self.fhe_generator_program {
            pid.parse::<Pubkey>()
                .context("Invalid fhe_generator_program format")?;
        }

        // Validate keypair path exists if provided
        if let Some(ref path) = self.keypair_path {
            if !Path::new(path).exists() {
                anyhow::bail!("Keypair file not found: {}", path);
            }
        }

        Ok(())
    }
}

/// Starknet chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarknetChainConfig {
    /// Enable this chain
    #[serde(default)]
    pub enabled: bool,

    /// Starknet RPC URL
    pub rpc_url: String,

    /// PbtcfiJobs contract address
    pub contract_address: String,

    /// Prover account address
    pub prover_address: String,

    /// Private key (loaded from environment variable STARKNET_PRIVATE_KEY)
    /// NOTE: Never store private keys in config files!
    #[serde(skip)]
    pub private_key: Option<String>,
}

impl StarknetChainConfig {
    fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.rpc_url.is_empty() {
            anyhow::bail!("rpc_url is required for Starknet");
        }

        if self.contract_address.is_empty() {
            anyhow::bail!("contract_address is required for Starknet");
        }

        if self.prover_address.is_empty() {
            anyhow::bail!("prover_address is required for Starknet");
        }

        // Validate address formats (basic check for 0x prefix)
        if !self.contract_address.starts_with("0x") {
            anyhow::bail!("contract_address must start with '0x'");
        }

        if !self.prover_address.starts_with("0x") {
            anyhow::bail!("prover_address must start with '0x'");
        }

        Ok(())
    }

    /// Load private key from environment variable
    pub fn load_private_key(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.private_key = std::env::var("STARKNET_PRIVATE_KEY").ok();

        if self.private_key.is_none() {
            log::warn!("STARKNET_PRIVATE_KEY not set - running in read-only mode");
        }

        Ok(())
    }
}

/// Aptos chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptosChainConfig {
    /// Enable this chain
    #[serde(default)]
    pub enabled: bool,

    /// Aptos RPC URL
    pub rpc_url: String,

    /// Module address
    pub module_address: String,

    /// Prover account address
    pub prover_address: String,

    /// Private key (loaded from environment variable APTOS_PRIVATE_KEY)
    /// NOTE: Never store private keys in config files!
    #[serde(skip)]
    pub private_key: Option<String>,
}

impl AptosChainConfig {
    fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.rpc_url.is_empty() {
            anyhow::bail!("rpc_url is required for Aptos");
        }

        if self.module_address.is_empty() {
            anyhow::bail!("module_address is required for Aptos");
        }

        if self.prover_address.is_empty() {
            anyhow::bail!("prover_address is required for Aptos");
        }

        // Validate address formats (basic check for 0x prefix)
        if !self.module_address.starts_with("0x") {
            anyhow::bail!("module_address must start with '0x'");
        }

        if !self.prover_address.starts_with("0x") {
            anyhow::bail!("prover_address must start with '0x'");
        }

        Ok(())
    }

    /// Load private key from environment variable
    pub fn load_private_key(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.private_key = std::env::var("APTOS_PRIVATE_KEY").ok();

        if self.private_key.is_none() {
            log::warn!("APTOS_PRIVATE_KEY not set - running in read-only mode");
        }

        Ok(())
    }
}

// ========== Default Values ==========

fn default_max_concurrent_jobs() -> usize {
    3
}

fn default_poll_interval() -> u64 {
    5
}

fn default_min_roi() -> f64 {
    1.5
}

fn default_cost_multiplier() -> f64 {
    1.5
}

fn default_mock_proving_time() -> u64 {
    10
}

fn default_zk_circuits_path() -> String {
    "./prover-circuits".to_string()
}

fn default_witness_base_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_blink_backend_url() -> String {
    "http://localhost:3000".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[prover]
max_concurrent_jobs = 3
poll_interval_secs = 5

[witness]
base_url = "https://witness.example.com"

[chains.solana]
enabled = true
rpc_url = "https://api.devnet.solana.com"
program_id = "BedRock11111111111111111111111111111111111"
        "#;

        let config: ProverConfigFile = toml::from_str(toml).unwrap();

        assert_eq!(config.prover.max_concurrent_jobs, 3);
        assert_eq!(config.prover.poll_interval_secs, 5);
        assert_eq!(config.witness.base_url, "https://witness.example.com");
        assert_eq!(config.chains.len(), 1);
    }

    #[test]
    fn test_parse_multi_chain_config() {
        let toml = r#"
[prover]
max_concurrent_jobs = 5

[witness]
base_url = "https://witness.example.com"

[chains.solana]
enabled = true
rpc_url = "https://api.devnet.solana.com"
program_id = "BedRock11111111111111111111111111111111111"

[chains.starknet]
enabled = true
rpc_url = "http://localhost:5050"
contract_address = "0x0587f1e2a4494f7d72ea0811a4c20185f2e2b12e05b6d8bc8948847e2cf709bd"
prover_address = "0x0557ba9ef60b52dad611d79b60563901458f2476a5c1002a8b4869fcb6654c7e"

[chains.aptos]
enabled = false
rpc_url = "https://fullnode.devnet.aptoslabs.com"
module_address = "0x123"
prover_address = "0x456"
        "#;

        let config: ProverConfigFile = toml::from_str(toml).unwrap();

        assert_eq!(config.chains.len(), 3);
        assert!(config.chains.contains_key("solana"));
        assert!(config.chains.contains_key("starknet"));
        assert!(config.chains.contains_key("aptos"));

        let enabled = config.enabled_chains();
        assert_eq!(enabled.len(), 2); // Only Solana and Starknet enabled
    }

    #[test]
    fn test_validation_fails_no_enabled_chains() {
        let toml = r#"
[prover]
max_concurrent_jobs = 3

[witness]
base_url = "http://localhost:8080"

[chains.solana]
enabled = false
rpc_url = "https://api.devnet.solana.com"
program_id = "BedRock11111111111111111111111111111111111"
        "#;

        let config: ProverConfigFile = toml::from_str(toml).unwrap();
        assert!(config.validate().is_err());
    }
}
