//! FHE Generator - Fully Homomorphic Encryption Processing
//!
//! This program manages FHE computation jobs with multi-prover consensus:
//! - Job creation with encrypted witness
//! - Multi-prover claiming (2-5 provers per job)
//! - Result submission and consensus checking
//! - Finalization with payment distribution
//!
//! # Architecture
//!
//! ```text
//! zyberlink-jobs
//!       │
//!       ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  FHE-GENERATOR (this)                        │
//! │    FheJob { common: JobCommon, circuit_type, consensus_bump}│
//! │    FheConsensusData { provers, results, consensus_hash }    │
//! │                                                              │
//! │    Instructions:                                             │
//! │    - CreateJob    → creates FheJob + FheConsensusData       │
//! │    - ClaimJob     → prover joins consensus (up to N)        │
//! │    - SubmitResult → prover submits encrypted result         │
//! │    - FinalizeJob  → check consensus, distribute payments    │
//! │    - CancelJob    → creator cancels pending job             │
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
//! # Supported Operations
//!
//! - `CIRCUIT_FHE_ADD` (4) - Encrypted addition
//! - `CIRCUIT_FHE_MULTIPLY` (5) - Encrypted multiplication
//! - `CIRCUIT_FHE_SUM` (6) - Sum of encrypted values
//! - `CIRCUIT_FHE_THRESHOLD` (7) - Threshold check
//! - `CIRCUIT_FHE_RANGE_CHECK` (8) - Range verification
//! - `CIRCUIT_FHE_AVERAGE` (9) - Average computation
//! - `CIRCUIT_FHE_COUNT_IF` (10) - Conditional counting
//! - `CIRCUIT_FHE_HISTOGRAM` (11) - Distribution histogram
//!
//! # Consensus
//!
//! FHE jobs require multiple provers to agree on the result:
//! - `required_provers`: 2-5 provers must claim
//! - `consensus_threshold`: minimum matching results for consensus
//! - Provers with matching results split the payment
//! - Provers with mismatching results get slashed

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
