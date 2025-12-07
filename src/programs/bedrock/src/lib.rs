//! Bedrock - ZyberLink Core Registry
//!
//! This program manages:
//! - Global configuration (admin, fees, generator program IDs)
//! - Prover registration and staking
//! - Prover reputation tracking
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      BEDROCK (this)                          │
//! │            (Registry, Provers, Config)                       │
//! └─────────────────────────────────────────────────────────────┘
//!                               ↑ CPI
//! ┌─────────────────────────────────────────────────────────────┐
//! │         FHE-GENERATOR          │        ZK-GENERATOR         │
//! │    verify_prover()             │    verify_prover()          │
//! │    update_prover_stats()       │    update_prover_stats()    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Instructions
//!
//! - `Initialize` - Set up global config (admin only)
//! - `RegisterProver` - Register a new prover with stake
//! - `SlashProver` - Slash a misbehaving prover
//! - `UpdateProverStats` - Update prover reputation (CPI from generators)

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub use error::*;
pub use instruction::*;
pub use processor::*;
pub use state::*;
