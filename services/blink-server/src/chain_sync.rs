use borsh::BorshDeserialize;
use chrono::{DateTime, NaiveDateTime, Utc};
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use zyberlink_sdk::JobAccount;
use zyberlink_chain_client::{ChainClient, SolanaClient, SolanaSpecificOps};

use crate::db::NetworkMetricsQueries;

// Circuit type ID constants (must match program)
const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
const CIRCUIT_CREDENTIAL: u8 = 3;
const CIRCUIT_FHE_ADD: u8 = 4;
const CIRCUIT_FHE_MULTIPLY: u8 = 5;
const CIRCUIT_FHE_SUM: u8 = 6;
const CIRCUIT_FHE_THRESHOLD: u8 = 7;
const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
const CIRCUIT_FHE_AVERAGE: u8 = 9;
const CIRCUIT_FHE_COUNT_IF: u8 = 10;
const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

/// Check if circuit_type u8 represents an FHE job
fn is_fhe_circuit(circuit_type: u8) -> bool {
    (4..=11).contains(&circuit_type)
}

/// Convert circuit_type u8 to string
fn circuit_type_to_str(circuit_type: u8) -> &'static str {
    match circuit_type {
        CIRCUIT_ZCASH_ORCHARD | 1 => "zcash_orchard",
        CIRCUIT_ANONYMOUS_VOTE => "anonymous_vote",
        CIRCUIT_CREDENTIAL => "credential",
        CIRCUIT_FHE_ADD..=CIRCUIT_FHE_HISTOGRAM => "fhe_computation",
        _ => "custom",
    }
}

/// Get FHE operation name from circuit type
fn fhe_operation_name(circuit_type: u8) -> Option<String> {
    match circuit_type {
        CIRCUIT_FHE_ADD => Some("Add".to_string()),
        CIRCUIT_FHE_MULTIPLY => Some("Multiply".to_string()),
        CIRCUIT_FHE_SUM => Some("Sum".to_string()),
        CIRCUIT_FHE_THRESHOLD => Some("Threshold".to_string()),
        CIRCUIT_FHE_RANGE_CHECK => Some("RangeCheck".to_string()),
        CIRCUIT_FHE_AVERAGE => Some("Average".to_string()),
        CIRCUIT_FHE_COUNT_IF => Some("CountIf".to_string()),
        CIRCUIT_FHE_HISTOGRAM => Some("Histogram".to_string()),
        _ => None,
    }
}

/// Convert Unix timestamp (i64) to NaiveDateTime for PostgreSQL
fn timestamp_to_naive(ts: i64) -> NaiveDateTime {
    DateTime::from_timestamp(ts, 0)
        .unwrap_or_else(Utc::now)
        .naive_utc()
}

/// Start background task that syncs blockchain jobs to PostgreSQL
///
/// # Multi-chain Ready
/// This function now accepts a SolanaClient instead of raw RPC URL.
/// For multi-chain support, verticales can pass MockChainClient during development.
pub fn start_chain_sync(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: PgPool,
) {
    tokio::spawn(async move {
        log::info!("Starting blockchain sync task...");
        log::info!("  Chain: {}", client.chain_id());
        log::info!("  Network: {}", client.network());
        log::info!("  Program ID: {}", program_id);
        log::info!("  Sync Interval: 8 seconds");

        loop {
            match sync_jobs(Arc::clone(&client), program_id, &db_pool).await {
                Ok(count) => {
                    log::info!("Synced {} jobs from blockchain", count);
                }
                Err(e) => {
                    log::error!("Blockchain sync error: {}", e);
                }
            }

            // Wait 8 seconds before next sync
            tokio::time::sleep(Duration::from_secs(8)).await;
        }
    });
}

/// Sync all job accounts from blockchain to database
///
/// # Multi-chain Ready
/// Uses SolanaSpecificOps trait for Solana-specific operations like get_program_accounts.
async fn sync_jobs(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: &PgPool,
) -> anyhow::Result<usize> {
    // Fetch all program accounts with JobAccount size filter
    // JobAccount has a fixed size of 203 bytes (updated structure)
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(203)]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Use SolanaSpecificOps trait for get_program_accounts
    let accounts = client
        .get_program_accounts(&program_id, Some(config))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch program accounts: {}", e))?;

    log::debug!("Fetched {} accounts from blockchain", accounts.len());

    let mut synced_count = 0;

    for (pubkey, account) in accounts {
        // Debug: print first 20 bytes
        let preview = &account.data[..account.data.len().min(20)];
        log::debug!(
            "Account {} data preview (first 20 bytes): {:?}",
            pubkey,
            preview
        );

        // Deserialize account data as JobAccount
        match JobAccount::deserialize(&mut &account.data[..]) {
            Ok(job) => {
                // Insert or update in database
                if let Err(e) = upsert_job(db_pool, &pubkey, &job).await {
                    log::warn!("Failed to upsert job {}: {}", job.id, e);
                } else {
                    synced_count += 1;
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to deserialize account {} (size: {}): {}",
                    pubkey,
                    account.data.len(),
                    e
                );
                log::debug!(
                    "First 50 bytes: {:?}",
                    &account.data[..account.data.len().min(50)]
                );
            }
        }
    }

    Ok(synced_count)
}

/// Insert or update a job in the database
async fn upsert_job(db_pool: &PgPool, pubkey: &Pubkey, job: &JobAccount) -> anyhow::Result<()> {
    // Check if job is newly completed (for metrics tracking)
    let is_newly_completed = if job.status == zyberlink_types::JobStatus::Completed {
        // Check if this job was already marked completed in the database
        let existing_status: Option<(String,)> = sqlx::query_as(
            r#"SELECT status FROM blockchain_jobs WHERE job_id = $1"#
        )
        .bind(job.id as i64)
        .fetch_optional(db_pool)
        .await
        .ok()
        .flatten();

        // Job is "newly completed" if it either doesn't exist or had a different status
        existing_status.as_ref().map_or(true, |(s,)| s != "completed")
    } else {
        false
    };

    // Extract FHE operation name from circuit type
    let fhe_operation = fhe_operation_name(job.circuit_type);

    // Convert timestamps
    let created_at = timestamp_to_naive(job.created_at);
    let timeout_at = timestamp_to_naive(job.timeout_at);

    // FHE config values will be populated from FheConsensusData if available
    // For now, we don't have access to the RPC client here, so we use defaults for FHE jobs
    let (required_provers, consensus_threshold): (Option<i16>, Option<i16>) =
        if is_fhe_circuit(job.circuit_type) {
            // Default values - ideally we'd fetch FheConsensusData
            (Some(3), Some(2))
        } else {
            (None, None)
        };

    // Convert enums to strings for database storage
    let status_str = match job.status {
        zyberlink_types::JobStatus::Pending => "pending",
        zyberlink_types::JobStatus::Claimed => "claimed",
        zyberlink_types::JobStatus::Completed => "completed",
        zyberlink_types::JobStatus::Failed => "failed",
        zyberlink_types::JobStatus::Cancelled => "cancelled",
    };

    let circuit_type_str = circuit_type_to_str(job.circuit_type);

    // Note: claimed_at and completed_at are no longer stored in JobAccount
    // They would need to be tracked separately or derived from status changes
    let claimed_at: Option<NaiveDateTime> = if job.status == zyberlink_types::JobStatus::Claimed {
        Some(timestamp_to_naive(job.created_at)) // Use created_at as approximation
    } else {
        None
    };

    let completed_at: Option<NaiveDateTime> = if job.status == zyberlink_types::JobStatus::Completed
    {
        Some(timestamp_to_naive(job.timeout_at)) // Use timeout_at as approximation
    } else {
        None
    };

    // Convert witness_hash bytes to hex string for database storage
    let witness_hash_hex = hex::encode(job.witness_hash);

    sqlx::query(
        r#"
        INSERT INTO blockchain_jobs (
            job_id,
            pubkey,
            creator_pubkey,
            prover_pubkey,
            status,
            circuit_type,
            price_lamports,
            created_at,
            claimed_at,
            completed_at,
            timeout_at,
            required_provers,
            consensus_threshold,
            fhe_operation,
            witness_hash
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (job_id) DO UPDATE SET
            prover_pubkey = EXCLUDED.prover_pubkey,
            status = EXCLUDED.status,
            claimed_at = EXCLUDED.claimed_at,
            completed_at = EXCLUDED.completed_at,
            witness_hash = EXCLUDED.witness_hash,
            synced_at = NOW()
        "#
    )
    .bind(job.id as i64)
    .bind(pubkey.to_string())
    .bind(job.creator.to_string())
    .bind(job.prover.map(|p| p.to_string()))
    .bind(status_str)
    .bind(circuit_type_str)
    .bind(job.price_lamports as i64)
    .bind(created_at)
    .bind(claimed_at)
    .bind(completed_at)
    .bind(timeout_at)
    .bind(required_provers)
    .bind(consensus_threshold)
    .bind(fhe_operation)
    .bind(witness_hash_hex)
    .execute(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("Database upsert failed: {}", e))?;

    // If job just completed, update historical network metrics
    if is_newly_completed {
        log::info!(
            "Job {} completed - updating historical metrics (circuit_type: {})",
            job.id,
            job.circuit_type
        );

        // Get the data size from temp_job_data if available
        let data_size_result: Option<(i64,)> = sqlx::query_as(
            r#"SELECT COALESCE(LENGTH(encrypted_data) + LENGTH(server_key), 0)::bigint FROM temp_job_data WHERE job_id = $1"#
        )
        .bind(job.id as i64)
        .fetch_optional(db_pool)
        .await
        .ok()
        .flatten();
        let data_size = data_size_result.map(|(s,)| s).unwrap_or(0);

        // If no temp_job_data, use witness data size
        let final_data_size = if data_size == 0 {
            let witness_hash_hex = hex::encode(job.witness_hash);
            let witness_size_result: Option<(i64,)> = sqlx::query_as(
                r#"SELECT COALESCE(LENGTH(data), 0)::bigint FROM witnesses WHERE commitment = $1"#
            )
            .bind(&witness_hash_hex)
            .fetch_optional(db_pool)
            .await
            .ok()
            .flatten();
            witness_size_result.map(|(s,)| s).unwrap_or(0)
        } else {
            data_size
        };

        // Record in historical metrics
        let is_fhe = is_fhe_circuit(job.circuit_type);
        if let Err(e) = NetworkMetricsQueries::record_completed_job(db_pool, final_data_size, is_fhe).await {
            log::warn!("Failed to update network metrics for job {}: {}", job.id, e);
        } else {
            log::info!(
                "Updated historical metrics: +{} bytes, is_fhe={}",
                final_data_size,
                is_fhe
            );
        }
    }

    Ok(())
}
