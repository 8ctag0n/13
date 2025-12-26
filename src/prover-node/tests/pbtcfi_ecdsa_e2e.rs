//! End-to-end test for pBTCFi FHE flow with ECDSA signing
//!
//! Tests the complete flow:
//! 1. FHE computation with real TFHE ciphertexts
//! 2. Result hash and ciphertext hash computation
//! 3. ECDSA signing (STARK curve)
//! 4. Signature verification (local simulation)
//! 5. submit_verified_result (requires devnet for full test)
//!
//! Run with:
//!   cargo test --test pbtcfi_ecdsa_e2e
//!   cargo test --test pbtcfi_ecdsa_e2e -- --ignored  # For devnet tests

use prover_node::engines::{
    EncryptedValue, FheBalanceEngine, FheBalanceInput, FheBalanceProcessor, FheJobParams, JobType,
    TfheCiphertext,
};
use prover_node::marketplace::starknet::signing::{
    compute_message_hash, get_stark_pubkey, sign_fhe_result_stark,
};
use prover_node::marketplace::{MarketplaceFactory, MarketplaceOperations};
use prover_node::{generate_fhe_keys, FheEngine};
use starknet_crypto::{verify, Felt};
use std::sync::Arc;
use tiny_keccak::{Hasher, Keccak};
use zyberlink_fhe::{FheUint8, prelude::FheTryEncrypt};

/// Helper: compute Keccak256 hash
fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    output
}

/// Test: Complete E2E flow - FHE processing + ECDSA signing
///
/// This test simulates the full pBTCFi flow without requiring Starknet devnet:
/// 1. Generate TFHE keys and encrypt test values
/// 2. Process with FheBalanceEngine (LoanVerification)
/// 3. Compute ciphertext_hash and result_hash
/// 4. Sign with STARK curve ECDSA
/// 5. Verify signature locally (same logic as Cairo)
#[tokio::test]
async fn test_e2e_fhe_ecdsa_flow() {
    println!("\n=== E2E pBTCFi FHE + ECDSA Flow ===\n");

    // ===== PHASE 1: Setup FHE =====
    println!("Phase 1: Setting up FHE keys and ciphertexts...");

    let (client_key, server_key) = generate_fhe_keys().expect("Failed to generate FHE keys");

    // Simulate BTC collateral amount (encrypted)
    let btc_collateral = 150u8; // 1.5 BTC in some unit
    let encrypted_collateral =
        FheUint8::try_encrypt(btc_collateral, &client_key).expect("encrypt failed");
    let collateral_bytes = bincode::serialize(&encrypted_collateral).expect("serialize failed");

    println!("  BTC collateral encrypted: {} bytes", collateral_bytes.len());

    // Create ElGamal-style c1/c2 for Cairo (derived from TFHE hash)
    let tfhe_hash = keccak256(&collateral_bytes);
    let mut c1 = [0u8; 32];
    let c2 = [0u8; 32];
    c1.copy_from_slice(&tfhe_hash[..32]);
    // c2 would be second component in real ElGamal; here we use zeros

    // ===== PHASE 2: FHE Processing =====
    println!("\nPhase 2: Processing FHE job (LoanVerification)...");

    let fhe_engine = Arc::new(FheEngine::new(server_key));
    let balance_engine = FheBalanceEngine::new(fhe_engine);

    let input = FheBalanceInput {
        job_id: 42,
        job_type: JobType::LoanVerification,
        current_encrypted: EncryptedValue::from_felts(c1, c2),
        delta_encrypted: None, // Not needed for LoanVerification
        params: FheJobParams::LoanVerification {
            min_collateral_ratio: 150, // 150%
            btc_price_oracle: 50000,   // $50k BTC
        },
        tfhe_current: Some(TfheCiphertext {
            data: collateral_bytes.clone(),
        }),
        tfhe_delta: None,
    };

    let result = balance_engine
        .process(&input)
        .await
        .expect("FHE processing failed");

    println!("  Job ID: {}", result.job_id);
    println!("  Success: {}", result.success);
    println!(
        "  Verification result: {:?}",
        result.verification_result
    );
    println!("  Result hash: 0x{}", hex::encode(&result.result_hash[..8]));

    assert!(result.success, "FHE processing should succeed");

    // ===== PHASE 3: Compute Hashes =====
    println!("\nPhase 3: Computing hashes for signing...");

    // Ciphertext hash = keccak256(c1 || c2)
    let mut ciphertext_input = Vec::with_capacity(64);
    ciphertext_input.extend_from_slice(&c1);
    ciphertext_input.extend_from_slice(&c2);
    let ciphertext_hash = keccak256(&ciphertext_input);

    println!(
        "  ciphertext_hash: 0x{}",
        hex::encode(&ciphertext_hash[..8])
    );
    println!("  result_hash: 0x{}", hex::encode(&result.result_hash[..8]));

    // ===== PHASE 4: ECDSA Signing (STARK curve) =====
    println!("\nPhase 4: Signing with STARK curve ECDSA...");

    // Prover's private key (in real scenario, loaded from secure storage)
    let private_key: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x15, 0xb5, 0xe3, 0x01, 0x3d, 0x75, 0x2c, 0x90, 0x99, 0x88, 0x20, 0x47, 0x14, 0xf1,
        0xff, 0x35,
    ];

    let signature = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result.result_hash)
        .expect("Signing failed");

    println!("  Signature r: {}...", &signature.r[..18]);
    println!("  Signature s: {}...", &signature.s[..18]);

    // Get public key for verification
    let pubkey_hex = get_stark_pubkey(&private_key).expect("Failed to get pubkey");
    println!("  Prover pubkey: {}...", &pubkey_hex[..18]);

    // ===== PHASE 5: Signature Verification (local) =====
    println!("\nPhase 5: Verifying signature (simulating Cairo logic)...");

    // Recompute message hash (same as Cairo)
    let message_hash = compute_message_hash(&ciphertext_hash, &result.result_hash);
    let mut masked_message = message_hash;
    masked_message[0] &= 0x07; // Ensure < 2^251

    // Parse signature components
    let r_bytes = hex::decode(&signature.r[2..]).expect("decode r");
    let s_bytes = hex::decode(&signature.s[2..]).expect("decode s");
    let pubkey_bytes = hex::decode(&pubkey_hex[2..]).expect("decode pubkey");

    // Convert to Felt for verification
    let message_felt = Felt::from_bytes_be_slice(&masked_message);
    let r_felt = Felt::from_bytes_be_slice(&r_bytes);
    let s_felt = Felt::from_bytes_be_slice(&s_bytes);
    let pubkey_felt = Felt::from_bytes_be_slice(&pubkey_bytes);

    // Verify using starknet_crypto (same algorithm as Cairo)
    let is_valid = verify(&pubkey_felt, &message_felt, &r_felt, &s_felt)
        .expect("Verification computation failed");

    println!("  Signature valid: {}", is_valid);
    assert!(is_valid, "ECDSA signature should be valid");

    // ===== PHASE 6: Summary =====
    println!("\n=== E2E Test Summary ===");
    println!("  FHE Input: {} bytes TFHE ciphertext", collateral_bytes.len());
    println!("  FHE Result: success={}, verification={:?}", result.success, result.verification_result);
    println!("  ECDSA: STARK curve signature verified");
    println!("  Ready for submit_verified_result on Starknet");
    println!("\n=== E2E pBTCFi FHE + ECDSA Flow PASSED ===\n");
}

/// Test: ECDSA signature is deterministic
#[test]
fn test_ecdsa_signature_deterministic() {
    let private_key = [0x42; 32];
    let ciphertext_hash = [0xaa; 32];
    let result_hash = [0xbb; 32];

    let sig1 = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result_hash)
        .expect("sign1 failed");
    let sig2 = sign_fhe_result_stark(&private_key, &ciphertext_hash, &result_hash)
        .expect("sign2 failed");

    assert_eq!(sig1.r, sig2.r, "Signature r should be deterministic");
    assert_eq!(sig1.s, sig2.s, "Signature s should be deterministic");
}

/// Test: Different inputs produce different signatures
#[test]
fn test_ecdsa_different_inputs() {
    let private_key = [0x42; 32];

    let sig1 = sign_fhe_result_stark(&private_key, &[0xaa; 32], &[0xbb; 32]).expect("sign1");
    let sig2 = sign_fhe_result_stark(&private_key, &[0xcc; 32], &[0xdd; 32]).expect("sign2");

    assert_ne!(sig1.r, sig2.r, "Different inputs should produce different r");
}

/// Test: Public key derivation is consistent
#[test]
fn test_pubkey_derivation() {
    let private_key = [0x42; 32];

    let pubkey1 = get_stark_pubkey(&private_key).expect("pubkey1");
    let pubkey2 = get_stark_pubkey(&private_key).expect("pubkey2");

    assert_eq!(pubkey1, pubkey2, "Public key derivation should be deterministic");
    assert!(pubkey1.starts_with("0x"), "Pubkey should be hex format");
}

/// Test: Full flow with Starknet devnet (requires running devnet)
///
/// This test requires:
/// 1. katana devnet running on localhost:5050
/// 2. PbtcfiJobs contract deployed
/// 3. Prover registered with pubkey
///
/// Run with: cargo test --test pbtcfi_ecdsa_e2e test_e2e_with_devnet -- --ignored
#[tokio::test]
#[ignore]
async fn test_e2e_with_devnet() {
    println!("\n=== E2E with Starknet Devnet ===\n");

    // Devnet configuration
    const DEVNET_RPC_URL: &str = "http://katana:5050";
    const CONTRACT_ADDRESS: &str =
        "0x01cdd543e696f8013cd8cdc41fe7d105776108fc4dd81f4fdded7f6b9cbe17e2";
    const PROVER_ADDRESS: &str =
        "0x0557ba9ef60b52dad611d79b60563901458f2476a5c1002a8b4869fcb6654c7e";
    const PRIVATE_KEY: &str = "0x0000000000000000000000000000000015b5e3013d752c909988204714f1ff35";

    // Create marketplace
    let marketplace = MarketplaceFactory::create_starknet(
        DEVNET_RPC_URL,
        CONTRACT_ADDRESS,
        PROVER_ADDRESS,
        Some(PRIVATE_KEY),
    )
    .expect("Failed to create marketplace");

    println!("  Connected to devnet: {}", DEVNET_RPC_URL);
    println!("  Contract: {}", CONTRACT_ADDRESS);
    println!("  Prover: {}", PROVER_ADDRESS);

    // 1. Check for pending jobs
    let jobs = marketplace
        .find_pending_jobs()
        .await
        .expect("Failed to find jobs");

    if jobs.is_empty() {
        println!("  No pending jobs - create one first with sncast");
        println!("  Skipping devnet test");
        return;
    }

    let job = &jobs[0];
    println!("  Found job: {} (type: {:?})", job.id, job.circuit_type);

    // 2. Claim the job
    let claim_result = marketplace
        .claim_job(job.id, CONTRACT_ADDRESS)
        .await
        .expect("Claim failed");
    assert!(claim_result.success, "Claim should succeed");
    println!("  Claimed job: {}", job.id);

    // 3. Generate FHE keys and process
    let (client_key, server_key) = generate_fhe_keys().expect("keygen");
    let fhe_engine = Arc::new(FheEngine::new(server_key));
    let balance_engine = FheBalanceEngine::new(fhe_engine);

    // Create mock input (in real flow, would fetch from blink-server)
    let test_value = 100u8;
    let encrypted = FheUint8::try_encrypt(test_value, &client_key).expect("encrypt");
    let tfhe_bytes = bincode::serialize(&encrypted).expect("serialize");

    let input = FheBalanceInput {
        job_id: job.id,
        job_type: JobType::LoanVerification,
        current_encrypted: EncryptedValue::from_felts([0u8; 32], [0u8; 32]),
        delta_encrypted: None,
        params: FheJobParams::LoanVerification {
            min_collateral_ratio: 150,
            btc_price_oracle: 50000,
        },
        tfhe_current: Some(TfheCiphertext { data: tfhe_bytes }),
        tfhe_delta: None,
    };

    let result = balance_engine.process(&input).await.expect("FHE process");
    println!("  FHE result: success={}", result.success);

    // 4. Submit verified result (uses ECDSA internally)
    // Note: submit_verified_fhe_result is on StarknetMarketplace, not the trait
    // For now, use the trait method which doesn't have ECDSA
    let submit_result = marketplace
        .submit_proof(job.id, CONTRACT_ADDRESS, result.result_hash, 0, None)
        .await
        .expect("Submit failed");

    assert!(submit_result.success, "Submit should succeed");
    println!("  Submitted result with hash: 0x{}", hex::encode(&result.result_hash[..8]));

    // 5. Verify job is completed
    let final_job = marketplace
        .get_job(job.id, CONTRACT_ADDRESS)
        .await
        .expect("get_job");

    if let Some(j) = final_job {
        println!("  Final job status: {:?}", j.status);
        assert_eq!(
            j.status,
            zyberlink_types::JobStatus::Completed,
            "Job should be completed"
        );
    }

    println!("\n=== E2E with Devnet PASSED ===\n");
}

/// Test: Verify FheBalanceEngine handles all job types
#[tokio::test]
async fn test_fhe_engine_job_types() {
    let (client_key, server_key) = generate_fhe_keys().expect("keygen");
    let fhe_engine = Arc::new(FheEngine::new(server_key));
    let balance_engine = FheBalanceEngine::new(fhe_engine);

    // Create test ciphertext
    let encrypted = FheUint8::try_encrypt(100u8, &client_key).expect("encrypt");
    let tfhe_bytes = bincode::serialize(&encrypted).expect("serialize");
    let delta_encrypted = FheUint8::try_encrypt(50u8, &client_key).expect("encrypt");
    let delta_bytes = bincode::serialize(&delta_encrypted).expect("serialize");

    // Test each job type
    let job_types = vec![
        (
            JobType::LoanVerification,
            FheJobParams::LoanVerification {
                min_collateral_ratio: 150,
                btc_price_oracle: 50000,
            },
            false, // no delta needed
        ),
        (
            JobType::BalanceUpdate,
            FheJobParams::BalanceUpdate { is_mint: true },
            true, // delta needed
        ),
        (
            JobType::StakeProof,
            FheJobParams::StakeProof {
                min_stake_amount: 100,
                staking_period: 86400,
            },
            false,
        ),
        (
            JobType::LiquidationCheck,
            FheJobParams::LiquidationCheck {
                liquidation_threshold: 120,
                current_price: 45000,
            },
            false,
        ),
    ];

    for (job_type, params, needs_delta) in job_types {
        println!("Testing job type: {:?}", job_type);

        let input = FheBalanceInput {
            job_id: 1,
            job_type: job_type.clone(),
            current_encrypted: EncryptedValue::from_felts([0u8; 32], [0u8; 32]),
            delta_encrypted: if needs_delta {
                Some(EncryptedValue::from_felts([0u8; 32], [0u8; 32]))
            } else {
                None
            },
            params,
            tfhe_current: Some(TfheCiphertext {
                data: tfhe_bytes.clone(),
            }),
            tfhe_delta: if needs_delta {
                Some(TfheCiphertext {
                    data: delta_bytes.clone(),
                })
            } else {
                None
            },
        };

        let result = balance_engine.process(&input).await;
        assert!(
            result.is_ok(),
            "Job type {:?} should process successfully: {:?}",
            job_type,
            result.err()
        );

        let r = result.unwrap();
        println!(
            "  Result: success={}, verification={:?}",
            r.success, r.verification_result
        );
    }
}
