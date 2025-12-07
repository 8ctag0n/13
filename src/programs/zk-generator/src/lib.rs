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
//! - `CIRCUIT_ZCASH_ORCHARD` (0) - Zcash Orchard proof
//! - `CIRCUIT_ZCASH_SAPLING` (1) - Zcash Sapling proof
//! - `CIRCUIT_ANONYMOUS_VOTE` (2) - Anonymous voting proof
//! - `CIRCUIT_CREDENTIAL` (3) - Credential verification proof

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
