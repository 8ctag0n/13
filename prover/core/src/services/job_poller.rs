use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use solana_sdk::pubkey::Pubkey;
use zyberlink_sdk::MarketplaceClient;
use crate::roi_calculator::{ROICalculator, OperationMode};

/// Represents a discovered job ready to be processed
#[derive(Debug, Clone)]
pub struct DiscoveredJob {
    /// PDA of the job account
    pub job_pda: Pubkey,
    /// Unique job ID
    pub job_id: u64,
    /// Circuit type identifier
    pub circuit_type: u8,
    /// Hash/commitment of the witness data
    pub witness_hash: [u8; 32],
    /// Price offered for the job in lamports
    pub price_lamports: u64,
}

/// Service for polling and discovering jobs from the blockchain
pub struct JobPoller {
    client: Arc<MarketplaceClient>,
    roi_calculator: ROICalculator,
    poll_interval: Duration,
    min_roi: f64,
}

impl JobPoller {
    /// Create a new JobPoller
    ///
    /// # Arguments
    /// * `client` - Marketplace client for querying jobs
    /// * `poll_interval` - How often to poll for new jobs
    /// * `min_roi` - Minimum ROI percentage required to accept a job
    /// * `cost_multiplier` - Operational cost multiplier for ROI calculation
    pub fn new(
        client: Arc<MarketplaceClient>,
        poll_interval: Duration,
        min_roi: f64,
        cost_multiplier: f64,
    ) -> Self {
        // JobPoller uses default Profit mode - for full mode support use ROICalculator directly
        let roi_calculator = ROICalculator::new(min_roi, cost_multiplier, OperationMode::default());

        Self {
            client,
            roi_calculator,
            poll_interval,
            min_roi,
        }
    }

    /// Poll for pending jobs that meet ROI criteria
    ///
    /// This method queries the blockchain for available jobs and filters them
    /// based on profitability using the ROI calculator.
    ///
    /// # Returns
    /// Vector of discovered jobs that are profitable
    pub async fn poll_jobs(&self) -> Result<Vec<DiscoveredJob>> {
        // Query pending jobs from blockchain
        let pending_jobs = zyberlink_sdk::find_pending_jobs(
            &self.client.rpc_client,
            &self.client.program_id,
        )?;

        let mut discovered_jobs = Vec::new();

        for (job_pda, job) in pending_jobs {
            let discovered = DiscoveredJob {
                job_pda,
                job_id: job.id,
                circuit_type: job.circuit_type,
                witness_hash: job.witness_hash,
                price_lamports: job.price_lamports,
            };

            // Filter by ROI requirements
            if self.meets_roi_requirements(&discovered) {
                discovered_jobs.push(discovered);
            }
        }

        Ok(discovered_jobs)
    }

    /// Check if a job meets minimum ROI requirements
    ///
    /// This evaluates the job's profitability based on the circuit type,
    /// price offered, and operational costs.
    pub fn meets_roi_requirements(&self, job: &DiscoveredJob) -> bool {
        // Get circuit type for ROI calculation
        let circuit_type = crate::core::CircuitRegistry::get_circuit_type(
            job.circuit_type,
            None, // TODO: Add FHE data support when needed
        );

        // Evaluate ROI (assuming single prover for now)
        let roi = self.roi_calculator.evaluate_job(&circuit_type, job.price_lamports, 1);

        roi.is_profitable
    }

    /// Get poll interval
    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// Get minimum ROI percentage
    pub fn min_roi(&self) -> f64 {
        self.min_roi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::commitment_config::CommitmentConfig;

    #[test]
    fn test_job_poller_creation() {
        let client = Arc::new(MarketplaceClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            Pubkey::new_unique(),
            CommitmentConfig::confirmed(),
        ));

        let poller = JobPoller::new(
            client,
            Duration::from_secs(10),
            20.0,  // 20% min ROI
            1.5,   // 1.5x cost multiplier
        );

        assert_eq!(poller.poll_interval(), Duration::from_secs(10));
        assert_eq!(poller.min_roi(), 20.0);
    }

    #[test]
    fn test_discovered_job_fields() {
        let job = DiscoveredJob {
            job_pda: Pubkey::new_unique(),
            job_id: 42,
            circuit_type: 1,
            witness_hash: [1u8; 32],
            price_lamports: 1_000_000,
        };

        assert_eq!(job.job_id, 42);
        assert_eq!(job.circuit_type, 1);
        assert_eq!(job.price_lamports, 1_000_000);
    }
}
