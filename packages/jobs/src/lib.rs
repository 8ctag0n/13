//! ZyberLink Jobs - Shared on-chain job structures
//!
//! This library provides the common `JobCommon` structure used by all generators
//! (ZK-Generator, FHE-Generator, etc.) to ensure consistent job handling.
//!
//! # Architecture
//!
//! ```text
//! zyberlink-jobs (this crate)
//!       │
//!       ├── JobCommon (200 bytes) - shared fields
//!       ├── EscrowOperations - escrow helpers
//!       └── Validation - common validators
//!       │
//!       ▼
//! ┌─────────────┬─────────────┐
//! │ ZK-Generator│FHE-Generator│
//! │  ZkJob      │   FheJob    │
//! └─────────────┴─────────────┘
//! ```

pub mod common;
pub mod escrow;
pub mod validation;

pub use common::*;
pub use escrow::*;
pub use validation::*;
