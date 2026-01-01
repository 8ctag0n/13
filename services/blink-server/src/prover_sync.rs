use borsh::BorshDeserialize;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use zyberlink_sdk::ProverAccount;
use zyberlink_chain_client::{ChainClient, SolanaClient, SolanaSpecificOps};

/// ProverAccount size in bytes (must match on-chain program)
const PROVER_ACCOUNT_SIZE: u64 = 114;

/// Start background task that syncs provers from blockchain to PostgreSQL
///
/// # Multi-chain Ready
/// This function now accepts a SolanaClient instead of raw RPC URL.
/// For multi-chain support, verticales can pass MockChainClient during development.
pub fn start_prover_sync(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: PgPool,
) {
    tokio::spawn(async move {
        log::info!("Starting prover sync task...");
        log::info!("  Chain: {}", client.chain_id());
        log::info!("  Network: {}", client.network());
        log::info!("  Program ID: {}", program_id);
        log::info!("  Sync Interval: 12 seconds");

        loop {
            match sync_provers(Arc::clone(&client), program_id, &db_pool).await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Synced {} provers from blockchain", count);
                    }
                }
                Err(e) => {
                    log::error!("Prover sync error: {}", e);
                }
            }

            // Wait 12 seconds before next sync
            tokio::time::sleep(Duration::from_secs(12)).await;
        }
    });
}

/// Sync all prover accounts from blockchain to database
///
/// # Multi-chain Ready
/// Uses SolanaSpecificOps trait for Solana-specific operations like get_program_accounts.
async fn sync_provers(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: &PgPool,
) -> anyhow::Result<usize> {
    // Fetch all program accounts with ProverAccount size filter
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(PROVER_ACCOUNT_SIZE)]),
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
        .map_err(|e| anyhow::anyhow!("Failed to fetch prover accounts: {}", e))?;

    log::debug!("Fetched {} prover accounts from blockchain", accounts.len());

    let mut synced_count = 0;

    for (pubkey, account) in accounts {
        match ProverAccount::deserialize(&mut &account.data[..]) {
            Ok(prover) => {
                if let Err(e) = upsert_prover(db_pool, &pubkey, &prover).await {
                    log::warn!("Failed to upsert prover {}: {}", pubkey, e);
                } else {
                    synced_count += 1;
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to deserialize prover account {} (size: {}): {}",
                    pubkey,
                    account.data.len(),
                    e
                );
            }
        }
    }

    Ok(synced_count)
}

/// Insert or update a prover in the database
async fn upsert_prover(
    db_pool: &PgPool,
    pubkey: &Pubkey,
    prover: &ProverAccount,
) -> anyhow::Result<()> {
    // Convert encryption_pubkey to hex string
    let encryption_pubkey_hex = hex::encode(prover.encryption_pubkey);

    sqlx::query(
        r#"
        INSERT INTO provers (
            pubkey,
            authority_pubkey,
            stake_amount,
            reputation_score,
            is_active,
            total_jobs_completed,
            total_jobs_failed,
            avg_completion_time_secs,
            total_earnings_lamports,
            encryption_pubkey,
            registration_timestamp,
            last_seen_at,
            synced_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, to_timestamp($11), NOW(), NOW())
        ON CONFLICT (pubkey) DO UPDATE SET
            stake_amount = EXCLUDED.stake_amount,
            reputation_score = EXCLUDED.reputation_score,
            is_active = EXCLUDED.is_active,
            total_jobs_completed = EXCLUDED.total_jobs_completed,
            total_jobs_failed = EXCLUDED.total_jobs_failed,
            avg_completion_time_secs = EXCLUDED.avg_completion_time_secs,
            total_earnings_lamports = EXCLUDED.total_earnings_lamports,
            encryption_pubkey = EXCLUDED.encryption_pubkey,
            last_seen_at = NOW(),
            synced_at = NOW()
        "#
    )
    .bind(pubkey.to_string())
    .bind(prover.authority.to_string())
    .bind(prover.stake_amount as i64)
    .bind(prover.reputation_score as i32)
    .bind(prover.is_active)
    .bind(prover.total_jobs_completed as i64)
    .bind(prover.total_jobs_failed as i64)
    .bind(prover.avg_completion_time_secs as i32)
    .bind(prover.total_earnings_lamports as i64)
    .bind(encryption_pubkey_hex)
    .bind(prover.registration_timestamp as f64)
    .execute(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("Database upsert failed: {}", e))?;

    Ok(())
}
