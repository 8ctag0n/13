/// Integration tests for FHE (Fully Homomorphic Encryption) functionality
///
/// Tests the complete FHE workflow:
/// 1. Key generation
/// 2. Client-side encryption
/// 3. Prover-side computation
/// 4. Result hashing for consensus
/// 5. Client-side decryption and verification

use prover_node::{fhe_engine::FheEngine, generate_fhe_keys};
use tfhe::{prelude::*, FheUint8};

#[test]
fn test_fhe_engine_integration() {
    println!("Testing FHE engine integration...");

    // Step 1: Generate keys (simulates client setup)
    println!("  1. Generating FHE keypair...");
    let (client_key, server_key) = generate_fhe_keys().expect("Failed to generate keys");

    // Step 2: Client encrypts value
    println!("  2. Client encrypting value...");
    let plaintext_value = 100u8;
    let encrypted = FheUint8::try_encrypt(plaintext_value, &client_key)
        .expect("Failed to encrypt value");
    let encrypted_bytes = bincode::serialize(&encrypted).expect("Failed to serialize ciphertext");

    println!("     Encrypted {} to {} bytes", plaintext_value, encrypted_bytes.len());

    // Step 3: Prover computes on encrypted data
    println!("  3. Prover computing on encrypted data...");
    let engine = FheEngine::new(server_key);
    let constant_to_add = 50u8;
    let result_bytes = engine
        .compute_add(&encrypted_bytes, constant_to_add)
        .expect("Failed to compute FHE addition");

    println!("     Added {} to encrypted value", constant_to_add);

    // Step 4: Hash result for consensus
    println!("  4. Hashing result for consensus...");
    let result_hash = FheEngine::hash_result(&result_bytes);
    println!("     Result hash: {}", hex::encode(&result_hash[..8]));

    // Step 5: Client decrypts result
    println!("  5. Client decrypting result...");
    let result_ciphertext: FheUint8 =
        bincode::deserialize(&result_bytes).expect("Failed to deserialize result");
    let decrypted: u8 = result_ciphertext.decrypt(&client_key);

    println!("     Decrypted result: {}", decrypted);

    // Verify correctness
    let expected = plaintext_value + constant_to_add;
    assert_eq!(
        decrypted, expected,
        "FHE computation incorrect: got {}, expected {}",
        decrypted, expected
    );

    println!("\nFHE integration test PASSED!");
}

#[test]
fn test_fhe_multiply_operation() {
    println!("Testing FHE multiplication...");

    let (client_key, server_key) = generate_fhe_keys().unwrap();

    // Encrypt
    let plaintext = 7u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

    // Compute
    let engine = FheEngine::new(server_key);
    let multiplier = 6u8;
    let result_bytes = engine.compute_multiply(&encrypted_bytes, multiplier).unwrap();

    // Decrypt
    let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
    let decrypted: u8 = result.decrypt(&client_key);

    assert_eq!(decrypted, 42, "Multiplication failed");
    println!("FHE multiplication test PASSED!");
}

#[test]
fn test_fhe_subtract_operation() {
    println!("Testing FHE subtraction...");

    let (client_key, server_key) = generate_fhe_keys().unwrap();

    let plaintext = 100u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

    let engine = FheEngine::new(server_key);
    let result_bytes = engine.compute_subtract(&encrypted_bytes, 42).unwrap();

    let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
    let decrypted: u8 = result.decrypt(&client_key);

    assert_eq!(decrypted, 58, "Subtraction failed");
    println!("FHE subtraction test PASSED!");
}

#[test]
fn test_fhe_encrypted_addition() {
    println!("Testing FHE addition of two encrypted values...");

    let (client_key, server_key) = generate_fhe_keys().unwrap();

    // Encrypt two values
    let value_a = 100u8;
    let value_b = 50u8;

    let encrypted_a = FheUint8::try_encrypt(value_a, &client_key).unwrap();
    let encrypted_b = FheUint8::try_encrypt(value_b, &client_key).unwrap();

    let bytes_a = bincode::serialize(&encrypted_a).unwrap();
    let bytes_b = bincode::serialize(&encrypted_b).unwrap();

    // Prover adds encrypted values
    let engine = FheEngine::new(server_key);
    let result_bytes = engine.compute_add_encrypted(&bytes_a, &bytes_b).unwrap();

    // Client decrypts
    let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
    let decrypted: u8 = result.decrypt(&client_key);

    assert_eq!(decrypted, 150, "Encrypted addition failed");
    println!("FHE encrypted addition test PASSED!");
}

#[test]
fn test_fhe_consensus_hashing() {
    println!("Testing FHE result hashing for consensus...");

    let (client_key, server_key) = generate_fhe_keys().unwrap();

    let plaintext = 42u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

    let engine = FheEngine::new(server_key);
    let result_bytes = engine.compute_add(&encrypted_bytes, 10).unwrap();

    // Multiple provers should produce identical hashes for same result
    let hash1 = FheEngine::hash_result(&result_bytes);
    let hash2 = FheEngine::hash_result(&result_bytes);

    assert_eq!(hash1, hash2, "Hashes should be identical for same input");

    // Different results should produce different hashes
    let result_bytes_2 = engine.compute_add(&encrypted_bytes, 20).unwrap();
    let hash3 = FheEngine::hash_result(&result_bytes_2);

    assert_ne!(hash1, hash3, "Different results should produce different hashes");

    println!("FHE consensus hashing test PASSED!");
}

#[test]
fn test_fhe_performance_benchmark() {
    use std::time::Instant;

    println!("Benchmarking FHE operations...");

    // Key generation benchmark
    let start = Instant::now();
    let (client_key, server_key) = generate_fhe_keys().unwrap();
    let keygen_time = start.elapsed();
    println!("  Key generation: {:?}", keygen_time);

    // Encryption benchmark
    let start = Instant::now();
    let plaintext = 42u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();
    let encrypt_time = start.elapsed();
    println!("  Encryption: {:?}", encrypt_time);

    // Addition benchmark
    let engine = FheEngine::new(server_key);
    let start = Instant::now();
    let result_bytes = engine.compute_add(&encrypted_bytes, 10).unwrap();
    let add_time = start.elapsed();
    println!("  FHE Addition: {:?}", add_time);

    // Decryption benchmark
    let start = Instant::now();
    let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
    let decrypted: u8 = result.decrypt(&client_key);
    let decrypt_time = start.elapsed();
    println!("  Decryption: {:?}", decrypt_time);

    // Verify correctness
    assert_eq!(decrypted, 52);

    // Performance assertions (based on spike results)
    assert!(
        add_time.as_millis() < 500,
        "FHE addition should complete in < 500ms (got {:?})",
        add_time
    );

    println!("\nFHE performance benchmark PASSED!");
    println!("  Total operation time: {:?}", encrypt_time + add_time + decrypt_time);
}

#[test]
fn test_fhe_key_serialization_roundtrip() {
    use prover_node::fhe_engine::{
        deserialize_client_key, deserialize_server_key, serialize_client_key,
        serialize_server_key,
    };

    println!("Testing FHE key serialization...");

    // Generate keys
    let (client_key, server_key) = generate_fhe_keys().unwrap();

    // Serialize
    let client_bytes = serialize_client_key(&client_key).unwrap();
    let server_bytes = serialize_server_key(&server_key).unwrap();

    println!("  Client key size: {:.2} MB", client_bytes.len() as f64 / 1_000_000.0);
    println!("  Server key size: {:.2} MB", server_bytes.len() as f64 / 1_000_000.0);

    // Deserialize
    let client_key_restored = deserialize_client_key(&client_bytes).unwrap();
    let server_key_restored = deserialize_server_key(&server_bytes).unwrap();

    // Verify keys work after round-trip
    let plaintext = 123u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key_restored).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

    let engine = FheEngine::new(server_key_restored);
    let result_bytes = engine.compute_add(&encrypted_bytes, 1).unwrap();

    let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
    let decrypted: u8 = result.decrypt(&client_key_restored);

    assert_eq!(decrypted, 124, "Keys don't work after serialization");

    println!("FHE key serialization test PASSED!");
}

#[test]
#[ignore] // This test is slow due to multiple key generations
fn test_fhe_multi_prover_consensus() {
    println!("Testing multi-prover consensus simulation...");

    // Simulate 3 provers with same server key
    let (client_key, server_key) = generate_fhe_keys().unwrap();

    let plaintext = 100u8;
    let encrypted = FheUint8::try_encrypt(plaintext, &client_key).unwrap();
    let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

    // Each prover computes independently
    let mut hashes = vec![];
    for prover_id in 1..=3 {
        let engine = FheEngine::new(server_key.clone());
        let result_bytes = engine.compute_add(&encrypted_bytes, 50).unwrap();
        let hash = FheEngine::hash_result(&result_bytes);
        hashes.push(hash);
        println!("  Prover {} hash: {}", prover_id, hex::encode(&hash[..8]));
    }

    // All hashes should match (consensus)
    assert_eq!(
        hashes[0], hashes[1],
        "Prover 1 and 2 should produce same hash"
    );
    assert_eq!(
        hashes[1], hashes[2],
        "Prover 2 and 3 should produce same hash"
    );

    println!("Multi-prover consensus test PASSED!");
}
