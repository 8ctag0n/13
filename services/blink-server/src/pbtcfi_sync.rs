//! pBTCFi Event Synchronization
//!
//! Background task that polls Starknet for pBTCFi contract events
//! and syncs them to PostgreSQL for API access.

use std::sync::Arc;
use std::time::Duration;
use sqlx::PgPool;
use anyhow::{anyhow, Result};

use crate::db::{pbtcfi_queries::{LoanData, PbtcfiQueries}};
use zyberlink_chain_client::{ChainClient, StarknetClient, StarknetSpecificOps, StarknetEvent};

/// Start the pBTCFi event synchronization background task
///
/// # Arguments
/// * `client` - StarknetClient instance
/// * `contract_address` - pBTCFi core contract address
/// * `db_pool` - PostgreSQL connection pool
pub fn start_pbtcfi_sync(
    client: Arc<StarknetClient>,
    contract_address: String,
    db_pool: PgPool,
) {
    tokio::spawn(async move {
        log::info!("Starting pBTCFi event sync task...");
        log::info!("  Contract: {}", contract_address);
        log::info!("  Sync Interval: 6 seconds");

        // Verify connection
        match client.health_check().await {
            Ok(true) => log::info!("Starknet connection verified"),
            Ok(false) | Err(_) => {
                log::error!("Starknet connection failed");
                log::error!("pBTCFi sync task will not start");
                return;
            }
        }

        loop {
            match sync_events(
                Arc::clone(&client),
                &contract_address,
                &db_pool,
            ).await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Synced {} pBTCFi events", count);
                    }
                }
                Err(e) => {
                    log::error!("pBTCFi sync error: {}", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(6)).await;
        }
    });
}

/// Sync events from Starknet to database
async fn sync_events(
    client: Arc<StarknetClient>,
    contract_address: &str,
    db_pool: &PgPool,
) -> Result<usize> {
    // 1. Get last synced block from database
    let last_block = PbtcfiQueries::get_last_synced_block(db_pool).await?;

    // 2. Get current block height from Starknet
    let current_block = client.get_block_height().await?;

    // Nothing new to sync
    if current_block <= last_block {
        return Ok(0);
    }

    // 3. Fetch events (max 1000 blocks per query to avoid timeouts)
    let to_block = std::cmp::min(last_block + 1000, current_block);

    log::debug!(
        "Syncing pBTCFi events from block {} to {}",
        last_block + 1,
        to_block
    );

    // Get events for LoanCreated (MVP: only this event type)
    // TODO: Add other event types (CollateralRegistered, LoanActivated, etc.)
    let events = client.get_events(
        last_block + 1,
        to_block,
        Some(contract_address),
        Some(vec![vec![EVENT_KEY_LOAN_CREATED.to_string()]]),
    ).await?;

    log::debug!("Fetched {} LoanCreated events", events.len());

    // 4. Process each event
    let mut synced_count = 0;
    for (index, event) in events.iter().enumerate() {
        match process_loan_created_event(db_pool, event, index as i32).await {
            Ok(_) => synced_count += 1,
            Err(e) => {
                log::warn!(
                    "Failed to process event at block {}, tx {}: {}",
                    event.block_number,
                    event.transaction_hash,
                    e
                );
            }
        }
    }

    // 5. Update last synced block
    PbtcfiQueries::update_last_synced_block(db_pool, to_block).await?;

    Ok(synced_count)
}

/// Process a LoanCreated event
async fn process_loan_created_event(
    db_pool: &PgPool,
    event: &StarknetEvent,
    event_index: i32,
) -> Result<()> {
    // Parse event data
    // Expected fields: [loan_id, borrower, btc_commitment, btc_encrypted.0, btc_encrypted.1, timestamp]
    if event.data.len() < 6 {
        return Err(anyhow!(
            "LoanCreated event has insufficient data fields: {}",
            event.data.len()
        ));
    }

    // Extract and normalize fields
    let loan_id = normalize_felt252(&event.data[0])?;
    let borrower = normalize_felt252(&event.data[1])?; // ContractAddress is also felt252
    let btc_commitment = normalize_felt252(&event.data[2])?;
    let btc_encrypted_c1 = normalize_felt252(&event.data[3])?;
    let btc_encrypted_c2 = normalize_felt252(&event.data[4])?;
    let timestamp = parse_u64_from_felt(&event.data[5])?;

    // Create loan data
    let loan = LoanData {
        loan_id: loan_id.clone(),
        borrower,
        btc_commitment,
        btc_encrypted_c1,
        btc_encrypted_c2,
        created_at: timestamp as i64,
    };

    // Upsert loan to database
    PbtcfiQueries::upsert_loan(db_pool, &loan).await?;

    // Insert event record for audit log
    let event_data = serde_json::json!({
        "loan_id": loan.loan_id,
        "borrower": loan.borrower,
        "btc_commitment": loan.btc_commitment,
        "btc_encrypted": [loan.btc_encrypted_c1, loan.btc_encrypted_c2],
        "timestamp": timestamp,
    });

    PbtcfiQueries::insert_event(
        db_pool,
        "LoanCreated",
        &loan.loan_id,
        event.block_number,
        &event.transaction_hash,
        event_index,
        &event_data,
        timestamp as i64,
    ).await?;

    // Create FHE verification job for the loan
    // This triggers the FHE provers to verify the encrypted collateral
    match PbtcfiQueries::create_fhe_job(db_pool, &loan.loan_id).await {
        Ok(()) => {
            log::info!(
                "Created FHE verification job for loan {} (borrower: {})",
                loan.loan_id,
                loan.borrower
            );
        }
        Err(e) => {
            // Log but don't fail - the loan is already in the DB
            // FHE job can be created manually or on retry
            log::warn!(
                "Failed to create FHE job for loan {}: {} (loan saved, job pending)",
                loan.loan_id,
                e
            );
        }
    }

    log::debug!("Processed LoanCreated event for loan {}", loan.loan_id);

    Ok(())
}

// =============================================================================
// Helper Functions (duplicated from pbtcfi/types/utils.rs for now)
// TODO: Share these via a common crate
// =============================================================================

fn normalize_felt252(felt: &str) -> Result<String> {
    let hex = felt.trim_start_matches("0x");

    if hex.is_empty() {
        return Err(anyhow!("Empty felt252 value"));
    }

    if hex.len() > 64 {
        return Err(anyhow!("felt252 too long: {} chars (max 64)", hex.len()));
    }

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("Invalid hex characters in felt252: {}", felt));
    }

    let padded = format!("{:0>64}", hex);
    Ok(format!("0x{}", padded))
}

fn parse_u64_from_felt(felt: &str) -> Result<u64> {
    let hex = felt.trim_start_matches("0x");
    u64::from_str_radix(hex, 16)
        .map_err(|e| anyhow!("Failed to parse u64 from felt252 '{}': {}", felt, e))
}

// =============================================================================
// Event Key Constants (calculated with starkli selector <name>)
// =============================================================================

/// Event selector for LoanCreated
/// Calculated with: starkli selector LoanCreated
const EVENT_KEY_LOAN_CREATED: &str = "0x03b632a8f9576e6775190ccd7c22bd55e4533d9e375fb405bc9ed5591accab4d";
