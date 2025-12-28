//! Cross-program job queries
//!
//! Queries jobs from ZK-Generator, FHE-Generator, and legacy programs.

use borsh::BorshDeserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::error::{Result, UnifiedError};
use crate::query::UnifiedJob;
use zyberlink_jobs::JobCommon;
pub use zyberlink_types::JobStatus;

/// FHE Consensus Data (from on-chain)
/// Matches src/programs/fhe-generator/src/state/consensus.rs
#[derive(Debug, Clone, BorshDeserialize)]
pub struct FheConsensusData {
    pub job_id: u64,
    pub operation_type: u8,
    pub operation_param1: u16,
    pub operation_param2: u8,
    pub operation_param3: u8,
    pub required_provers: u8,
    pub consensus_threshold: u8,
    pub submission_timeout: i64,
    pub claimed_provers: [Pubkey; 5],
    pub claimed_count: u8,
    pub result_hashes: [[u8; 32]; 5],
    pub result_submitted: [bool; 5],
    pub results_count: u8,
    pub consensus_hash: Option<[u8; 32]>,
    pub bump: u8,
}

/// On-chain ZK Job structure
/// Matches src/programs/zk-generator/src/state/job.rs
#[derive(Debug, Clone, BorshDeserialize)]
struct ZkJobAccount {
    pub common: JobCommon,
    pub circuit_type: u8,
}

/// On-chain FHE Job structure
/// Matches src/programs/fhe-generator/src/state/job.rs
#[derive(Debug, Clone, BorshDeserialize)]
struct FheJobAccount {
    pub common: JobCommon,
    pub circuit_type: u8,
    pub fhe_consensus_bump: u8,
}

/// Cross-program job query client
pub struct JobQuery {
    client: Arc<RpcClient>,
    zk_generator_program: Pubkey,
    fhe_generator_program: Pubkey,
    legacy_program: Option<Pubkey>,
}

impl JobQuery {
    /// Create a new JobQuery with program IDs
    pub fn new(
        client: Arc<RpcClient>,
        zk_generator_program: Pubkey,
        fhe_generator_program: Pubkey,
        legacy_program: Option<Pubkey>,
    ) -> Self {
        Self {
            client,
            zk_generator_program,
            fhe_generator_program,
            legacy_program,
        }
    }

    /// Create JobQuery from RPC URL with program IDs
    pub fn from_url(
        rpc_url: &str,
        zk_generator_program: Pubkey,
        fhe_generator_program: Pubkey,
    ) -> Self {
        Self {
            client: Arc::new(RpcClient::new(rpc_url.to_string())),
            zk_generator_program,
            fhe_generator_program,
            legacy_program: None,
        }
    }

    /// Get all pending jobs from all configured generators
    pub async fn get_pending_jobs(&self) -> Result<Vec<UnifiedJob>> {
        let mut all_jobs = Vec::new();

        // Query ZK jobs
        match self.get_zk_jobs_by_status(JobStatus::Pending) {
            Ok(jobs) => all_jobs.extend(jobs),
            Err(e) => log::warn!("Failed to query ZK jobs: {}", e),
        }

        // Query FHE jobs
        match self.get_fhe_jobs_by_status(JobStatus::Pending) {
            Ok(jobs) => all_jobs.extend(jobs),
            Err(e) => log::warn!("Failed to query FHE jobs: {}", e),
        }

        Ok(all_jobs)
    }

    /// Get FHE jobs that need more provers for consensus
    pub async fn get_fhe_jobs_needing_provers(&self, my_pubkey: &Pubkey) -> Result<Vec<UnifiedJob>> {
        let config = RpcProgramAccountsConfig {
            filters: None,
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.client
            .get_program_accounts_with_config(&self.fhe_generator_program, config)
            ?;

        // First pass: collect FheConsensusData by job_id
        let mut consensus_by_job: HashMap<u64, FheConsensusData> = HashMap::new();
        for (_pubkey, account) in &accounts {
            // FheConsensusData is ~384 bytes
            if account.data.len() >= 350 && account.data.len() <= 420 {
                if let Ok(consensus) = FheConsensusData::deserialize(&mut &account.data[..]) {
                    consensus_by_job.insert(consensus.job_id, consensus);
                }
            }
        }

        // Second pass: find FHE jobs that need provers
        let mut result = Vec::new();
        for (pubkey, account) in &accounts {
            // FheJob is 202 bytes
            if account.data.len() < 200 || account.data.len() > 210 {
                continue;
            }

            if let Ok(job) = FheJobAccount::deserialize(&mut &account.data[..]) {
                // Only FHE jobs (circuit_type 4-11)
                if job.circuit_type < 4 || job.circuit_type > 11 {
                    continue;
                }

                // Check if job needs provers
                if let Some(consensus) = consensus_by_job.get(&job.common.id) {
                    // Skip if already have enough provers
                    if consensus.claimed_count >= consensus.required_provers {
                        continue;
                    }

                    // Skip if I already claimed this job
                    let already_claimed = consensus.claimed_provers[..consensus.claimed_count as usize]
                        .iter()
                        .any(|p| p == my_pubkey);
                    if already_claimed {
                        continue;
                    }

                    // This job needs provers and I haven't claimed it
                    result.push(UnifiedJob::Fhe {
                        program_id: self.fhe_generator_program,
                        address: *pubkey,
                        common: job.common,
                        circuit_type: job.circuit_type,
                        consensus_bump: job.fhe_consensus_bump,
                    });
                } else if job.common.status == JobStatus::Pending {
                    // No consensus data yet = pending, can claim
                    result.push(UnifiedJob::Fhe {
                        program_id: self.fhe_generator_program,
                        address: *pubkey,
                        common: job.common,
                        circuit_type: job.circuit_type,
                        consensus_bump: job.fhe_consensus_bump,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Get a specific job by address
    pub async fn get_job(&self, job_address: &Pubkey) -> Result<UnifiedJob> {
        let account = self.client
            .get_account(job_address)?;

        let owner = account.owner;

        // Determine job type by owner program
        if owner == self.zk_generator_program {
            let job = ZkJobAccount::deserialize(&mut &account.data[..])
                .map_err(|e| UnifiedError::DeserializationError(e.to_string()))?;

            Ok(UnifiedJob::Zk {
                program_id: self.zk_generator_program,
                address: *job_address,
                common: job.common,
                circuit_type: job.circuit_type,
            })
        } else if owner == self.fhe_generator_program {
            let job = FheJobAccount::deserialize(&mut &account.data[..])
                .map_err(|e| UnifiedError::DeserializationError(e.to_string()))?;

            Ok(UnifiedJob::Fhe {
                program_id: self.fhe_generator_program,
                address: *job_address,
                common: job.common,
                circuit_type: job.circuit_type,
                consensus_bump: job.fhe_consensus_bump,
            })
        } else {
            Err(UnifiedError::UnknownProgram(owner))
        }
    }

    /// Get claimable jobs for a prover (pending ZK + FHE needing provers)
    pub async fn get_claimable_jobs(&self, prover: &Pubkey) -> Result<Vec<UnifiedJob>> {
        let mut all_jobs = Vec::new();

        // Get pending ZK jobs (single prover)
        match self.get_zk_jobs_by_status(JobStatus::Pending) {
            Ok(jobs) => all_jobs.extend(jobs),
            Err(e) => log::warn!("Failed to query ZK jobs: {}", e),
        }

        // Get FHE jobs needing provers (multi-prover consensus)
        match self.get_fhe_jobs_needing_provers(prover).await {
            Ok(jobs) => all_jobs.extend(jobs),
            Err(e) => log::warn!("Failed to query FHE jobs: {}", e),
        }

        Ok(all_jobs)
    }

    // ========== Internal helpers ==========

    fn get_zk_jobs_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>> {
        let config = RpcProgramAccountsConfig {
            filters: None,
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.client
            .get_program_accounts_with_config(&self.zk_generator_program, config)?;

        let mut jobs = Vec::new();
        for (pubkey, account) in accounts {
            // ZkJob is 201 bytes
            if account.data.len() < 200 || account.data.len() > 210 {
                continue;
            }

            if let Ok(job) = ZkJobAccount::deserialize(&mut &account.data[..]) {
                if job.common.status == status {
                    jobs.push(UnifiedJob::Zk {
                        program_id: self.zk_generator_program,
                        address: pubkey,
                        common: job.common,
                        circuit_type: job.circuit_type,
                    });
                }
            }
        }

        Ok(jobs)
    }

    fn get_fhe_jobs_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>> {
        let config = RpcProgramAccountsConfig {
            filters: None,
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.client
            .get_program_accounts_with_config(&self.fhe_generator_program, config)
            ?;

        let mut jobs = Vec::new();
        for (pubkey, account) in accounts {
            // FheJob is 202 bytes
            if account.data.len() < 200 || account.data.len() > 210 {
                continue;
            }

            if let Ok(job) = FheJobAccount::deserialize(&mut &account.data[..]) {
                if job.common.status == status {
                    jobs.push(UnifiedJob::Fhe {
                        program_id: self.fhe_generator_program,
                        address: pubkey,
                        common: job.common,
                        circuit_type: job.circuit_type,
                        consensus_bump: job.fhe_consensus_bump,
                    });
                }
            }
        }

        Ok(jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_query_creation() {
        let zk_program = Pubkey::new_unique();
        let fhe_program = Pubkey::new_unique();

        let query = JobQuery::from_url("http://localhost:8899", zk_program, fhe_program);

        assert_eq!(query.zk_generator_program, zk_program);
        assert_eq!(query.fhe_generator_program, fhe_program);
    }
}
