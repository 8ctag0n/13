use anyhow::Result;
use borsh::BorshDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

// Re-export types from cypherlink program
pub use cypherlink_types::{CircuitType, JobStatus};

/// Marketplace configuration account
/// IMPORTANT: Field order must match programs/cypherlink/src/state/config.rs
#[derive(Debug, Clone, BorshDeserialize)]
pub struct MarketplaceConfig {
    pub authority: Pubkey,
    pub fee_basis_points: u16,
    pub min_stake_amount: u64,
    pub min_reputation_score: u32,
    pub default_job_timeout_seconds: i64,
    pub protocol_fee_recipient: Pubkey,
    pub next_job_id: u64,
    pub total_provers: u64,
    pub total_jobs_created: u64,
    pub total_jobs_completed: u64,
}

/// Prover account
/// IMPORTANT: Field order must match programs/cypherlink/src/state/prover.rs
#[derive(Debug, Clone, BorshDeserialize)]
pub struct ProverAccount {
    pub authority: Pubkey,
    pub stake_amount: u64,
    pub reputation_score: u32,
    pub total_jobs_completed: u64,
    pub total_jobs_failed: u64,
    pub avg_completion_time_secs: u32,
    pub is_active: bool,
    pub registration_timestamp: i64,
    pub total_earnings_lamports: u64,
    pub bump: u8,
}

/// Job account
/// IMPORTANT: Field order must match programs/cypherlink/src/state/job.rs
#[derive(Debug, Clone, BorshDeserialize)]
pub struct JobAccount {
    pub id: u64,
    pub creator: Pubkey,
    pub prover: Option<Pubkey>,
    pub status: JobStatus,
    pub circuit_type: CircuitType,
    pub witness_commitment: [u8; 32],
    pub witness_size: u32,
    pub proof_commitment: Option<[u8; 32]>,
    pub proof_size: Option<u32>,
    pub price_lamports: u64,
    pub escrow_account: Pubkey,
    pub created_at: i64,
    pub claimed_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub timeout_at: i64,
    pub bump: u8,
}

// ============================================================================
// Account Fetchers
// ============================================================================

/// Fetch and deserialize marketplace config
pub fn fetch_config(rpc_client: &RpcClient, config_pda: &Pubkey) -> Result<MarketplaceConfig> {
    let account = rpc_client.get_account(config_pda)?;
    let config = MarketplaceConfig::deserialize(&mut &account.data[..])?;
    Ok(config)
}

/// Fetch and deserialize prover account
pub fn fetch_prover(rpc_client: &RpcClient, prover_pda: &Pubkey) -> Result<ProverAccount> {
    let account = rpc_client.get_account(prover_pda)?;
    let prover = ProverAccount::deserialize(&mut &account.data[..])?;
    Ok(prover)
}

/// Fetch and deserialize job account
pub fn fetch_job(rpc_client: &RpcClient, job_pda: &Pubkey) -> Result<JobAccount> {
    let account = rpc_client.get_account(job_pda)?;
    let job = JobAccount::deserialize(&mut &account.data[..])?;
    Ok(job)
}

// ============================================================================
// Job Queries
// ============================================================================

/// Query all jobs with a specific status
pub fn find_jobs_by_status(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    status: JobStatus,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    let accounts = rpc_client.get_program_accounts(program_id)?;

    let mut jobs = Vec::new();
    for (pubkey, account) in accounts {
        // Skip if account is too small to be a job
        if account.data.len() < 100 {
            continue;
        }

        // Try to deserialize as job
        if let Ok(job) = JobAccount::deserialize(&mut &account.data[..]) {
            if job.status == status {
                jobs.push((pubkey, job));
            }
        }
    }

    Ok(jobs)
}

/// Query all jobs created by a specific creator
pub fn find_jobs_by_creator(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    creator: &Pubkey,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    let accounts = rpc_client.get_program_accounts(program_id)?;

    let mut jobs = Vec::new();
    for (pubkey, account) in accounts {
        if account.data.len() < 100 {
            continue;
        }

        if let Ok(job) = JobAccount::deserialize(&mut &account.data[..]) {
            if job.creator == *creator {
                jobs.push((pubkey, job));
            }
        }
    }

    Ok(jobs)
}

/// Query all jobs claimed by a specific prover
pub fn find_jobs_by_prover(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    prover: &Pubkey,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    let accounts = rpc_client.get_program_accounts(program_id)?;

    let mut jobs = Vec::new();
    for (pubkey, account) in accounts {
        if account.data.len() < 100 {
            continue;
        }

        if let Ok(job) = JobAccount::deserialize(&mut &account.data[..]) {
            if let Some(job_prover) = job.prover {
                if job_prover == *prover {
                    jobs.push((pubkey, job));
                }
            }
        }
    }

    Ok(jobs)
}

/// Find all pending jobs (available to claim)
pub fn find_pending_jobs(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    find_jobs_by_status(rpc_client, program_id, JobStatus::Pending)
}

/// Find all active jobs (currently being worked on)
pub fn find_claimed_jobs(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    find_jobs_by_status(rpc_client, program_id, JobStatus::Claimed)
}

/// Find all completed jobs
pub fn find_completed_jobs(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, JobAccount)>> {
    find_jobs_by_status(rpc_client, program_id, JobStatus::Completed)
}

// ============================================================================
// Prover Queries
// ============================================================================

/// Query all registered provers
pub fn find_all_provers(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, ProverAccount)>> {
    let accounts = rpc_client.get_program_accounts(program_id)?;

    let mut provers = Vec::new();
    for (pubkey, account) in accounts {
        // Prover accounts are smaller than job accounts
        if account.data.len() > 50 && account.data.len() < 100 {
            if let Ok(prover) = ProverAccount::deserialize(&mut &account.data[..]) {
                provers.push((pubkey, prover));
            }
        }
    }

    Ok(provers)
}

/// Find provers with reputation above a threshold
pub fn find_provers_by_reputation(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    min_reputation: u32,
) -> Result<Vec<(Pubkey, ProverAccount)>> {
    let all_provers = find_all_provers(rpc_client, program_id)?;

    let filtered = all_provers
        .into_iter()
        .filter(|(_, prover)| prover.reputation_score >= min_reputation)
        .collect();

    Ok(filtered)
}

// ============================================================================
// Event Parsing
// ============================================================================

/// Parse program logs for events
#[derive(Debug, Clone)]
pub enum MarketplaceEvent {
    MarketplaceInitialized {
        authority: Pubkey,
        fee_basis_points: u16,
    },
    ProverRegistered {
        prover_authority: Pubkey,
        stake_amount: u64,
    },
    JobCreated {
        job_id: u64,
        creator: Pubkey,
        price_lamports: u64,
    },
    JobClaimed {
        job_id: u64,
        prover: Pubkey,
    },
    ProofSubmitted {
        job_id: u64,
        prover: Pubkey,
    },
    JobCancelled {
        job_id: u64,
    },
    ProverSlashed {
        prover: Pubkey,
        amount: u64,
    },
}

/// Parse logs from a transaction to extract marketplace events
pub fn parse_events_from_logs(logs: &[String]) -> Vec<MarketplaceEvent> {
    let mut events = Vec::new();

    for log in logs {
        if log.contains("Program log: Marketplace initialized") {
            // Try to extract event details from subsequent logs
            events.push(MarketplaceEvent::MarketplaceInitialized {
                authority: Pubkey::default(), // Would need to parse from logs
                fee_basis_points: 0,
            });
        } else if log.contains("Program log: Prover registered successfully") {
            events.push(MarketplaceEvent::ProverRegistered {
                prover_authority: Pubkey::default(),
                stake_amount: 0,
            });
        } else if log.contains("Program log: Job created successfully") {
            events.push(MarketplaceEvent::JobCreated {
                job_id: 0,
                creator: Pubkey::default(),
                price_lamports: 0,
            });
        } else if log.contains("Program log: Job claimed successfully") {
            events.push(MarketplaceEvent::JobClaimed {
                job_id: 0,
                prover: Pubkey::default(),
            });
        } else if log.contains("Program log: Proof submitted and verified") {
            events.push(MarketplaceEvent::ProofSubmitted {
                job_id: 0,
                prover: Pubkey::default(),
            });
        } else if log.contains("Program log: Job cancelled successfully") {
            events.push(MarketplaceEvent::JobCancelled {
                job_id: 0,
            });
        } else if log.contains("Program log: Prover slashed") {
            events.push(MarketplaceEvent::ProverSlashed {
                prover: Pubkey::default(),
                amount: 0,
            });
        }
    }

    events
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if a job has timed out
pub fn is_job_timed_out(job: &JobAccount, current_timestamp: i64) -> bool {
    current_timestamp > job.timeout_at
}

/// Calculate estimated time remaining for a job
pub fn get_time_remaining(job: &JobAccount, current_timestamp: i64) -> Option<i64> {
    if job.claimed_at.is_some() {
        let remaining = job.timeout_at - current_timestamp;
        Some(remaining.max(0))
    } else {
        None
    }
}

/// Calculate platform fee for a given price
pub fn calculate_platform_fee(price_lamports: u64, fee_basis_points: u16) -> u64 {
    (price_lamports as u128 * fee_basis_points as u128 / 10_000) as u64
}

/// Calculate prover payout after fee
pub fn calculate_prover_payout(price_lamports: u64, fee_basis_points: u16) -> u64 {
    price_lamports - calculate_platform_fee(price_lamports, fee_basis_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        // 10% fee (1000 basis points)
        assert_eq!(calculate_platform_fee(1_000_000, 1000), 100_000);
        assert_eq!(calculate_prover_payout(1_000_000, 1000), 900_000);

        // 5% fee (500 basis points)
        assert_eq!(calculate_platform_fee(1_000_000, 500), 50_000);
        assert_eq!(calculate_prover_payout(1_000_000, 500), 950_000);
    }

    #[test]
    fn test_time_remaining() {
        let mut job = JobAccount {
            id: 0,
            creator: Pubkey::default(),
            prover: Some(Pubkey::default()),
            status: JobStatus::Claimed,
            circuit_type: CircuitType::ZcashOrchard,
            witness_commitment: [0u8; 32],
            witness_size: 1024,
            proof_commitment: None,
            proof_size: None,
            price_lamports: 1_000_000,
            escrow_account: Pubkey::default(),
            created_at: 1000,
            claimed_at: Some(1000),
            completed_at: None,
            timeout_at: 1600, // Timeout at timestamp 1600 (600 seconds after claim at 1000)
            bump: 0,
        };

        // 300 seconds elapsed, 300 remaining
        assert_eq!(get_time_remaining(&job, 1300), Some(300));

        // 700 seconds elapsed, timed out
        assert_eq!(get_time_remaining(&job, 1700), Some(0));
        assert!(is_job_timed_out(&job, 1700));

        // Not claimed yet
        job.claimed_at = None;
        assert_eq!(get_time_remaining(&job, 1000), None);
        assert!(!is_job_timed_out(&job, 1000));
    }
}
