// SDK for interacting with ZyberLink marketplace
//
//! # ZyberLink SDK
//!
//! Simple, high-level SDK for privacy-preserving computation on Solana.
//!
//! ## Quick Start
//!
//! ```ignore
//! use zyberlink_sdk::Zyber;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let zyber = Zyber::connect("devnet").await?;
//!     let sum = zyber.sum(&[100, 200, 300]).await?;
//!     println!("Sum: {}", sum);
//!     Ok(())
//! }
//! ```

// High-level client (recommended)
pub mod zyber;
pub mod config;

// Low-level modules
pub mod client;
pub mod client_ext;
pub mod helpers;
pub mod instruction;
pub mod instructions;
pub mod marketplace_sdk;
pub mod transaction;

// Re-export high-level API
pub use zyber::{Zyber, ZyberBuilder};
pub use config::NetworkConfig;

// Re-export low-level API
pub use client::*;
pub use helpers::*;
pub use instruction::*;
pub use marketplace_sdk::MarketplaceSDK;

// Re-export FHE types for convenience
pub use zyberlink_fhe::{
    FheEngine, FheError, FheOperation, FhePredicate,
    encrypt_value, encrypt_values, decrypt_result,
    generate_keys, serialize_server_key, deserialize_server_key,
};

// Re-export common types
pub use zyberlink_types::{CircuitType, FheConsensusConfig};
