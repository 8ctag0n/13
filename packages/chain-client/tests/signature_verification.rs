use zyberlink_chain_client::{AptosClient, StarknetClient, SignatureVerifier};

#[test]
fn test_aptos_signature_encoding() {
    let client = AptosClient::new("https://api.devnet.aptoslabs.com").unwrap();
    assert_eq!(client.signature_encoding(), "hex");
    assert_eq!(client.signature_algorithm(), "ed25519");
}

#[test]
fn test_starknet_signature_encoding() {
    let client = StarknetClient::new("https://alpha4.starknet.io").unwrap();
    assert_eq!(client.signature_encoding(), "hex");
    assert_eq!(client.signature_algorithm(), "ecdsa-stark");
}

#[test]
fn test_aptos_verify_invalid_pubkey() {
    let client = AptosClient::new("https://api.devnet.aptoslabs.com").unwrap();

    // Invalid hex should fail
    let result = client.verify_signature("invalid_hex", "0102", b"message");
    assert!(result.is_err());
}

#[test]
fn test_aptos_verify_invalid_signature() {
    let client = AptosClient::new("https://api.devnet.aptoslabs.com").unwrap();

    // Valid hex pubkey (32 bytes), invalid signature format
    let pubkey = "0".repeat(64); // 32 bytes in hex
    let result = client.verify_signature(&pubkey, "invalid_hex", b"message");
    assert!(result.is_err());
}

#[test]
fn test_starknet_not_implemented() {
    let client = StarknetClient::new("https://alpha4.starknet.io").unwrap();

    // Should return NotImplemented error
    let result = client.verify_signature("0102", "0304", b"message");
    assert!(result.is_err());

    if let Err(e) = result {
        let err_str = e.to_string();
        // Check for the specific error message
        assert!(err_str.contains("requires starknet-crypto dependency"));
    }
}
