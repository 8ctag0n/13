//! Common types used across all chain clients

use serde::{Deserialize, Serialize};

/// Transaction status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending (not yet processed)
    Pending,
    /// Transaction is confirmed on-chain
    Confirmed,
    /// Transaction failed
    Failed,
    /// Transaction status unknown
    Unknown,
}

impl TransactionStatus {
    /// Check if transaction is finalized (confirmed or failed)
    pub fn is_finalized(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Failed)
    }

    /// Check if transaction is successful
    pub fn is_successful(&self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

/// Chain identifier enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainId {
    Solana,
    Starknet,
    Aptos,
}

impl ChainId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainId::Solana => "solana",
            ChainId::Starknet => "starknet",
            ChainId::Aptos => "aptos",
        }
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Network type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Localnet,
}

impl Network {
    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Devnet => "devnet",
            Network::Localnet => "localnet",
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Transaction receipt containing execution details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub signature: String,
    pub status: TransactionStatus,
    pub block_height: u64,
    pub fee: u64,
    pub logs: Vec<String>,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: u64,
    pub data: Vec<u8>,
    pub owner: Option<String>,
}
