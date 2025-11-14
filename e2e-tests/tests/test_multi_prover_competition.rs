//! Multi-Prover Competition Tests
//!
//! Tests race conditions and concurrent behavior when multiple provers
//! compete for the same jobs.

use anyhow::Result;
use blake2::{Blake2s256, Digest};
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::CircuitType;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Helper to register a prover
async fn register_prover(
    rpc_client: &RpcClient,
    sdk_client: &MarketplaceClient,
    prover: &Keypair,
    stake_amount: u64,
) -> Result<()> {
    // Check if already registered
    let (prover_pda, _) = sdk_client.get_prover_pda(&prover.pubkey());
    if rpc_client.get_account(&prover_pda).is_ok() {
        return Ok(()); // Already registered
    }

    // Fund prover
    let airdrop_sig = rpc_client.request_airdrop(&prover.pubkey(), 20_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Register
    let encryption_key = [99u8; 32];
    let register_ix = sdk_client.register_prover_instruction(
        &prover.pubkey(),
        stake_amount,
        encryption_key,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover.pubkey()));
    tx.sign(&[prover], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper to create a job
async fn create_job(
    rpc_client: &RpcClient,
    sdk_client: &MarketplaceClient,
    client: &Keypair,
) -> Result<(solana_sdk::pubkey::Pubkey, u64)> {
    // Fund client
    let airdrop_sig = rpc_client.request_airdrop(&client.pubkey(), 10_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Get next job ID
    let (config_pda, _) = sdk_client.get_config_pda();
    let config_account = rpc_client.get_account(&config_pda)?;

    use borsh::BorshDeserialize;

    #[derive(Debug, BorshDeserialize)]
    #[allow(dead_code)]
    struct MarketplaceConfigData {
        authority: solana_sdk::pubkey::Pubkey,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
        protocol_fee_recipient: solana_sdk::pubkey::Pubkey,
        next_job_id: u64,
        total_provers: u64,
        total_jobs_created: u64,
        total_jobs_completed: u64,
        is_paused: bool,
        bump: u8,
    }

    let config_data = MarketplaceConfigData::try_from_slice(&config_account.data)?;
    let job_id = config_data.next_job_id;

    // Create witness
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    let create_job_ix = sdk_client.create_job_instruction(
        &client.pubkey(),
        job_id,
        CircuitType::ZcashOrchard,
        witness_commitment,
        witness_data.len() as u32,
        2_000_000_000,
        600,
        None,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&client.pubkey()));
    tx.sign(&[client], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?;

    let (job_pda, _) = sdk_client.get_job_pda(&client.pubkey(), job_id);
    Ok((job_pda, job_id))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_two_provers_one_job_race() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Two Provers Competing for One Job (Race Condition)");
    println!("{}", "=".repeat(80));

    // Setup
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    println!("\nSetup: Register Two Provers");
    println!("{}", "-".repeat(80));

    let prover1 = Keypair::new();
    let prover2 = Keypair::new();

    register_prover(&rpc_client, &sdk_client, &prover1, 5_000_000_000).await?;
    register_prover(&rpc_client, &sdk_client, &prover2, 5_000_000_000).await?;

    println!("  ✓ Prover 1: {}", prover1.pubkey());
    println!("  ✓ Prover 2: {}", prover2.pubkey());

    println!("\nSetup: Create One Job");
    println!("{}", "-".repeat(80));

    let client = Keypair::new();
    let (job_pda, job_id) = create_job(&rpc_client, &sdk_client, &client).await?;

    println!("  ✓ Job created");
    println!("    Job ID: {}", job_id);
    println!("    Job PDA: {}", job_pda);

    println!("\nTest: Both Provers Attempt to Claim Simultaneously");
    println!("{}", "-".repeat(80));

    // Spawn both claim attempts concurrently
    let prover1_claim = {
        let rpc_url = rpc_url.to_string();
        let prover1 = prover1.insecure_clone();
        let job_pda = job_pda;

        tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);
            let claim_ix = sdk_client
                .claim_job_instruction(&prover1.pubkey(), &job_pda)
                .unwrap();

            let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash().unwrap();
            let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover1.pubkey()));
            tx.sign(&[&prover1], recent_blockhash);

            sdk_client.rpc_client.send_and_confirm_transaction(&tx)
        })
    };

    let prover2_claim = {
        let rpc_url = rpc_url.to_string();
        let prover2 = prover2.insecure_clone();
        let job_pda = job_pda;

        tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);
            let claim_ix = sdk_client
                .claim_job_instruction(&prover2.pubkey(), &job_pda)
                .unwrap();

            let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash().unwrap();
            let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover2.pubkey()));
            tx.sign(&[&prover2], recent_blockhash);

            sdk_client.rpc_client.send_and_confirm_transaction(&tx)
        })
    };

    // Wait for both attempts
    let result1 = prover1_claim.await;
    let result2 = prover2_claim.await;

    // Exactly ONE should succeed
    let success1 = result1.is_ok() && result1.unwrap().is_ok();
    let success2 = result2.is_ok() && result2.unwrap().is_ok();

    println!("  Prover 1 result: {}", if success1 { "SUCCESS" } else { "FAILED" });
    println!("  Prover 2 result: {}", if success2 { "SUCCESS" } else { "FAILED" });

    // Verify exactly one succeeded
    assert!(
        success1 ^ success2,
        "Exactly one prover should succeed (got success1={}, success2={})",
        success1,
        success2
    );

    println!("  ✓ Exactly one prover claimed the job (no race condition)");

    println!("\n{}", "=".repeat(80));
    println!("✅ RACE CONDITION TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_three_provers_three_jobs_concurrent() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Three Provers Processing Three Jobs Concurrently");
    println!("{}", "=".repeat(80));

    // Setup
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    println!("\nSetup: Register Three Provers");
    println!("{}", "-".repeat(80));

    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();

    register_prover(&rpc_client, &sdk_client, &prover1, 5_000_000_000).await?;
    register_prover(&rpc_client, &sdk_client, &prover2, 5_000_000_000).await?;
    register_prover(&rpc_client, &sdk_client, &prover3, 5_000_000_000).await?;

    println!("  ✓ Prover 1: {}", prover1.pubkey());
    println!("  ✓ Prover 2: {}", prover2.pubkey());
    println!("  ✓ Prover 3: {}", prover3.pubkey());

    println!("\nSetup: Create Three Jobs");
    println!("{}", "-".repeat(80));

    let client1 = Keypair::new();
    let client2 = Keypair::new();
    let client3 = Keypair::new();

    let (job1_pda, job1_id) = create_job(&rpc_client, &sdk_client, &client1).await?;

    // Small delay to ensure config updates between job creations
    sleep(Duration::from_millis(500)).await;

    let (job2_pda, job2_id) = create_job(&rpc_client, &sdk_client, &client2).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let (job3_pda, job3_id) = create_job(&rpc_client, &sdk_client, &client3).await?;

    println!("  ✓ Job 1: {} (PDA: {})", job1_id, job1_pda);
    println!("  ✓ Job 2: {} (PDA: {})", job2_id, job2_pda);
    println!("  ✓ Job 3: {} (PDA: {})", job3_id, job3_pda);

    println!("\nTest: Each Prover Claims a Different Job");
    println!("{}", "-".repeat(80));

    // Each prover claims a different job
    let mut claim_handles: Vec<JoinHandle<Result<()>>> = vec![];

    // Prover 1 -> Job 1
    claim_handles.push({
        let rpc_url = rpc_url.to_string();
        let prover = prover1.insecure_clone();
        let job_pda = job1_pda;

        tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);
            let claim_ix = sdk_client
                .claim_job_instruction(&prover.pubkey(), &job_pda)?;

            let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash()?;
            let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
            tx.sign(&[&prover], recent_blockhash);

            sdk_client.rpc_client.send_and_confirm_transaction(&tx)?;
            Ok(())
        })
    });

    // Prover 2 -> Job 2
    claim_handles.push({
        let rpc_url = rpc_url.to_string();
        let prover = prover2.insecure_clone();
        let job_pda = job2_pda;

        tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);
            let claim_ix = sdk_client
                .claim_job_instruction(&prover.pubkey(), &job_pda)?;

            let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash()?;
            let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
            tx.sign(&[&prover], recent_blockhash);

            sdk_client.rpc_client.send_and_confirm_transaction(&tx)?;
            Ok(())
        })
    });

    // Prover 3 -> Job 3
    claim_handles.push({
        let rpc_url = rpc_url.to_string();
        let prover = prover3.insecure_clone();
        let job_pda = job3_pda;

        tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);
            let claim_ix = sdk_client
                .claim_job_instruction(&prover.pubkey(), &job_pda)?;

            let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash()?;
            let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
            tx.sign(&[&prover], recent_blockhash);

            sdk_client.rpc_client.send_and_confirm_transaction(&tx)?;
            Ok(())
        })
    });

    // Wait for all claims
    let mut all_succeeded = true;
    for (i, handle) in claim_handles.into_iter().enumerate() {
        let result = handle.await;
        if result.is_ok() && result.unwrap().is_ok() {
            println!("  ✓ Prover {} claimed job successfully", i + 1);
        } else {
            println!("  ✗ Prover {} failed to claim", i + 1);
            all_succeeded = false;
        }
    }

    assert!(all_succeeded, "All provers should claim their respective jobs");

    println!("\n{}", "=".repeat(80));
    println!("✅ CONCURRENT PROCESSING TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_five_provers_two_jobs() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Five Provers Competing for Two Jobs");
    println!("{}", "=".repeat(80));

    // Setup
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    println!("\nSetup: Register Five Provers");
    println!("{}", "-".repeat(80));

    let mut provers = vec![];
    for i in 0..5 {
        let prover = Keypair::new();
        register_prover(&rpc_client, &sdk_client, &prover, 5_000_000_000).await?;
        println!("  ✓ Prover {}: {}", i + 1, prover.pubkey());
        provers.push(prover);
    }

    println!("\nSetup: Create Two Jobs");
    println!("{}", "-".repeat(80));

    let client1 = Keypair::new();
    let client2 = Keypair::new();

    let (job1_pda, job1_id) = create_job(&rpc_client, &sdk_client, &client1).await?;

    // Small delay to ensure config updates between job creations
    sleep(Duration::from_millis(500)).await;

    let (job2_pda, job2_id) = create_job(&rpc_client, &sdk_client, &client2).await?;

    println!("  ✓ Job 1: {} (PDA: {})", job1_id, job1_pda);
    println!("  ✓ Job 2: {} (PDA: {})", job2_id, job2_pda);

    println!("\nTest: All Five Provers Attempt to Claim Jobs");
    println!("{}", "-".repeat(80));

    // All provers try to claim job 1, then job 2
    let mut claim_handles: Vec<JoinHandle<(usize, bool, bool)>> = vec![];

    for (i, prover) in provers.into_iter().enumerate() {
        let rpc_url = rpc_url.to_string();
        let job1_pda = job1_pda;
        let job2_pda = job2_pda;

        claim_handles.push(tokio::spawn(async move {
            let sdk_client = MarketplaceClient::new(rpc_url, program_id);

            // Try to claim job 1
            let claim1_result = {
                let claim_ix = sdk_client
                    .claim_job_instruction(&prover.pubkey(), &job1_pda)
                    .unwrap();

                let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash().unwrap();
                let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
                tx.sign(&[&prover], recent_blockhash);

                sdk_client.rpc_client.send_and_confirm_transaction(&tx).is_ok()
            };

            // Try to claim job 2
            let claim2_result = {
                let claim_ix = sdk_client
                    .claim_job_instruction(&prover.pubkey(), &job2_pda)
                    .unwrap();

                let recent_blockhash = sdk_client.rpc_client.get_latest_blockhash().unwrap();
                let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
                tx.sign(&[&prover], recent_blockhash);

                sdk_client.rpc_client.send_and_confirm_transaction(&tx).is_ok()
            };

            (i, claim1_result, claim2_result)
        }));
    }

    // Collect results
    let mut total_successful_claims = 0;
    for handle in claim_handles {
        let (prover_idx, job1_claimed, job2_claimed) = handle.await.unwrap();

        let mut status = String::new();
        if job1_claimed {
            status.push_str("claimed job 1");
            total_successful_claims += 1;
        }
        if job2_claimed {
            if !status.is_empty() {
                status.push_str(" and ");
            }
            status.push_str("claimed job 2");
            total_successful_claims += 1;
        }
        if status.is_empty() {
            status = "failed to claim any job".to_string();
        }

        println!("  Prover {}: {}", prover_idx + 1, status);
    }

    // Verify exactly 2 claims succeeded (one per job)
    assert_eq!(
        total_successful_claims, 2,
        "Exactly 2 jobs should be claimed (got {})",
        total_successful_claims
    );

    println!("  ✓ Exactly 2 provers claimed jobs (3 remained idle)");

    println!("\n{}", "=".repeat(80));
    println!("✅ MORE PROVERS THAN JOBS TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
