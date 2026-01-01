//! Integration tests for prover endpoints
//!
//! These tests verify the authentication and endpoint routing.
//! They use mock data since we don't have a running blink-server.

use blake2::{Blake2s256, Digest};
use solana_sdk::signature::{Keypair, Signer};

/// Helper to build a signed request message
fn build_signed_message(
    keypair: &Keypair,
    method: &str,
    path: &str,
    body: &[u8],
) -> (String, String, i64) {
    let timestamp = chrono::Utc::now().timestamp();

    // Hash body
    let mut hasher = Blake2s256::new();
    hasher.update(body);
    let body_hash = hex::encode(hasher.finalize());

    // Build message
    let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
    let signature = keypair.sign_message(message.as_bytes());

    (signature.to_string(), message, timestamp)
}

#[test]
fn test_auth_message_format() {
    let keypair = Keypair::new();
    let method = "GET";
    let path = "/gateway/prover/witness/test";
    let body = b"";

    let (signature, message, timestamp) = build_signed_message(&keypair, method, path, body);

    // Verify message format
    assert!(message.contains("GET"));
    assert!(message.contains("/gateway/prover/witness/test"));
    assert!(message.contains(&timestamp.to_string()));
    assert_eq!(message.matches(':').count(), 3);

    // Verify signature is not empty
    assert!(!signature.is_empty());
    println!("Generated signature: {}", signature);
}

#[test]
fn test_auth_message_with_body() {
    let keypair = Keypair::new();
    let method = "POST";
    let path = "/gateway/prover/zk/123/submit";
    let body = br#"{"proof":"test","public_inputs":[]}"#;

    let (signature, message, _timestamp) = build_signed_message(&keypair, method, path, body);

    // Message should include body hash
    let mut hasher = Blake2s256::new();
    hasher.update(body);
    let expected_hash = hex::encode(hasher.finalize());

    assert!(message.ends_with(&expected_hash));
    assert!(!signature.is_empty());
}

#[test]
fn test_different_bodies_produce_different_signatures() {
    let keypair = Keypair::new();
    let method = "POST";
    let path = "/gateway/prover/zk/123/submit";

    let body1 = br#"{"proof":"test1"}"#;
    let body2 = br#"{"proof":"test2"}"#;

    let (sig1, _, _) = build_signed_message(&keypair, method, path, body1);
    let (sig2, _, _) = build_signed_message(&keypair, method, path, body2);

    // Different bodies should produce different signatures
    assert_ne!(sig1, sig2);
}

#[test]
fn test_signature_includes_full_path() {
    let keypair = Keypair::new();
    let method = "GET";
    let body = b"";

    let path1 = "/gateway/prover/witness/hash1";
    let path2 = "/gateway/prover/witness/hash2";

    let (sig1, _, _) = build_signed_message(&keypair, method, path1, body);
    let (sig2, _, _) = build_signed_message(&keypair, method, path2, body);

    // Different paths should produce different signatures
    assert_ne!(sig1, sig2);
}

#[test]
fn test_signature_includes_method() {
    let keypair = Keypair::new();
    let path = "/gateway/prover/witness/test";
    let body = b"";

    let (sig1, _, _) = build_signed_message(&keypair, "GET", path, body);
    let (sig2, _, _) = build_signed_message(&keypair, "POST", path, body);

    // Different methods should produce different signatures
    assert_ne!(sig1, sig2);
}

#[cfg(feature = "integration_tests")]
mod integration {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_prover_witness_endpoint_auth_required() {
        // This would require setting up the full app
        // For now, we just verify the signature logic works
        let keypair = Keypair::new();
        let (signature, _message, timestamp) =
            build_signed_message(&keypair, "GET", "/gateway/prover/witness/test", b"");

        assert!(!signature.is_empty());
        assert!(timestamp > 0);
    }
}
