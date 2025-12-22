//! Integration tests for Starknet marketplace
//!
//! These tests require a running starknet-devnet-rs instance on localhost:5050
//! with the PbtcfiJobs contract deployed.
//!
//! Run with: cargo test --test starknet_integration_test -- --ignored

use zyberlink_chain_client::StarknetClient;
use prover_node::marketplace::{MarketplaceFactory, MarketplaceOperations, StarknetMarketplace};
use std::sync::Arc;

/// Devnet configuration (from session logs)
const DEVNET_RPC_URL: &str = "http://localhost:5050";
const CONTRACT_ADDRESS: &str = "0x0587f1e2a4494f7d72ea0811a4c20185f2e2b12e05b6d8bc8948847e2cf709bd";
const PROVER_ADDRESS: &str = "0x0557ba9ef60b52dad611d79b60563901458f2476a5c1002a8b4869fcb6654c7e";
const PRIVATE_KEY: &str = "0x0000000000000000000000000000000015b5e3013d752c909988204714f1ff35";

/// Test: Create marketplace with factory
#[test]
#[ignore] // Requires devnet running
fn test_create_starknet_marketplace_with_factory() {
    let marketplace = MarketplaceFactory::create_starknet(
        DEVNET_RPC_URL,
        CONTRACT_ADDRESS,
        PROVER_ADDRESS,
        Some(PRIVATE_KEY),
    ).expect("Failed to create marketplace");

    assert_eq!(marketplace.chain_id(), "starknet");
    assert_eq!(marketplace.program_address(), CONTRACT_ADDRESS);
    assert_eq!(marketplace.prover_address(), PROVER_ADDRESS);
    assert!(marketplace.can_sign());
}

/// Test: Read pending jobs from devnet
#[tokio::test]
#[ignore] // Requires devnet running
async fn test_find_pending_jobs() {
    let client = StarknetClient::new(DEVNET_RPC_URL)
        .expect("Failed to create client");

    let marketplace = StarknetMarketplace::new(
        Arc::new(client),
        CONTRACT_ADDRESS.to_string(),
        PROVER_ADDRESS.to_string(),
    );

    let jobs = marketplace.find_pending_jobs().await;
    println!("find_pending_jobs result: {:?}", jobs);

    // Should not error
    assert!(jobs.is_ok(), "Failed to find pending jobs: {:?}", jobs.err());
}

/// Test: Get specific job details
#[tokio::test]
#[ignore] // Requires devnet running with job #3 pending
async fn test_get_job() {
    let client = StarknetClient::new(DEVNET_RPC_URL)
        .expect("Failed to create client");

    let marketplace = StarknetMarketplace::new(
        Arc::new(client),
        CONTRACT_ADDRESS.to_string(),
        PROVER_ADDRESS.to_string(),
    );

    let job = marketplace.get_job(3, CONTRACT_ADDRESS).await;
    println!("get_job(3) result: {:?}", job);

    assert!(job.is_ok(), "Failed to get job: {:?}", job.err());
}

/// Test: Full flow - claim and submit (requires job #3 pending)
#[tokio::test]
#[ignore] // Requires devnet running with job #3 pending
async fn test_claim_and_submit() {
    let marketplace = MarketplaceFactory::create_starknet(
        DEVNET_RPC_URL,
        CONTRACT_ADDRESS,
        PROVER_ADDRESS,
        Some(PRIVATE_KEY),
    ).expect("Failed to create marketplace");

    // 1. Find pending jobs
    let jobs = marketplace.find_pending_jobs().await
        .expect("Failed to find pending jobs");

    if jobs.is_empty() {
        println!("No pending jobs found - create one first with sncast");
        return;
    }

    let job_id = jobs[0].id;
    println!("Found pending job: {}", job_id);

    // 2. Claim the job
    let claim_result = marketplace.claim_job(job_id, CONTRACT_ADDRESS).await;
    println!("claim_job({}) result: {:?}", job_id, claim_result);

    assert!(claim_result.is_ok(), "Failed to claim job: {:?}", claim_result.err());
    assert!(claim_result.unwrap().success);

    // 3. Submit result
    let result_hash = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00,
                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

    let submit_result = marketplace.submit_proof(job_id, CONTRACT_ADDRESS, result_hash, 0, None).await;
    println!("submit_proof({}) result: {:?}", job_id, submit_result);

    assert!(submit_result.is_ok(), "Failed to submit proof: {:?}", submit_result.err());
    assert!(submit_result.unwrap().success);

    // 4. Verify job is completed
    let final_job = marketplace.get_job(job_id, CONTRACT_ADDRESS).await
        .expect("Failed to get job");

    if let Some(job) = final_job {
        println!("Job {} final status: {:?}", job_id, job.status);
        assert_eq!(job.status, zyberlink_types::JobStatus::Completed);
    }
}
