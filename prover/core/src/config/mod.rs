pub mod multi_chain;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;
use std::time::Duration;

pub use multi_chain::{
    ProverConfigFile, ProverSettings, WitnessSettings, ChainConfig,
    SolanaChainConfig, StarknetChainConfig, AptosChainConfig,
};

/// Runtime configuration for the prover node
#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub rpc_url: String,
    pub program_id: Pubkey,
    pub keypair_path: String,
    pub poll_interval: Duration,
    pub min_price: u64, // Deprecated: kept for backward compatibility
    pub min_roi: f64,
    pub cost_multiplier: f64,
    pub mock_proving_time: Duration,
    pub max_concurrent_jobs: usize,
    pub gateway_url: String,
    pub blink_backend_url: String,
    pub zk_circuits_path: String,
    pub fhe_server_key_path: Option<String>,
    /// ZK Generator program ID (for new architecture)
    pub zk_generator_program: Option<Pubkey>,
    /// FHE Generator program ID (for new architecture)
    pub fhe_generator_program: Option<Pubkey>,
}

impl ProverConfig {
    /// Create a new ProverConfig directly
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rpc_url: String,
        program_id: Pubkey,
        keypair_path: String,
        poll_interval: Duration,
        min_price: u64,
        min_roi: f64,
        cost_multiplier: f64,
        mock_proving_time: Duration,
        max_concurrent_jobs: usize,
        gateway_url: String,
        blink_backend_url: String,
        zk_circuits_path: String,
        fhe_server_key_path: Option<String>,
        zk_generator_program: Option<Pubkey>,
        fhe_generator_program: Option<Pubkey>,
    ) -> Self {
        Self {
            rpc_url,
            program_id,
            keypair_path,
            poll_interval,
            min_price,
            min_roi,
            cost_multiplier,
            mock_proving_time,
            max_concurrent_jobs,
            gateway_url,
            blink_backend_url,
            zk_circuits_path,
            fhe_server_key_path,
            zk_generator_program,
            fhe_generator_program,
        }
    }
}

/// Persisted configuration for the prover node (saved to disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfiguration {
    pub version: String,
    pub network: String,
    pub rpc_url: String,
    pub program_id: String,
    pub keypair_path: String,
    pub prover_authority: String,
    pub prover_pda: String,
    pub encryption_pubkey: String,
    pub stake_amount: u64,
    pub gateway_url: String,
    pub created_at: String,
}

impl ProverConfiguration {
    /// Create new configuration
    pub fn new(
        network: String,
        rpc_url: String,
        program_id: Pubkey,
        keypair_path: String,
        prover_authority: Pubkey,
        prover_pda: Pubkey,
        encryption_pubkey: String,
        stake_amount: u64,
        gateway_url: String,
    ) -> Self {
        use chrono::Utc;

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            network,
            rpc_url,
            program_id: program_id.to_string(),
            keypair_path,
            prover_authority: prover_authority.to_string(),
            prover_pda: prover_pda.to_string(),
            encryption_pubkey,
            stake_amount,
            gateway_url,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Get default config file path
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        Ok(home.join(".zyberlink").join("config.json"))
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<PathBuf> {
        let path = Self::default_path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        // Write config
        let json = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        tokio::fs::write(&path, json)
            .await
            .context("Failed to write config file")?;

        Ok(path)
    }

    /// Load configuration from file
    pub async fn load() -> Result<Self> {
        let path = Self::default_path()?;

        let json = tokio::fs::read_to_string(&path)
            .await
            .context("Failed to read config file")?;

        let config: Self = serde_json::from_str(&json).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Check if configuration exists
    pub fn exists() -> bool {
        Self::default_path().map(|p| p.exists()).unwrap_or(false)
    }
}
