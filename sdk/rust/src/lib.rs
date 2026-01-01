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
pub mod prepared;
pub mod fluent;
pub mod batch;
pub mod circuits;
pub mod crypto;
pub mod groth16;

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
pub use prepared::{PreparedOperation, CostEstimate, SimulationResult, OperationPdas};
pub use fluent::{FluentOperation, RetryPolicy, ErrorAction, OperationError, IntoFluent};
pub use batch::{BatchBuilder, BatchOp, BatchResult, BatchSummary};
pub use circuits::{CircuitId, CircuitMetadata, list_circuits, get_circuit, load_vkey_json, vkey_path, VerificationKeyJson};
pub use crypto::{MerkleTree, MerkleProof, poseidon_hash, generate_commitment, generate_nullifier};
pub use groth16::{vk_from_json, proof_from_json, prepare_vk, verify};

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
