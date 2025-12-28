//! Futarchy Markets FHE Processing Module
//!
//! This module handles FHE operations for Futarchy Markets:
//! - Processing encrypted bet amounts
//! - Updating encrypted pools (pool + bet = new_pool)
//! - Fetching ciphertexts from app server
//! - Submitting results back to chain
//! - Polling for pending jobs from blink-server
//! - Hybrid flow: API discovery + on-chain verification

pub mod pool_worker;
pub mod ciphertext_fetcher;
pub mod poller;
pub mod api_client;

pub use pool_worker::{FutarchyPoolWorker, FutarchyPoolJob, FutarchyPoolResult};
pub use ciphertext_fetcher::CiphertextFetcher;
pub use poller::{FutarchyPoller, FutarchyPollerConfig};
pub use api_client::{FutarchyApiClient, ApiFheJob, FheJobData};
