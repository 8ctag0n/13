//! Circuit verifiers for ZK proof types
//!
//! This module contains on-chain verification logic for different circuit types.
//! Each circuit type has its own verifier that validates proofs and public inputs.
//!
//! # Circuit Types
//!
//! - Legacy (v1.x): ZcashOrchard, ZcashSapling, AnonymousVote, Credential
//! - Core Primitives (v2.0): ProofOfInnocence
//! - Verticals (v2.0): PrivateVote, MarketBet, PortfolioCompliance, etc.

pub mod poi;
pub mod types;

pub use poi::*;
pub use types::*;
