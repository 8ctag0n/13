//! Core types and utilities for the Unified SDK
//!
//! This module contains the fundamental types used across all program SDKs:
//! - `UnifiedJob`: Enum representing jobs from any generator
//! - `JobStatus`: Job state enumeration
//! - `CircuitType`: ZK and FHE circuit types
//! - Error types and results

pub mod circuits;
pub mod config;
pub mod error;
pub mod job;
pub mod status;

// Re-exports for convenience
pub use circuits::*;
pub use config::NetworkConfig;
pub use error::{Result, UnifiedError};
pub use job::{
    FheConsensusData, FheJobData, LegacyJobAccount, LegacyJobData, UnifiedJob, ZkJobData,
    MAX_FHE_PROVERS,
};
pub use status::JobStatus;

// Re-export JobCommon from zyberlink-jobs
pub use zyberlink_jobs::JobCommon;
