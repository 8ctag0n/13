//! Solana chain client implementation
//!
//! Wraps the existing Solana functionality into the ChainClient trait.

use super::ChainClient;
use crate::solana::{load_keypair, PaymentTxBuilder};
use anyhow::{Context, Result};
use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
    signer::Signer,
};
use std::path::PathBuf;

/// Solana blockchain client
pub struct SolanaChainClient {
    rpc_url: String,
    keypair_path: PathBuf,
    keypair: Option<Keypair>,
}

impl SolanaChainClient {
    /// Create a new Solana chain client
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint
    /// * `keypair_path` - Path to keypair file (will be loaded lazily)
    pub fn new(rpc_url: String, keypair_path: PathBuf) -> Self {
        Self {
            rpc_url,
            keypair_path,
            keypair: None,
        }
    }

    /// Load keypair (lazy initialization)
    fn ensure_keypair(&mut self) -> Result<&Keypair> {
        if self.keypair.is_none() {
            let path_str = self.keypair_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid keypair path"))?;

            let keypair = load_keypair(path_str)
                .context("Failed to load Solana keypair")?;

            self.keypair = Some(keypair);
        }

        Ok(self.keypair.as_ref().unwrap())
    }

    /// Get RPC client
    fn rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        )
    }

    /// Get public key as string
    pub fn pubkey(&mut self) -> Result<String> {
        let keypair = self.ensure_keypair()?;
        Ok(keypair.pubkey().to_string())
    }
}

#[async_trait]
impl ChainClient for SolanaChainClient {
    fn chain_id(&self) -> &str {
        "solana"
    }

    async fn get_balance(&self, address: &str) -> Result<u64> {
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;

        let pubkey = Pubkey::from_str(address)
            .context("Invalid Solana address")?;

        let client = self.rpc_client();
        client.get_balance(&pubkey)
            .context("Failed to get balance")
    }

    async fn send_payment(&self, to: &str, amount: u64) -> Result<String> {
        // Need to clone self data to make it mutable
        let keypair_path = self.keypair_path.clone();
        let rpc_url = self.rpc_url.clone();

        // Load keypair
        let path_str = keypair_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid keypair path"))?;
        let keypair = load_keypair(path_str)?;

        // Build and send transaction
        let tx_builder = PaymentTxBuilder::new(&rpc_url)?;
        let transaction = tx_builder.build_payment_transaction(
            &keypair,
            to,
            amount,
        ).await?;

        // Send and confirm
        let signature = crate::solana::sign_and_send_transaction(
            &rpc_url,
            transaction,
            &keypair,
        ).await?;

        Ok(signature.to_string())
    }

    fn native_symbol(&self) -> &str {
        "SOL"
    }

    fn format_amount(&self, amount: u64) -> String {
        let sol = lamports_to_sol(amount);
        format!("{:.9} SOL", sol)
    }

    async fn health_check(&self) -> Result<bool> {
        let client = self.rpc_client();
        match client.get_health() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}
