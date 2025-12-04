mod common;

use common::{create_test_witness, TestBackend};
use prover_node::witness_encryption::WitnessEncryption;
use std::sync::Arc;

/// Test 1: Upload/Download Roundtrip
/// Verifies complete HTTP workflow: upload witness, get commitment, download by commitment
#[tokio::test]
async fn test_backend_upload_download() {
    // Start backend server on random port
    let backend = TestBackend::start().await.unwrap();

    let witness = create_test_witness();
    let encryption = WitnessEncryption::new().unwrap();
    let encrypted = WitnessEncryption::encrypt_witness(
        &witness,
        &encryption.public_key()
    ).unwrap();

    // Upload witness
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/witness", backend.url()))
        .header("Content-Type", "application/octet-stream")
        .body(encrypted.clone())
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload failed: {}", response.status());

    let json: serde_json::Value = response.json().await.unwrap();
    let commitment_hex = json["commitment"].as_str().unwrap();

    assert_eq!(commitment_hex.len(), 64, "Commitment should be 32 bytes in hex (64 chars)");

    // Download witness by commitment
    let response = client
        .get(format!("{}/witness/{}", backend.url(), commitment_hex))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Download failed: {}", response.status());

    let downloaded = response.bytes().await.unwrap().to_vec();
    assert_eq!(encrypted, downloaded, "Downloaded witness doesn't match uploaded");

    // Cleanup
    backend.shutdown().await.unwrap();
}

/// Test 2: Commitment Correctness
/// Verifies that the commitment is calculated correctly using Blake2b
#[tokio::test]
async fn test_commitment_calculation() {
    let backend = TestBackend::start().await.unwrap();

    let data = vec![1, 2, 3, 4, 5];

    // Calculate expected commitment (Blake2b-512, first 32 bytes)
    use blake2::{Blake2b512, Digest};
    let mut hasher = Blake2b512::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    let expected_commitment = hex::encode(&hash[..32]);

    // Upload data
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/witness", backend.url()))
        .body(data)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let json: serde_json::Value = response.json().await.unwrap();
    let actual_commitment = json["commitment"].as_str().unwrap();

    assert_eq!(
        expected_commitment, actual_commitment,
        "Commitment calculation mismatch"
    );

    backend.shutdown().await.unwrap();
}

/// Test 3: Not Found Error
/// Verifies that requesting a non-existent witness returns 404
#[tokio::test]
async fn test_download_nonexistent_witness() {
    let backend = TestBackend::start().await.unwrap();

    let fake_commitment = "a".repeat(64); // Valid hex format, but doesn't exist

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/witness/{}", backend.url(), fake_commitment))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404, "Should return 404 for non-existent witness");

    backend.shutdown().await.unwrap();
}

/// Test 4: Health Check
/// Verifies that the health endpoint returns correct status
#[tokio::test]
async fn test_health_endpoint() {
    let backend = TestBackend::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", backend.url()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Health check failed");

    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "ok", "Health status should be 'ok'");
    assert!(json["witnesses_stored"].is_number(), "Should report witnesses count");
    assert!(json["total_bytes"].is_number(), "Should report total bytes");

    backend.shutdown().await.unwrap();
}

/// Test 5: Concurrent Access
/// Verifies that the backend can handle concurrent uploads and downloads
#[tokio::test]
async fn test_concurrent_uploads_downloads() {
    let backend = TestBackend::start().await.unwrap();
    let client = Arc::new(reqwest::Client::new());

    let mut handles = vec![];

    // Spawn 10 concurrent tasks that upload and download
    for i in 0..10 {
        let client = client.clone();
        let url = backend.url().to_string();

        let handle = tokio::spawn(async move {
            // Create unique data per task
            let data = vec![i; 100];

            // Upload
            let response = client
                .post(format!("{}/witness", url))
                .body(data.clone())
                .send()
                .await
                .unwrap();

            assert!(response.status().is_success(), "Task {} upload failed", i);

            let json: serde_json::Value = response.json().await.unwrap();
            let commitment = json["commitment"].as_str().unwrap().to_string();

            // Download immediately
            let response = client
                .get(format!("{}/witness/{}", url, commitment))
                .send()
                .await
                .unwrap();

            assert!(response.status().is_success(), "Task {} download failed", i);

            let downloaded = response.bytes().await.unwrap().to_vec();
            assert_eq!(data, downloaded, "Task {} data mismatch", i);
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    backend.shutdown().await.unwrap();
}

/// Test 6: Invalid Commitment Format
/// Verifies that invalid commitment formats are rejected
#[tokio::test]
async fn test_invalid_commitment_format() {
    let backend = TestBackend::start().await.unwrap();

    let commitment_63 = "a".repeat(63);
    let commitment_65 = "a".repeat(65);

    let invalid_commitments = vec![
        "too_short",               // Too short
        "not_hex_!@#$",           // Non-hex characters
        commitment_63.as_str(),   // 63 chars (need 64)
        commitment_65.as_str(),   // 65 chars (need 64)
    ];

    let client = reqwest::Client::new();

    for commitment in invalid_commitments {
        let response = client
            .get(format!("{}/witness/{}", backend.url(), commitment))
            .send()
            .await
            .unwrap();

        assert!(
            response.status().is_client_error(),
            "Invalid commitment '{}' should return 4xx error, got {}",
            commitment,
            response.status()
        );
    }

    backend.shutdown().await.unwrap();
}

/// Test 7: Large Witness Upload
/// Verifies that reasonably large witnesses can be uploaded
#[tokio::test]
async fn test_large_witness_upload() {
    let backend = TestBackend::start().await.unwrap();

    // Create a 1MB witness (within 10MB limit)
    let large_data = vec![0u8; 1_000_000];

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/witness", backend.url()))
        .body(large_data.clone())
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Large upload failed");

    let json: serde_json::Value = response.json().await.unwrap();
    let commitment = json["commitment"].as_str().unwrap();

    // Verify we can download it back
    let response = client
        .get(format!("{}/witness/{}", backend.url(), commitment))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let downloaded = response.bytes().await.unwrap().to_vec();
    assert_eq!(large_data.len(), downloaded.len());

    backend.shutdown().await.unwrap();
}
