//! ZK Generator - Zero-Knowledge Proof Processing
//!
//! This program manages ZK proof jobs:
//! - Job creation with encrypted witness
//! - Single-prover job claiming
//! - Proof submission and verification
//! - Payment release via escrow
//!
//! # Architecture
//!
//! ```text
//! zyberlink-jobs
//!       │
//!       ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  ZK-GENERATOR (this)                         │
//! │    ZkJob { common: JobCommon, circuit_type: u8 }            │
//! │                                                              │
//! │    Instructions:                                             │
//! │    - CreateJob  → creates ZkJob + escrow                    │
//! │    - ClaimJob   → single prover claims                      │
//! │    - SubmitProof → prover submits proof                     │
//! │    - CancelJob  → creator cancels pending job               │
//! └─────────────────────────────────────────────────────────────┘
//!                               │ CPI
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      BEDROCK                                 │
//! │    verify_prover() - check prover is registered             │
//! │    update_prover_stats() - update reputation                │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported Circuits
//!
//! ## Legacy (v1.x)
//! - `ZcashOrchard` (0) - Zcash Orchard proof
//! - `ZcashSapling` (1) - Zcash Sapling proof
//! - `AnonymousVote` (2) - Anonymous voting proof
//! - `Credential` (3) - Credential verification proof
//!
//! ## Core Primitives (v2.0)
//! - `ProofOfInnocence` (10) - Merkle non-membership proof (blacklist)
//!
//! ## Verticals (v2.0)
//! - `PrivateVote` (20) - Anonymous DAO voting
//! - `PrivateVoteWithPoI` (21) - Voting with conflict of interest check
//! - `MarketBet` (30) - Private prediction market bet
//! - `MarketBetWithPoI` (31) - Bet with insider trading check
//! - `MarketClaim` (32) - Claim market winnings
//! - `PortfolioCompliance` (40) - Regulatory compliance proof
//! - `PortfolioNetWorth` (41) - Net worth threshold proof

pub mod circuits;
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
