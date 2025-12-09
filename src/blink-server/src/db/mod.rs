pub mod attestation_queries;
pub mod connection;
pub mod models;
pub mod queries;
pub mod zk_queries;

// Re-export commonly used types
pub use attestation_queries::AttestationQueries;
pub use connection::{create_pool, run_migrations};
pub use models::{InsertJobData, JobStatus};
pub use queries::{FheResultQueries, JobQueries, NetworkMetricsQueries, NonceQueries, ServerKeyQueries, WitnessQueries};
pub use zk_queries::{InsertZkJobData, ZkJobQueries, ZkJobStatus};
