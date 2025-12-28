//! Cross-program job queries and UnifiedJob abstraction
//!
//! This module provides:
//! - `UnifiedJob` - Enum representing jobs from any generator
//! - `JobQuery` - Cross-program query client
//! - `FheConsensusData` - FHE consensus state

mod types;
mod queries;

pub use types::UnifiedJob;
pub use queries::{JobQuery, FheConsensusData, JobStatus};
