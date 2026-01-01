//! # ZyberLink Unified SDK
//!
//! A unified SDK for all ZyberLink programs:
//! - **Bedrock**: Registry for provers and validators
//! - **ZK-Generator**: Zero-knowledge proof job management
//! - **FHE-Generator**: Fully homomorphic encryption job management with consensus
//! - **Futarchy**: Prediction markets with encrypted pools
//!
//! ## Core Abstraction: UnifiedJob
//!
//! The key innovation is `UnifiedJob`, an enum that can represent jobs from any generator:
//!
//! ```rust,ignore
//! use zyberlink_unified_sdk::core::UnifiedJob;
//!
//! // Works with jobs from any generator
//! match job {
//!     UnifiedJob::Zk(data) => handle_zk_job(data),
//!     UnifiedJob::Fhe(data) => handle_fhe_job(data),
//!     UnifiedJob::Legacy(data) => handle_legacy_job(data),
//! }
//!
//! // Or use common API
//! println!("Job ID: {}", job.id());
//! println!("Price: {} lamports", job.price());
//! if job.can_claim() {
//!     // Claim the job
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! zyberlink-unified-sdk
//!   ├── core/           - UnifiedJob, JobStatus, CircuitType
//!   ├── programs/       - Per-program SDKs (bedrock, zk, fhe, futarchy)
//!   ├── unified/        - High-level unified client
//!   └── prover/         - Prover-specific utilities
//! ```
//!
//! ## Usage
//!
//! ### Basic Example
//!
//! ```rust,ignore
//! use zyberlink_unified_sdk::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Connect to devnet
//!     let client = UnifiedClient::connect("devnet").await?;
//!     
//!     // Get all pending jobs (from all generators)
//!     let jobs = client.jobs().get_pending().await?;
//!     
//!     for job in jobs {
//!         println!("Job {}: {} ({})", 
//!             job.id(), 
//!             job.circuit_name(),
//!             job.price()
//!         );
//!     }
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod core;
pub mod query;

// TODO: Implement these modules in future phases
// pub mod programs;  // PHASE 2: Program-specific SDKs
// pub mod client;    // PHASE 3: Unified client
// pub mod utils;     // PHASE 5: Utilities

// #[cfg(feature = "prover")]
// pub mod prover;    // PHASE 4: Prover module

// Prelude for convenient imports
pub mod prelude {
    //! Convenient re-exports for common usage

    pub use crate::core::{
        CircuitType, FheConsensusData, FheJobData, JobCommon, JobStatus,
        LegacyJobAccount, LegacyJobData, Result, UnifiedError, UnifiedJob,
        ZkJobData, MAX_FHE_PROVERS,
    };

    // Re-export query types
    pub use crate::query::{JobQuery, UnifiedJob as QueryUnifiedJob};

    // Re-export extension traits
    pub use crate::core::status::JobStatusExt;

    // Re-export circuit helpers
    pub use crate::core::circuits::{
        circuit_name, is_fhe_circuit, is_zk_circuit, is_legacy_circuit,
        is_v2_circuit, requires_poi,
    };
}

// Version information
/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// SDK name
pub const NAME: &str = env!("CARGO_PKG_NAME");
