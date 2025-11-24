use borsh::BorshDeserialize;
use chrono::{DateTime, NaiveDateTime, Utc};
use cypherlink_sdk::JobAccount;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::time::Duration;

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

        // Fetch all program accounts with JobAccount discriminator
        // JobAccount has a fixed size of 879 bytes
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(879)]),
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
    // Extract FHE operation name if present
    let fhe_operation = job
        .fhe_config
        .as_ref()
        .map(|cfg| cfg.operation.name().to_string());

    // Convert timestamps
    let created_at = timestamp_to_naive(job.created_at);
    let claimed_at = job.claimed_at.map(timestamp_to_naive);
    let completed_at = job.completed_at.map(timestamp_to_naive);
    let timeout_at = timestamp_to_naive(job.timeout_at);

    // Extract FHE config values
    let (required_provers, consensus_threshold) = job
        .fhe_config
        .as_ref()
        .map(|cfg| (Some(cfg.required_provers as i16), Some(cfg.consensus_threshold as i16)))
        .unwrap_or((None, None));

    // Convert enums to strings for database storage
    let status_str = match job.status {
        cypherlink_types::JobStatus::Pending => "pending",
        cypherlink_types::JobStatus::Claimed => "claimed",
        cypherlink_types::JobStatus::Completed => "completed",
        cypherlink_types::JobStatus::Failed => "failed",
        cypherlink_types::JobStatus::Cancelled => "cancelled",
    };

    let circuit_type_str = match &job.circuit_type {
        cypherlink_types::CircuitType::ZcashOrchard => "zcash_orchard",
        cypherlink_types::CircuitType::AnonymousVote => "anonymous_vote",
        cypherlink_types::CircuitType::Credential => "credential",
        cypherlink_types::CircuitType::FheComputation(_) => "fhe_computation",
        cypherlink_types::CircuitType::Custom(_) => "custom",
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
