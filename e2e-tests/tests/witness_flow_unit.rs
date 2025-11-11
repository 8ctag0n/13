mod common;

use common::create_test_witness;
use common::fixtures::create_invalid_witness;
use prover_node::witness_encryption::{WitnessEncryption, EncryptedWitness};

/// Test 1: Encrypt/Decrypt Roundtrip
/// Verifies that witness data can be encrypted and decrypted successfully
#[test]
fn test_witness_encrypt_decrypt_roundtrip() {
    // Setup
    let witness = create_test_witness();
    let encryption = WitnessEncryption::new().unwrap();

    // Encrypt
    let encrypted = WitnessEncryption::encrypt_witness(
        &witness,
        &encryption.public_key()
    ).unwrap();

    // Verify encrypted data properties
    assert!(encrypted.len() > 1000, "Encrypted witness too small: {} bytes", encrypted.len());
    assert!(encrypted.len() < 2000, "Encrypted witness too large: {} bytes", encrypted.len());

    // Decrypt
    let decrypted = encryption.decrypt_witness(&encrypted).unwrap();

    // Verify all fields match exactly
    assert_eq!(witness.spend_auth_sig, decrypted.spend_auth_sig, "spend_auth_sig mismatch");
    assert_eq!(witness.note_value, decrypted.note_value, "note_value mismatch");
    assert_eq!(witness.note_rho, decrypted.note_rho, "note_rho mismatch");
    assert_eq!(witness.note_rseed, decrypted.note_rseed, "note_rseed mismatch");
    assert_eq!(witness.merkle_path, decrypted.merkle_path, "merkle_path mismatch");
    assert_eq!(witness.merkle_position, decrypted.merkle_position, "merkle_position mismatch");
    assert_eq!(witness.recipient_address, decrypted.recipient_address, "recipient_address mismatch");
    assert_eq!(witness.output_value, decrypted.output_value, "output_value mismatch");
    assert_eq!(witness.rcv, decrypted.rcv, "rcv mismatch");
}

/// Test 2: Wrong Key Fails
/// Verifies that decryption with wrong key fails (security property)
#[test]
fn test_decrypt_with_wrong_key_fails() {
    let witness = create_test_witness();
    let encryption1 = WitnessEncryption::new().unwrap();
    let encryption2 = WitnessEncryption::new().unwrap();

    // Encrypt with first key
    let encrypted = WitnessEncryption::encrypt_witness(
        &witness,
        &encryption1.public_key()
    ).unwrap();

    // Try to decrypt with second key (should fail)
    let result = encryption2.decrypt_witness(&encrypted);
    assert!(result.is_err(), "Decryption with wrong key should fail");
}

/// Test 3: Tampered Data Fails
/// Verifies that tampering with ciphertext is detected by AEAD authentication
#[test]
fn test_tampered_ciphertext_fails() {
    let witness = create_test_witness();
    let encryption = WitnessEncryption::new().unwrap();

    let encrypted = WitnessEncryption::encrypt_witness(
        &witness,
        &encryption.public_key()
    ).unwrap();

    // Parse the encrypted envelope
    let mut enc_witness: EncryptedWitness = borsh::BorshDeserialize::try_from_slice(&encrypted)
        .expect("Failed to parse encrypted witness");

    // Tamper with a byte in the ciphertext
    if enc_witness.ciphertext.len() > 100 {
        enc_witness.ciphertext[100] ^= 0xFF;
    }

    // Re-serialize
    let tampered = borsh::to_vec(&enc_witness).unwrap();

    // Decryption should fail due to AEAD authentication failure
    let result = encryption.decrypt_witness(&tampered);
    assert!(result.is_err(), "Tampered ciphertext should fail authentication");
}

/// Test 4: Witness Validation
/// Verifies that witness validation rules are enforced
#[test]
fn test_witness_validation_rules() {
    // Valid witness should pass
    let witness = create_test_witness();
    assert!(witness.validate().is_ok(), "Valid witness should pass validation");

    // Zero value should fail
    let mut invalid = witness.clone();
    invalid.note_value = 0;
    assert!(invalid.validate().is_err(), "Zero note value should fail validation");

    // Empty merkle path should fail
    let mut invalid = witness.clone();
    invalid.merkle_path = vec![];
    assert!(invalid.validate().is_err(), "Empty merkle path should fail validation");

    // Output > input should fail (insufficient funds)
    let mut invalid = witness.clone();
    invalid.output_value = invalid.note_value + 1;
    assert!(invalid.validate().is_err(), "Output > input should fail validation");

    // Merkle path too long should fail (max 32 levels)
    let mut invalid = witness.clone();
    invalid.merkle_path = vec![[0u8; 32]; 33];
    assert!(invalid.validate().is_err(), "Merkle path > 32 levels should fail validation");
}

/// Test 5: Multiple Encryptions Produce Different Ciphertexts
/// Verifies that encryption is non-deterministic (security property)
#[test]
fn test_multiple_encryptions_produce_different_ciphertexts() {
    let witness = create_test_witness();
    let encryption = WitnessEncryption::new().unwrap();
    let pubkey = encryption.public_key();

    // Encrypt same witness twice
    let encrypted1 = WitnessEncryption::encrypt_witness(&witness, &pubkey).unwrap();
    let encrypted2 = WitnessEncryption::encrypt_witness(&witness, &pubkey).unwrap();

    // Ciphertexts must be different (due to random ephemeral keys + nonces)
    assert_ne!(
        encrypted1, encrypted2,
        "Multiple encryptions should produce different ciphertexts (randomness required)"
    );

    // But both should decrypt to the same witness
    let dec1 = encryption.decrypt_witness(&encrypted1).unwrap();
    let dec2 = encryption.decrypt_witness(&encrypted2).unwrap();

    assert_eq!(dec1.note_value, dec2.note_value);
    assert_eq!(dec1.note_value, witness.note_value);
}

/// Test 6: Invalid Witness Cannot Be Used
/// Verifies that invalid witness is rejected early
#[test]
fn test_invalid_witness_rejected() {
    let invalid_witness = create_invalid_witness();

    // Validation should fail
    assert!(
        invalid_witness.validate().is_err(),
        "Invalid witness should fail validation"
    );
}
