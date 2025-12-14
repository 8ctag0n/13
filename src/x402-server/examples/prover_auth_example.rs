//! Example: How a prover authenticates requests to x402-server
//!
//! This example shows how to:
//! 1. Build a signed request message
//! 2. Sign it with a Solana keypair
//! 3. Make authenticated requests to prover endpoints

use blake2::{Blake2s256, Digest};
use solana_sdk::signature::{Keypair, Signer};

fn main() {
    println!("=== Prover Authentication Example ===\n");

    // Generate a keypair (in production, load from file)
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();

    println!("Prover Pubkey: {}\n", pubkey);

    // Example 1: Download witness
    {
        println!("--- Example 1: GET /gateway/prover/witness/abc123 ---");

        let method = "GET";
        let path = "/gateway/prover/witness/abc123";
        let timestamp = chrono::Utc::now().timestamp();
        let body = b""; // GET has no body

        // Hash the body
        let mut hasher = Blake2s256::new();
        hasher.update(body);
        let body_hash = hex::encode(hasher.finalize());

        // Build message to sign
        let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
        println!("Message to sign: {}", message);

        // Sign the message
        let signature = keypair.sign_message(message.as_bytes());
        println!("Signature: {}\n", signature);

        println!("HTTP Headers:");
        println!("  X-Prover-Pubkey: {}", pubkey);
        println!("  X-Prover-Signature: {}", signature);
        println!("  X-Timestamp: {}", timestamp);
        println!();
    }

    // Example 2: Submit ZK proof
    {
        println!("--- Example 2: POST /gateway/prover/zk/123/submit ---");

        let method = "POST";
        let path = "/gateway/prover/zk/123/submit";
        let timestamp = chrono::Utc::now().timestamp();
        let body = br#"{"proof":"base64encodedproof","public_inputs":["0x1","0x2"]}"#;

        // Hash the body
        let mut hasher = Blake2s256::new();
        hasher.update(body);
        let body_hash = hex::encode(hasher.finalize());

        // Build message to sign
        let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
        println!("Message to sign: {}", message);

        // Sign the message
        let signature = keypair.sign_message(message.as_bytes());
        println!("Signature: {}\n", signature);

        println!("HTTP Headers:");
        println!("  X-Prover-Pubkey: {}", pubkey);
        println!("  X-Prover-Signature: {}", signature);
        println!("  X-Timestamp: {}", timestamp);
        println!("\nBody:");
        println!("  {}", String::from_utf8_lossy(body));
        println!();
    }

    // Example 3: Submit FHE result
    {
        println!("--- Example 3: POST /gateway/prover/fhe/456/submit ---");

        let method = "POST";
        let path = "/gateway/prover/fhe/456/submit";
        let timestamp = chrono::Utc::now().timestamp();
        let body = b"binary encrypted result data here...";

        // Hash the body
        let mut hasher = Blake2s256::new();
        hasher.update(body);
        let body_hash = hex::encode(hasher.finalize());

        // Build message to sign
        let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
        println!("Message to sign: {}", message);

        // Sign the message
        let signature = keypair.sign_message(message.as_bytes());
        println!("Signature: {}\n", signature);

        println!("HTTP Headers:");
        println!("  X-Prover-Pubkey: {}", pubkey);
        println!("  X-Prover-Signature: {}", signature);
        println!("  X-Timestamp: {}", timestamp);
        println!("  Content-Type: application/octet-stream");
        println!("\nBody: <binary data>");
        println!();
    }

    println!("\n=== cURL Example ===\n");

    // Generate a real example for curl
    let method = "GET";
    let path = "/gateway/prover/witness/test123";
    let timestamp = chrono::Utc::now().timestamp();
    let body = b"";

    let mut hasher = Blake2s256::new();
    hasher.update(body);
    let body_hash = hex::encode(hasher.finalize());

    let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
    let signature = keypair.sign_message(message.as_bytes());

    println!("curl -X GET \\");
    println!("  -H 'X-Prover-Pubkey: {}' \\", pubkey);
    println!("  -H 'X-Prover-Signature: {}' \\", signature);
    println!("  -H 'X-Timestamp: {}' \\", timestamp);
    println!("  http://localhost:8081{}", path);
    println!();
}
