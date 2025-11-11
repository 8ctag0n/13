# Witness Encryption Integration Guide

This document explains how to use the complete witness encryption system in CypherLink.

## Overview

The system consists of three main components:

1. **Solana Program**: ProverAccount stores encryption_pubkey on-chain
2. **Witness Storage Backend**: HTTP server for storing encrypted witness data
3. **Prover Node**: Downloads and decrypts witness, generates proofs

## Architecture Flow

```
Mobile Client                Solana                 Backend HTTP         Prover Node
     |                          |                         |                    |
     | 1. Query prover pubkey   |                         |                    |
     |------------------------->|                         |                    |
     | 2. Encrypt witness       |                         |                    |
     |    with prover pubkey    |                         |                    |
     | 3. Upload to backend     |                         |                    |
     |---------------------------------------------->     |                    |
     |                          |      4. Get commitment  |                    |
     | 5. CreateJob(commitment) |                         |                    |
     |------------------------->|                         |                    |
     |                          | 6. Poll jobs            |                    |
     |                          |<--------------------------------------------|
     |                          | 7. ClaimJob             |                    |
     |                          |<--------------------------------------------|
     |                          |                         | 8. Download witness|
     |                          |                         |<-------------------|
     |                          |                         | 9. Decrypt + prove |
     |                          | 10. SubmitProof         |                    |
     |                          |<--------------------------------------------|
```

## Setup

### 1. Start Witness Storage Backend

```bash
# Terminal 1
cd witness-storage
RUST_LOG=info cargo run

# Server will listen on http://0.0.0.0:3030
```

### 2. Register Prover with Encryption Key

```bash
# Generate and register prover
cd prover-node
cargo run -- register \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  --stake-amount 10000000000

# Output:
# Registering prover...
#   Authority: <PUBKEY>
#   Stake: 10000000000 lamports (10.0 SOL)
#   Encryption pubkey: a1b2c3d4...
# Prover registered successfully!
#   Signature: <TX_SIG>
#   Prover PDA: <PROVER_PDA>
```

### 3. Show Prover Encryption Key

```bash
cargo run -- show-pubkey --keypair ~/.config/solana/id.json

# Output:
# Prover Encryption Public Key
# =============================
# Authority:       <PUBKEY>
# Encryption Key:  a1b2c3d4e5f6...
#
# Clients should use this key to encrypt witness data before uploading.
```

### 4. Run Prover Node

```bash
cargo run -- run \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  --witness-backend-url http://localhost:3030
```

## Client Workflow

### Step 1: Query Prover Encryption Key

```rust
use solana_client::rpc_client::RpcClient;
use borsh::BorshDeserialize;

let rpc_client = RpcClient::new("http://localhost:8899".to_string());
let prover_pda = /* derive from prover authority */;

let prover_account = rpc_client.get_account(&prover_pda)?;
let prover: ProverAccount = BorshDeserialize::deserialize(&mut &prover_account.data[..])?;

let encryption_pubkey = prover.encryption_pubkey; // [u8; 32]
```

### Step 2: Encrypt Witness

```rust
use x25519_dalek::PublicKey;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::Aead};
use rand::rngs::OsRng;

// Your witness data (e.g., OrchardWitness serialized)
let witness_data = borsh::to_vec(&my_witness)?;

// Encrypt using prover's public key
let prover_pubkey = PublicKey::from(encryption_pubkey);
let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
let ephemeral_public = PublicKey::from(&ephemeral_secret);

let shared_secret = ephemeral_secret.diffie_hellman(&prover_pubkey);
let cipher = ChaCha20Poly1305::new(shared_secret.as_bytes().into());

let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
let ciphertext = cipher.encrypt(&nonce, witness_data.as_ref())?;

let encrypted_witness = EncryptedWitness {
    ephemeral_public_key: *ephemeral_public.as_bytes(),
    nonce: nonce.into(),
    ciphertext,
};

let encrypted_bytes = borsh::to_vec(&encrypted_witness)?;
```

### Step 3: Upload to Backend

```rust
use reqwest::Client;

let client = Client::new();
let response = client
    .post("http://localhost:3030/witness")
    .header("Content-Type", "application/octet-stream")
    .body(encrypted_bytes)
    .send()
    .await?;

let json: serde_json::Value = response.json().await?;
let commitment_hex = json["commitment"].as_str().unwrap();

// Parse commitment to [u8; 32]
let commitment_bytes = hex::decode(commitment_hex)?;
let mut witness_commitment = [0u8; 32];
witness_commitment.copy_from_slice(&commitment_bytes);
```

### Step 4: Create Job On-Chain

```rust
use cypherlink_sdk::MarketplaceClient;

let client = MarketplaceClient::new(rpc_url, program_id);

let create_job_ix = client.create_job_instruction(
    &job_creator.pubkey(),
    job_id,
    CircuitType::ZcashOrchard,
    witness_commitment, // From step 3
    encrypted_bytes.len() as u32,
    1_000_000, // price in lamports
    600, // timeout in seconds
)?;

let sig = client.send_and_confirm_transaction(&[create_job_ix], &[&job_creator])?;
```

## Prover Workflow (Automated)

The prover node automatically:

1. **Polls** for pending jobs
2. **Claims** suitable jobs
3. **Downloads** encrypted witness from backend using commitment
4. **Decrypts** witness using its private key
5. **Generates** Halo2 proof
6. **Submits** proof on-chain

Logs will show:

```
[Job 42] Starting processing
[Job 42] Claiming job...
[Job 42] Claimed successfully (sig: abc123...)
[Job 42] Downloading encrypted witness from backend...
[Job 42] Downloaded encrypted witness (1234 bytes)
[Job 42] Decrypting witness data...
[Job 42] Witness decrypted successfully
[Job 42] Generating proof (circuit: ZcashOrchard)...
[Job 42] Proof generated successfully (1536 bytes)
[Job 42] Submitting proof...
[Job 42] Proof submitted successfully (sig: def456...)
[Job 42] Completed!
```

## Security Considerations

1. **Encryption**: X25519 ECDH + ChaCha20-Poly1305 AEAD
2. **Key Management**: Each prover has unique keypair
3. **Ephemeral Keys**: Each witness encryption uses new ephemeral key
4. **Commitment Binding**: Blake2b commitment prevents witness tampering
5. **On-Chain Verification**: Prover pubkey stored immutably on-chain

## Testing

### Unit Tests

```bash
# Test Solana program
cargo test --package cypherlink

# Test witness storage backend
cargo test --package cypherlink-witness-storage

# Test prover node
cargo test --package cypherlink-prover
```

### Integration Test

```bash
# Terminal 1: Start witness storage
cd witness-storage
cargo run

# Terminal 2: Run prover node tests with --ignored flag
cd prover-node
cargo test --ignored
```

## Production Deployment

### Backend Storage

Replace in-memory HashMap with:
- **Redis**: For ephemeral storage with TTL
- **PostgreSQL**: For persistent storage
- **S3/IPFS**: For decentralized storage

### Prover Security

- Store private key in secure enclave (HSM, SGX, etc.)
- Use deterministic key derivation from master seed
- Implement key rotation mechanism
- Add monitoring and alerting

### Network

- Add authentication to backend (API keys, OAuth)
- Implement rate limiting
- Use HTTPS with proper certificates
- Add CORS restrictions for production

## API Reference

### Witness Storage Backend

**POST /witness**
- Upload encrypted witness
- Returns: `{"commitment": "hex", "size": 1234, "status": "stored"}`

**GET /witness/:commitment**
- Download encrypted witness by commitment
- Returns: Binary data (encrypted witness)

**GET /health**
- Health check
- Returns: `{"status": "ok", "witnesses_stored": 42, "total_bytes": 123456}`

### Prover CLI

```bash
# Run prover daemon
cypherlink-prover run \
  --program-id <PROGRAM_ID> \
  --keypair <PATH> \
  --witness-backend-url <URL>

# Register prover
cypherlink-prover register \
  --program-id <PROGRAM_ID> \
  --keypair <PATH> \
  --stake-amount 10000000000

# Show encryption pubkey
cypherlink-prover show-pubkey --keypair <PATH>
```

## Troubleshooting

### "Witness not found for commitment"
- Check that upload completed successfully
- Verify commitment hex matches exactly
- Ensure backend is running and accessible

### "Failed to decrypt witness data"
- Verify prover is using correct keypair
- Check encryption_pubkey matches on-chain
- Ensure client used correct prover pubkey

### "Invalid witness data"
- Witness may be corrupted during upload/download
- Check network connectivity
- Verify serialization format matches

## Future Enhancements

- [ ] Light Protocol integration for compressed witness storage
- [ ] Multi-prover redundancy (encrypt for multiple provers)
- [ ] Witness sharding for large data
- [ ] Decentralized storage (IPFS, Arweave)
- [ ] Client SDK for mobile (Swift, Kotlin)
