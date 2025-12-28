//! Chain abstraction layer for multi-chain CLI support
//!
//! This module provides a unified interface for interacting with different blockchains
//! (Solana, Starknet) through the CLI.

pub mod solana;
pub mod starknet;

use async_trait::async_trait;
use anyhow::Result;

/// Chain client trait for unified blockchain operations
///
/// This trait provides a simplified interface for CLI operations across different chains.
/// It abstracts away chain-specific details while maintaining enough flexibility for
/// chain-specific features.
#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Get the chain identifier (e.g., "solana", "starknet")
    fn chain_id(&self) -> &str;

    /// Get native token balance for an address
    ///
    /// # Arguments
    /// * `address` - Account address as string
    ///
    /// # Returns
    /// Balance in smallest unit (lamports for Solana, wei for Starknet)
    async fn get_balance(&self, address: &str) -> Result<u64>;

    /// Send a payment transaction
    ///
    /// # Arguments
    /// * `to` - Recipient address
    /// * `amount` - Amount in smallest unit
    ///
    /// # Returns
    /// Transaction signature/hash
    async fn send_payment(&self, to: &str, amount: u64) -> Result<String>;

    /// Get native token symbol (SOL, ETH, STRK, etc.)
    fn native_symbol(&self) -> &str;

    /// Format amount for display (converts from smallest unit to human-readable)
    ///
    /// # Arguments
    /// * `amount` - Amount in smallest unit
    ///
    /// # Returns
    /// Formatted string (e.g., "0.001 SOL")
    fn format_amount(&self, amount: u64) -> String;

    /// Check connection health
    async fn health_check(&self) -> Result<bool>;
}

/// Supported blockchain networks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Solana,
    Starknet,
}

impl Chain {
    /// Parse chain from string
    ///
    /// # Arguments
    /// * `s` - Chain name (case-insensitive)
    ///
    /// # Supported values
    /// - "solana" or "sol" -> Solana
    /// - "starknet" or "strk" -> Starknet
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "solana" | "sol" => Ok(Chain::Solana),
            "starknet" | "strk" => Ok(Chain::Starknet),
            _ => anyhow::bail!(
                "Unknown chain: {}. Supported chains: 'solana', 'starknet'",
                s
            ),
        }
    }

    /// Get chain name as string
    pub fn as_str(&self) -> &str {
        match self {
            Chain::Solana => "solana",
            Chain::Starknet => "starknet",
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
