//! Bedrock SDK
//!
//! Client SDK for interacting with the ZyberLink Bedrock program.
//!
//! This SDK provides:
//! - PDA derivation functions
//! - Instruction builders
//! - Account helpers
//! - Type re-exports from bedrock program

pub mod accounts;
pub mod instructions;
pub mod pda;

// Re-export core types from bedrock
pub use bedrock::cpi::*;
pub use bedrock::*;

// Re-export PDA derivation functions
pub use pda::{derive_config_pda, derive_prover_pda, derive_validator_pda};

// Re-export instruction builders
pub use instructions::*;
