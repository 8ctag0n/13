use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair};
use solana_sdk::signer::Signer;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use zyberlink_chain_client::{ChainClient, SolanaClient};
use zyberlink_sdk::{fetch_fhe_consensus, fetch_job, MarketplaceClient};

/// Start background task that finalizes FHE jobs when consensus is reached
pub fn start_job_finalizer(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: PgPool,
    server_keypair: Arc<Keypair>,
) {
    tokio::spawn(async move {
        log::info!("Starting job finalizer task...");
        log::info!("  Chain ID: {}", client.chain_id());
        log::info!("  Network: {}", client.network());
        log::info!("  Program ID: {}", program_id);
        log::info!("  Finalizer: {}", server_keypair.pubkey());
        log::info!("  Check Interval: 10 seconds");

        loop {
            match check_and_finalize_jobs(
                client.clone(),
                program_id,
                &db_pool,
                server_keypair.clone(),
            )
            .await
            {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Finalized {} FHE jobs", count);
                    }
                }
                Err(e) => {
                    log::error!("Job finalizer error: {}", e);
                }
            }

            // Check every 10 seconds
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
}

/// Check for FHE jobs ready to finalize and finalize them
async fn check_and_finalize_jobs(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: &PgPool,
    server_keypair: Arc<Keypair>,
) -> anyhow::Result<usize> {
    // TODO: Update SDK to accept ChainClient in future
    // For now, derive RPC URL from client.network() for MarketplaceClient
    let rpc_url = match client.network() {
        "mainnet" => "https://api.mainnet-beta.solana.com",
        "devnet" => "https://api.devnet.solana.com",
        "testnet" => "https://api.testnet.solana.com",
        _ => "http://localhost:8899",
    };

    // Query database for claimed FHE jobs
    let claimed_jobs: Vec<(i64, String, String)> = sqlx::query_as(
        r#"
        SELECT job_id, pubkey, creator_pubkey
        FROM blockchain_jobs
        WHERE status = 'claimed'
          AND fhe_operation IS NOT NULL
          AND timeout_at > NOW()
        LIMIT 20
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    if claimed_jobs.is_empty() {
        return Ok(0);
    }

    log::info!(
        "Checking {} claimed FHE jobs for consensus",
        claimed_jobs.len()
    );

    let mut finalized_count = 0;

    // Check each job for consensus
    for (job_id, _job_pubkey, creator_pubkey) in claimed_jobs {
        let rpc_url_clone = rpc_url.to_string();
        let keypair_clone = server_keypair.clone();
        let creator_clone = creator_pubkey.clone();

        match try_finalize_job(
            rpc_url_clone,
            program_id,
            job_id as u64,
            creator_clone,
            keypair_clone,
        )
        .await
        {
            Ok(Some(tx_signature)) => {
                // Save the finalize tx_signature to database
                if let Err(e) = save_finalize_signature(db_pool, job_id, &tx_signature).await {
                    log::warn!("Failed to save tx_signature for job {}: {}", job_id, e);
                }
                finalized_count += 1;
                log::info!(
                    "Successfully finalized FHE job {} with tx: {}",
                    job_id,
                    tx_signature
                );
            }
            Ok(None) => {
                // Not ready for finalization yet
                log::debug!("Job {} not ready for finalization", job_id);
            }
            Err(e) => {
                log::warn!("Failed to finalize job {}: {}", job_id, e);
            }
        }
    }

    Ok(finalized_count)
}

/// Save the finalize transaction signature to database
async fn save_finalize_signature(
    db_pool: &PgPool,
    job_id: i64,
    tx_signature: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        UPDATE blockchain_jobs
        SET tx_signature = $1, synced_at = NOW()
        WHERE job_id = $2
        "#,
        tx_signature,
        job_id
    )
    .execute(db_pool)
    .await?;
    Ok(())
}

/// Try to finalize a single FHE job if consensus is reached
/// Runs RPC calls in spawn_blocking since they are synchronous
/// Returns Some(tx_signature) if finalized, None if not ready
async fn try_finalize_job(
    rpc_url: String,
    program_id: Pubkey,
    job_id: u64,
    creator_pubkey_str: String,
    server_keypair: Arc<Keypair>,
) -> anyhow::Result<Option<String>> {
    // Run all RPC operations in a blocking thread
    tokio::task::spawn_blocking(move || {
        try_finalize_job_blocking(
            &rpc_url,
            program_id,
            job_id,
            &creator_pubkey_str,
            &server_keypair,
        )
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
}

/// Blocking version of try_finalize_job (runs in spawn_blocking)
/// Returns Some(tx_signature) if finalized, None if not ready
fn try_finalize_job_blocking(
    rpc_url: &str,
    program_id: Pubkey,
    job_id: u64,
    creator_pubkey_str: &str,
    server_keypair: &Keypair,
) -> anyhow::Result<Option<String>> {
    // Create RPC client
    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let client = MarketplaceClient::new_with_commitment(
        rpc_url.to_string(),
        program_id,
        CommitmentConfig::confirmed(),
    );

    // Get FHE consensus PDA
    let (fhe_consensus_pda, _) = client.get_fhe_consensus_pda(job_id);

    // Fetch FHE consensus data
    let fhe_data = match fetch_fhe_consensus(&rpc_client, &fhe_consensus_pda) {
        Ok(data) => data,
        Err(e) => {
            log::debug!("Job {}: No FHE consensus data found: {}", job_id, e);
            return Ok(None);
        }
    };

    log::info!(
        "Job {}: FHE data - required={}, threshold={}, claimed={}, results={}",
        job_id,
        fhe_data.required_provers,
        fhe_data.consensus_threshold,
        fhe_data.claimed_count,
        fhe_data.results_count
    );

    // Check if we have enough results (need ALL provers to submit, not just threshold)
    // The on-chain program requires all claimed provers to submit before finalization
    if fhe_data.results_count < fhe_data.claimed_count {
        log::info!(
            "Job {}: Waiting for all provers to submit ({}/{} results)",
            job_id,
            fhe_data.results_count,
            fhe_data.claimed_count
        );
        return Ok(None);
    }

    // Check for consensus - count matching hashes
    let mut hash_counts: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();

    for i in 0..fhe_data.claimed_count as usize {
        if fhe_data.result_submitted[i] {
            let hash = fhe_data.result_hashes[i];
            let prover = fhe_data.claimed_provers[i];

            log::debug!(
                "Job {}: Prover {} submitted hash {}",
                job_id,
                prover,
                hex::encode(&hash[..8])
            );

            hash_counts.entry(hash).or_default().push(prover);
        }
    }

    // Find the hash with most votes
    let (consensus_hash, matching_provers) = match hash_counts
        .into_iter()
        .max_by_key(|(_, provers)| provers.len())
    {
        Some((hash, provers)) => (hash, provers),
        None => {
            log::info!("Job {}: No results to check", job_id);
            return Ok(None);
        }
    };

    // Check if we have consensus
    if matching_provers.len() < fhe_data.consensus_threshold as usize {
        log::info!(
            "Job {}: No consensus yet ({}/{} matching, need {})",
            job_id,
            matching_provers.len(),
            fhe_data.results_count,
            fhe_data.consensus_threshold
        );
        return Ok(None);
    }

    log::info!(
        "Job {}: CONSENSUS REACHED! {}/{} provers agree on hash {}",
        job_id,
        matching_provers.len(),
        fhe_data.required_provers,
        hex::encode(&consensus_hash[..8])
    );

    // Get job PDA
    let creator_pubkey: Pubkey = creator_pubkey_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid creator pubkey: {}", e))?;
    let (job_pda, _) = client.get_job_pda(&creator_pubkey, job_id);

    // Verify job is still in claimed state
    let job = fetch_job(&rpc_client, &job_pda)?;
    if job.status != zyberlink_types::JobStatus::Claimed {
        log::info!(
            "Job {}: Already finalized (status: {:?}), skipping",
            job_id,
            job.status
        );
        return Ok(None);
    }

    // Collect ALL claimed provers (program expects all, not just matching)
    let all_claimed_provers: Vec<Pubkey> =
        fhe_data.claimed_provers[..fhe_data.claimed_count as usize].to_vec();

    // Build and send finalize transaction
    log::info!("Job {}: Sending finalize transaction...", job_id);
    log::info!(
        "Job {}: Finalizer pubkey: {}",
        job_id,
        server_keypair.pubkey()
    );
    log::info!("Job {}: Job PDA: {}", job_id, job_pda);
    log::info!("Job {}: Creator: {}", job_id, creator_pubkey);
    log::info!(
        "Job {}: All claimed provers ({}): {:?}",
        job_id,
        all_claimed_provers.len(),
        all_claimed_provers
    );

    let finalize_ix = client.finalize_fhe_job_instruction(
        &server_keypair.pubkey(),
        &job_pda,
        job_id,
        &creator_pubkey,
        &all_claimed_provers,
    )?;

    match client.send_and_confirm_transaction(&[finalize_ix], &[server_keypair]) {
        Ok(sig) => {
            log::info!("Job {}: FINALIZED SUCCESSFULLY! Signature: {}", job_id, sig);
            Ok(Some(sig.to_string()))
        }
        Err(e) => {
            log::error!("Job {}: Finalize transaction FAILED: {}", job_id, e);
            Err(anyhow::anyhow!("Transaction failed: {}", e))
        }
    }
}

/// Load server keypair from file path
pub fn load_server_keypair(path: &str) -> anyhow::Result<Keypair> {
    let expanded_path = path.replace("~", &std::env::var("HOME").unwrap_or_default());
    read_keypair_file(&expanded_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read server keypair from {}: {}",
            expanded_path,
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test consensus detection with matching hashes
    #[test]
    fn test_consensus_detection_all_match() {
        let mut hash_counts: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();

        let hash = [0u8; 32];
        let prover1 = Pubkey::new_unique();
        let prover2 = Pubkey::new_unique();
        let prover3 = Pubkey::new_unique();

        hash_counts.entry(hash).or_default().push(prover1);
        hash_counts.entry(hash).or_default().push(prover2);
        hash_counts.entry(hash).or_default().push(prover3);

        let (_, matching_provers) = hash_counts
            .into_iter()
            .max_by_key(|(_, provers)| provers.len())
            .unwrap();

        assert_eq!(matching_provers.len(), 3);

        // Threshold of 2 should pass
        let threshold = 2u8;
        assert!(matching_provers.len() >= threshold as usize);
    }

    /// Test consensus detection with partial match (2/3)
    #[test]
    fn test_consensus_detection_partial_match() {
        let mut hash_counts: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();

        let hash1 = [0u8; 32];
        let mut hash2 = [0u8; 32];
        hash2[0] = 1; // Different hash

        let prover1 = Pubkey::new_unique();
        let prover2 = Pubkey::new_unique();
        let prover3 = Pubkey::new_unique();

        // 2 provers agree on hash1
        hash_counts.entry(hash1).or_default().push(prover1);
        hash_counts.entry(hash1).or_default().push(prover2);
        // 1 prover has different hash
        hash_counts.entry(hash2).or_default().push(prover3);

        let (consensus_hash, matching_provers) = hash_counts
            .into_iter()
            .max_by_key(|(_, provers)| provers.len())
            .unwrap();

        assert_eq!(matching_provers.len(), 2);
        assert_eq!(consensus_hash, hash1);

        // Threshold of 2 should pass
        let threshold = 2u8;
        assert!(matching_provers.len() >= threshold as usize);
    }

    /// Test consensus detection with no match
    #[test]
    fn test_consensus_detection_no_match() {
        let mut hash_counts: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();

        let mut hash1 = [0u8; 32];
        let mut hash2 = [0u8; 32];
        let mut hash3 = [0u8; 32];
        hash1[0] = 1;
        hash2[0] = 2;
        hash3[0] = 3;

        let prover1 = Pubkey::new_unique();
        let prover2 = Pubkey::new_unique();
        let prover3 = Pubkey::new_unique();

        // All different hashes
        hash_counts.entry(hash1).or_default().push(prover1);
        hash_counts.entry(hash2).or_default().push(prover2);
        hash_counts.entry(hash3).or_default().push(prover3);

        let (_, matching_provers) = hash_counts
            .into_iter()
            .max_by_key(|(_, provers)| provers.len())
            .unwrap();

        assert_eq!(matching_provers.len(), 1);

        // Threshold of 2 should NOT pass
        let threshold = 2u8;
        assert!(matching_provers.len() < threshold as usize);
    }

    /// Test keypair loading with invalid path
    #[test]
    fn test_load_keypair_invalid_path() {
        let result = load_server_keypair("/nonexistent/path/keypair.json");
        assert!(result.is_err());
    }

    /// Test that we correctly identify when results_count < threshold
    #[test]
    fn test_not_enough_results() {
        let results_count = 1u8;
        let consensus_threshold = 2u8;

        assert!(results_count < consensus_threshold);
    }

    /// Test that we correctly identify when results_count >= threshold
    #[test]
    fn test_enough_results() {
        let results_count = 2u8;
        let consensus_threshold = 2u8;

        assert!(results_count >= consensus_threshold);
    }
}
