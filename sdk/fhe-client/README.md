# FHE Client SDK

Client SDK for Fully Homomorphic Encryption (FHE) of bet amounts in Futarchy Markets.

## Overview

This SDK enables users to encrypt their bet amounts before submitting them on-chain, maintaining privacy while allowing the smart contract to perform computations on the encrypted values.

Built on top of [TFHE-rs](https://github.com/zama-ai/tfhe-rs), this SDK provides:

- Simple client-side encryption of bet amounts
- Key management (client and server keys)
- Serialization utilities for on-chain transmission
- Batch processing capabilities

## Key Concepts

### Client Key (Private)
The client key is **PRIVATE** and must be kept secure by the user. It allows:
- Encrypting bet amounts
- Decrypting results (for testing/verification)

### Server Key (Public)
The server key is **PUBLIC** and shared with provers. It allows:
- Performing homomorphic operations on encrypted data
- Computing on ciphertexts without decrypting them

### Ciphertext Size
- FheUint64 ciphertexts are typically **500-2000 bytes**
- This is the data transmitted on-chain for each bet

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fhe-client-sdk = { path = "../fhe-client-sdk" }
```

## Quick Start

```rust
use fhe_client_sdk::FutarchyFheClient;

// Create a new client (generates keys - takes a few seconds)
let client = FutarchyFheClient::new()?;

// Encrypt a bet amount
let bet_amount = 1000_u64;
let encrypted_bet = client.encrypt_bet_amount(bet_amount)?;

// encrypted_bet is now ready to send on-chain (Vec<u8>)

// For testing: decrypt to verify
let decrypted = client.decrypt_pool(&encrypted_bet)?;
assert_eq!(bet_amount, decrypted);
```

## Usage Examples

### Basic Encryption

```rust
use fhe_client_sdk::FutarchyFheClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = FutarchyFheClient::new()?;

    // Encrypt bet
    let bet = 5000_u64;
    let encrypted = client.encrypt_bet_amount(bet)?;

    println!("Encrypted {} tokens into {} bytes", bet, encrypted.len());

    Ok(())
}
```

### Key Persistence

```rust
use fhe_client_sdk::FutarchyFheClient;
use std::fs;

fn save_keys(client: &FutarchyFheClient) -> std::io::Result<()> {
    // Save client key (PRIVATE - encrypt before storing in production)
    fs::write("client_key.bin", client.get_client_key_bytes())?;

    // Save server key (PUBLIC - can be shared)
    fs::write("server_key.bin", client.get_server_key_bytes())?;

    Ok(())
}

fn load_keys() -> Result<FutarchyFheClient, Box<dyn std::error::Error>> {
    let client_key = fs::read("client_key.bin")?;
    let server_key = fs::read("server_key.bin")?;

    let client = FutarchyFheClient::from_keys(&client_key, &server_key)?;

    Ok(client)
}
```

### Batch Processing

```rust
use fhe_client_sdk::FutarchyFheClient;

fn encrypt_multiple_bets() -> Result<(), Box<dyn std::error::Error>> {
    let client = FutarchyFheClient::new()?;

    let bets = vec![1000, 2000, 3000, 4000, 5000];
    let encrypted_bets = client.encrypt_batch(&bets)?;

    println!("Encrypted {} bets", encrypted_bets.len());

    // Each encrypted bet can now be sent on-chain
    for (i, encrypted) in encrypted_bets.iter().enumerate() {
        println!("Bet {}: {} bytes", i, encrypted.len());
    }

    Ok(())
}
```

### Using with Hex Encoding

```rust
use fhe_client_sdk::{FutarchyFheClient, serialization::ciphertext_to_hex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FutarchyFheClient::new()?;

    let encrypted_bytes = client.encrypt_bet_amount(1234)?;

    // Convert to hex for JSON/API transmission
    let ct = fhe_client_sdk::serialization::deserialize_ciphertext(&encrypted_bytes)?;
    let hex_string = ciphertext_to_hex(&ct)?;

    println!("Hex representation: {}", hex_string);

    Ok(())
}
```

## Architecture

```
┌─────────────────┐
│  User Wallet    │
│                 │
│  FutarchyFHE    │  <- Generates keys (first time)
│  Client         │  <- Encrypts bet amounts
└────────┬────────┘
         │
         │ encrypted_bet (Vec<u8>)
         ▼
┌─────────────────┐
│  Solana         │
│  Program        │  <- Stores encrypted bets
│                 │  <- Pools remain encrypted
└────────┬────────┘
         │
         │ server_key + encrypted_pools
         ▼
┌─────────────────┐
│  ZK Prover      │  <- Performs FHE operations
│                 │  <- Generates proofs
│                 │  <- Never sees plaintext
└─────────────────┘
```

## Security Considerations

### Client Key Storage
- **NEVER** store the client key in plaintext in production
- Consider using:
  - Browser local storage with encryption
  - Hardware wallets
  - Secure enclaves
  - Password-derived encryption

### Server Key Distribution
- The server key is public and can be:
  - Stored on-chain
  - Distributed via IPFS
  - Embedded in the prover service

### Ciphertext Transmission
- Ciphertexts are safe to transmit over insecure channels
- They reveal no information about the plaintext
- Each encryption produces a different ciphertext (randomness)

## Performance

### Key Generation
- Takes **3-10 seconds** on modern hardware
- Should be done once and keys stored for reuse

### Encryption
- Takes **50-200ms** per value
- Batch encryption is more efficient
- Consider doing encryption off the critical path

### Ciphertext Size
- FheUint64: **~500-2000 bytes**
- Plan for on-chain storage costs

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run specific test:

```bash
cargo test test_encrypt_decrypt_roundtrip
```

## Examples

See the `examples/` directory for more usage examples:

```bash
cargo run --example basic_usage
cargo run --example key_management
cargo run --example batch_processing
```

## API Reference

### `FutarchyFheClient`

#### `new() -> Result<Self, FheError>`
Create a new client with freshly generated keys.

#### `from_keys(client_key_bytes: &[u8], server_key_bytes: &[u8]) -> Result<Self, FheError>`
Restore a client from previously saved keys.

#### `encrypt_bet_amount(amount: u64) -> Result<Vec<u8>, FheError>`
Encrypt a bet amount for on-chain submission.

#### `decrypt_pool(encrypted: &[u8]) -> Result<u64, FheError>`
Decrypt an encrypted value (for testing/verification).

#### `get_client_key_bytes() -> Vec<u8>`
Get the client key for storage (keep private).

#### `get_server_key_bytes() -> Vec<u8>`
Get the server key for sharing with provers.

#### `encrypt_batch(amounts: &[u64]) -> Result<Vec<Vec<u8>>, FheError>`
Encrypt multiple values efficiently.

#### `verify_keys() -> Result<(), FheError>`
Verify that the keys work correctly.

## Troubleshooting

### Keys don't work after loading
- Ensure you're using the same version of TFHE
- Verify the serialized data isn't corrupted
- Check that client and server keys match

### Encryption is slow
- Key generation is slow (once per client)
- Encryption itself should be <200ms
- Use batch operations for multiple values

### Ciphertexts are too large
- This is normal for FHE (500-2000 bytes)
- Consider compression for transmission
- Plan for on-chain storage costs

## License

MIT

## Support

For issues and questions, please open an issue on GitHub.
