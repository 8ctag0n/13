//! Network configuration for ZyberLink
//!
//! Provides pre-configured network settings for devnet, mainnet, and localnet.

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Solana RPC URL
    pub rpc_url: String,
    /// ZyberLink program ID
    pub program_id: Pubkey,
    /// Witness backend URL
    pub backend_url: String,
    /// Network name (for display)
    pub name: String,
}

impl NetworkConfig {
    /// Get configuration for a named network
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "devnet" => Some(Self::devnet()),
            "mainnet" | "mainnet-beta" => Some(Self::mainnet()),
            "localnet" | "localhost" | "local" => Some(Self::localnet()),
            _ => None,
        }
    }

    /// Devnet configuration
    pub fn devnet() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: Self::default_program_id(),
            backend_url: "https://api.zyberlink.fun".to_string(),
            name: "devnet".to_string(),
        }
    }

    /// Mainnet configuration
    pub fn mainnet() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            program_id: Self::default_program_id(),
            backend_url: "https://api.zyberlink.fun".to_string(),
            name: "mainnet".to_string(),
        }
    }

    /// Localnet configuration
    pub fn localnet() -> Self {
        Self {
            rpc_url: "http://localhost:8899".to_string(),
            program_id: Self::read_local_program_id().unwrap_or_else(Self::default_program_id),
            backend_url: "http://localhost:8080".to_string(),
            name: "localnet".to_string(),
        }
    }

    /// Custom configuration
    pub fn custom(rpc_url: &str, program_id: Pubkey, backend_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            program_id,
            backend_url: backend_url.to_string(),
            name: "custom".to_string(),
        }
    }

    /// Try to read program ID from local file (for localnet)
    fn read_local_program_id() -> Option<Pubkey> {
        // Try common locations
        let paths = [
            "/tmp/zyberlink_program_id.txt",
            "./logs/zyberlink_program_id.txt",
            "../logs/zyberlink_program_id.txt",
        ];

        for path in paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(pubkey) = Pubkey::from_str(content.trim()) {
                    return Some(pubkey);
                }
            }
        }
        None
    }

    /// Default program ID (placeholder - should be updated for production)
    fn default_program_id() -> Pubkey {
        // This is a placeholder - real deployments should set the actual program ID
        Pubkey::from_str("ZYBRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
            .unwrap_or_else(|_| Pubkey::new_unique())
    }
}

/// Keys storage location
pub fn keys_dir() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".zyberlink").join("keys")
}

/// Default keypair path
pub fn default_keypair_path() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".config").join("solana").join("id.json")
}
