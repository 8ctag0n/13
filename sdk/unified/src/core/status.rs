//! Job status types
//!
//! Re-exports JobStatus from zyberlink-types for convenience

pub use zyberlink_types::JobStatus;

/// Extension trait for JobStatus with additional helper methods
pub trait JobStatusExt {
    /// Check if status is a terminal state
    fn is_terminal(&self) -> bool;

    /// Check if job can be claimed in this status
    fn can_claim_status(&self) -> bool;

    /// Check if job is actively being worked on
    fn is_active(&self) -> bool;

    /// Get human-readable description
    fn description(&self) -> &'static str;
}

impl JobStatusExt for JobStatus {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    fn can_claim_status(&self) -> bool {
        matches!(self, JobStatus::Pending)
    }

    fn is_active(&self) -> bool {
        matches!(self, JobStatus::Claimed)
    }

    fn description(&self) -> &'static str {
        match self {
            JobStatus::Pending => "Waiting to be claimed by a prover",
            JobStatus::Claimed => "Being processed by a prover",
            JobStatus::Completed => "Successfully completed with proof",
            JobStatus::Failed => "Processing failed or timed out",
            JobStatus::Cancelled => "Cancelled by creator",
        }
    }
}
