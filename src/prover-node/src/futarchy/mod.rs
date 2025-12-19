//! Futarchy Markets FHE Processing Module
//!
//! This module handles FHE operations for Futarchy Markets:
//! - Processing encrypted bet amounts
//! - Updating encrypted pools (pool + bet = new_pool)
//! - Fetching ciphertexts from app server
//! - Submitting results back to chain
//! - Polling for pending jobs from blink-server

pub mod pool_worker;
pub mod ciphertext_fetcher;
pub mod poller;

pub use pool_worker::{FutarchyPoolWorker, FutarchyPoolJob, FutarchyPoolResult};
pub use ciphertext_fetcher::CiphertextFetcher;
pub use poller::{FutarchyPoller, FutarchyPollerConfig};
