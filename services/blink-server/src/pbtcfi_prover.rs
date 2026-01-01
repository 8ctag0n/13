//! pBTCFi FHE Prover Worker
//!
//! Background task that processes FHE verification jobs for pBTCFi loans.
//! Takes pending jobs, computes FHE operations on encrypted data,
//! and submits results to update loan status.

use std::time::Duration;
use sqlx::PgPool;
use anyhow::Result;

use crate::db::pbtcfi_queries::{FhePendingLoan, PbtcfiQueries};

/// Prover configuration
pub struct PbtcfiProverConfig {
    /// Unique identifier for this prover instance
    pub prover_id: String,
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Maximum jobs to process per cycle
    pub max_jobs_per_cycle: i64,
}

impl Default for PbtcfiProverConfig {
    fn default() -> Self {
        Self {
            prover_id: format!("pbtcfi-prover-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            poll_interval_secs: 5,
            max_jobs_per_cycle: 5,
        }
    }
}

/// Start the pBTCFi FHE prover background task
pub fn start_pbtcfi_prover(db_pool: PgPool, config: PbtcfiProverConfig) {
    tokio::spawn(async move {
        log::info!("Starting pBTCFi FHE prover: {}", config.prover_id);
        log::info!("  Poll interval: {}s", config.poll_interval_secs);
        log::info!("  Max jobs/cycle: {}", config.max_jobs_per_cycle);

        loop {
            match process_pending_jobs(&db_pool, &config).await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("[{}] Processed {} FHE jobs", config.prover_id, count);
                    }
                }
                Err(e) => {
                    log::error!("[{}] Prover error: {}", config.prover_id, e);
                }
            }

            tokio::time::sleep(Duration::from_secs(config.poll_interval_secs)).await;
        }
    });
}

/// Process pending FHE jobs
async fn process_pending_jobs(db_pool: &PgPool, config: &PbtcfiProverConfig) -> Result<usize> {
    // Get pending jobs
    let pending = PbtcfiQueries::get_pending_fhe_jobs(db_pool, config.max_jobs_per_cycle).await?;

    if pending.is_empty() {
        return Ok(0);
    }

    log::debug!("[{}] Found {} pending FHE jobs", config.prover_id, pending.len());

    let mut processed = 0;

    for job in pending {
        match process_single_job(db_pool, &config.prover_id, &job).await {
            Ok(()) => {
                processed += 1;
                log::info!(
                    "[{}] Successfully processed FHE job for loan {}",
                    config.prover_id,
                    job.loan_id
                );
            }
            Err(e) => {
                log::warn!(
                    "[{}] Failed to process job {}: {}",
                    config.prover_id,
                    job.loan_id,
                    e
                );
            }
        }
    }

    Ok(processed)
}

/// Process a single FHE job
async fn process_single_job(
    db_pool: &PgPool,
    prover_id: &str,
    job: &FhePendingLoan,
) -> Result<()> {
    // 1. Claim the job atomically
    let claimed = PbtcfiQueries::claim_fhe_job(db_pool, &job.loan_id, prover_id).await?;

    if !claimed {
        log::debug!("Job {} already claimed by another prover", job.loan_id);
        return Ok(());
    }

    log::info!(
        "[{}] Claimed job {} - starting FHE computation",
        prover_id,
        job.loan_id
    );

    // 2. Perform FHE computation
    let result = compute_fhe_collateral_value(job).await;

    match result {
        Ok((plst_c1, plst_c2)) => {
            // 3. Submit result
            PbtcfiQueries::complete_fhe_job(
                db_pool,
                &job.loan_id,
                &plst_c1, // collateral_value_c1 (not used yet)
                &plst_c2, // collateral_value_c2 (not used yet)
                &plst_c1, // plst_amount_c1
                &plst_c2, // plst_amount_c2
            )
            .await?;

            // 4. Create callback for Starknet (mint_encrypted on pLST)
            let plst_contract = std::env::var("PLST_CONTRACT_ADDRESS")
                .unwrap_or_else(|_| "0x0".to_string());

            let call_args = serde_json::json!([
                job.borrower,  // to: ContractAddress
                plst_c1,       // encrypted_c1: felt252
                plst_c2        // encrypted_c2: felt252
            ]);

            if let Err(e) = PbtcfiQueries::create_callback(
                db_pool,
                &job.loan_id,
                "mint_encrypted",
                &plst_contract,
                "mint_encrypted",
                &call_args,
            ).await {
                log::warn!(
                    "[{}] Failed to create Starknet callback for {}: {}",
                    prover_id,
                    job.loan_id,
                    e
                );
            }

            log::info!(
                "[{}] Completed FHE job {} - loan activated, callback created",
                prover_id,
                job.loan_id
            );
        }
        Err(e) => {
            // Mark as failed
            PbtcfiQueries::fail_fhe_job(db_pool, &job.loan_id, &e.to_string()).await?;
            return Err(e);
        }
    }

    Ok(())
}

/// FHE computation for collateral value
///
/// Takes encrypted BTC amount and computes:
/// 1. collateral_value = btc_encrypted * btc_price (FHE multiply)
/// 2. plst_amount = collateral_value * ltv_ratio (FHE multiply)
///
/// Returns: (plst_encrypted_c1, plst_encrypted_c2)
async fn compute_fhe_collateral_value(job: &FhePendingLoan) -> Result<(String, String)> {
    log::debug!(
        "Computing FHE for loan {} with btc_encrypted: ({}, {})",
        job.loan_id,
        job.btc_encrypted_c1,
        job.btc_encrypted_c2
    );

    // ==========================================================================
    // MVP: Mock FHE computation
    // In production, this would:
    // 1. Load the server key for homomorphic operations
    // 2. Perform FHE multiplication: encrypted_btc * public_btc_price
    // 3. Perform FHE multiplication: result * ltv_ratio
    // 4. Return encrypted pLST amount
    // ==========================================================================

    // Mock: Transform the encrypted values (simulating FHE operation)
    // In reality, this would be actual FHE compute using tfhe-rs or similar

    let btc_c1 = parse_felt252(&job.btc_encrypted_c1)?;
    let btc_c2 = parse_felt252(&job.btc_encrypted_c2)?;

    // Simulate FHE multiply by price (60000) and LTV (75%)
    // Real FHE: result = Enc(btc) * 60000 * 0.75 = Enc(btc * 45000)
    // Mock: We just apply a transformation that preserves the structure
    let price_ltv_factor: u128 = 45000; // 60000 * 0.75

    // Mock homomorphic multiplication (NOT real FHE, just for testing flow)
    let plst_c1 = btc_c1.wrapping_mul(price_ltv_factor);
    let plst_c2 = btc_c2.wrapping_mul(price_ltv_factor);

    // Format back to felt252 hex strings
    let plst_c1_hex = format!("0x{:064x}", plst_c1);
    let plst_c2_hex = format!("0x{:064x}", plst_c2);

    log::debug!(
        "FHE result for loan {}: pLST = ({}, {})",
        job.loan_id,
        plst_c1_hex,
        plst_c2_hex
    );

    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((plst_c1_hex, plst_c2_hex))
}

/// Parse felt252 hex string to u128 (simplified for MVP)
fn parse_felt252(hex: &str) -> Result<u128> {
    let hex_clean = hex.trim_start_matches("0x");

    // Take last 32 chars (128 bits) for u128
    let truncated = if hex_clean.len() > 32 {
        &hex_clean[hex_clean.len() - 32..]
    } else {
        hex_clean
    };

    u128::from_str_radix(truncated, 16)
        .map_err(|e| anyhow::anyhow!("Failed to parse felt252: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_felt252() {
        let result = parse_felt252("0x0000000000000000000000000000000000000000000000000000000000000064").unwrap();
        assert_eq!(result, 100);

        let result = parse_felt252("0x64").unwrap();
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_compute_fhe_mock() {
        let job = FhePendingLoan {
            loan_id: "test-loan-1".to_string(),
            borrower: "0x123".to_string(),
            btc_encrypted_c1: "0x0000000000000000000000000000000000000000000000000000000000000064".to_string(), // 100
            btc_encrypted_c2: "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(), // 1
            btc_commitment: "0xabc".to_string(),
            created_at: 0,
        };

        let (c1, c2) = compute_fhe_collateral_value(&job).await.unwrap();

        // 100 * 45000 = 4500000 = 0x44AA20
        assert!(c1.contains("44aa20") || c1.contains("44AA20"));
    }
}
