use borsh::BorshDeserialize;
use chrono::{DateTime, NaiveDateTime, Utc};
use zyberlink_sdk::{JobAccount, FheConsensusData, fetch_fhe_consensus};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::time::Duration;

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
    circuit_type >= 4 && circuit_type <= 11
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
        .unwrap_or_else(|| Utc::now())
        .naive_utc()
}

/// Start background task that syncs blockchain jobs to PostgreSQL
pub fn start_chain_sync(rpc_url: String, program_id: Pubkey, db_pool: PgPool) {
    tokio::spawn(async move {
        log::info!("Starting blockchain sync task...");
        log::info!("  RPC URL: {}", rpc_url);
        log::info!("  Program ID: {}", program_id);
        log::info!("  Sync Interval: 8 seconds");

        loop {
            match sync_jobs(rpc_url.clone(), program_id, &db_pool).await {
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
async fn sync_jobs(
    rpc_url: String,
    program_id: Pubkey,
    db_pool: &PgPool,
) -> anyhow::Result<usize> {
    // Execute blocking RPC call in a separate thread pool
    let accounts = tokio::task::spawn_blocking(move || {
        // Create RPC client inside spawn_blocking (blocking context)
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

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

        rpc_client.get_program_accounts_with_config(&program_id, config)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
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
                log::debug!("First 50 bytes: {:?}", &account.data[..account.data.len().min(50)]);
            }
        }
    }

    Ok(synced_count)
}

/// Insert or update a job in the database
async fn upsert_job(db_pool: &PgPool, pubkey: &Pubkey, job: &JobAccount) -> anyhow::Result<()> {
    // Extract FHE operation name from circuit type
    let fhe_operation = fhe_operation_name(job.circuit_type);

    // Convert timestamps
    let created_at = timestamp_to_naive(job.created_at);
    let timeout_at = timestamp_to_naive(job.timeout_at);

    // FHE config values will be populated from FheConsensusData if available
    // For now, we don't have access to the RPC client here, so we use defaults for FHE jobs
    let (required_provers, consensus_threshold): (Option<i16>, Option<i16>) = if is_fhe_circuit(job.circuit_type) {
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

    let completed_at: Option<NaiveDateTime> = if job.status == zyberlink_types::JobStatus::Completed {
        Some(timestamp_to_naive(job.timeout_at)) // Use timeout_at as approximation
    } else {
        None
    };

    sqlx::query!(
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
            fhe_operation
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (job_id) DO UPDATE SET
            prover_pubkey = EXCLUDED.prover_pubkey,
            status = EXCLUDED.status,
            claimed_at = EXCLUDED.claimed_at,
            completed_at = EXCLUDED.completed_at,
            synced_at = NOW()
        "#,
        job.id as i64,
        pubkey.to_string(),
        job.creator.to_string(),
        job.prover.map(|p| p.to_string()),
        status_str,
        circuit_type_str,
        job.price_lamports as i64,
        created_at,
        claimed_at,
        completed_at,
        timeout_at,
        required_provers,
        consensus_threshold,
        fhe_operation,
    )
    .execute(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("Database upsert failed: {}", e))?;

    Ok(())
}
