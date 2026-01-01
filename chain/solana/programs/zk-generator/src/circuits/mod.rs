//! Circuit verifiers for ZK proof types
//!
//! This module contains on-chain verification logic for different circuit types.
//! Each circuit type has its own verifier that validates proofs and public inputs.
//!
//! # Circuit Types
//!
//! ## Legacy (v1.x)
//! - ZcashOrchard, ZcashSapling, AnonymousVote, Credential
//!
//! ## Core Primitives (v2.0)
//! - ProofOfInnocence - Merkle non-membership (blacklist/sanctions)
//!
//! ## Verticals (v2.0)
//! - PrivateVote - Anonymous DAO voting with nullifiers
//! - MarketBet - Private prediction market bets
//! - MarketClaim - Claim market winnings with proof
//! - PortfolioCompliance - Regulatory compliance proofs
//! - PortfolioNetWorth - Net worth threshold proofs

pub mod market;
pub mod poi;
pub mod portfolio;
pub mod types;
pub mod vote;

pub use market::*;
pub use poi::*;
pub use portfolio::*;
pub use types::*;
pub use vote::*;
