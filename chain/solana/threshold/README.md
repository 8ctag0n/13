# Threshold - ZyberLink Threshold Encryption Coordinator

On-chain coordinator for the 3-of-5 threshold encryption protocol that protects witness data in ZyberLink v2.0.

## Overview

The Threshold program manages the protocol where:
1. Provers request key shares for encrypted witness data
2. Validators (registered in Bedrock) submit their encrypted key shares
3. Once 3+ validators respond, the prover can reconstruct the decryption key off-chain

## Architecture

```text
┌──────────────────────────────────────────────────────┐
│              THRESHOLD (this)                         │
│   (Key Share Request/Response Coordination)           │
└──────────────────────────────────────────────────────┘
               ↓ reads              ↑ reads
┌──────────────────┐          ┌─────────────────┐
│   ZK-GENERATOR   │          │     BEDROCK     │
│  (ZK Job state)  │          │  (Validators)   │
└──────────────────┘          └─────────────────┘
```

## Instructions

### RequestKeyShare

Prover requests key shares for a ZK job.

**Accounts:**
- `[signer]` Prover (must match job's prover)
- `[writable]` Key share request PDA
- `[]` ZK job account
- `[]` Prover account (from bedrock)
- `[]` System program

**Parameters:**
- `encrypted_witness_cid: String` - IPFS CID of encrypted witness data

### SubmitKeyShare

Validator submits their encrypted key share.

**Accounts:**
- `[signer]` Validator authority
- `[writable]` Key share response PDA
- `[writable]` Key share request
- `[]` Validator account (from bedrock)
- `[]` System program
- `[]` Clock sysvar

**Parameters:**
- `encrypted_share: Vec<u8>` - Encrypted key share (max 256 bytes)

## State

### KeyShareRequest

PDA: `["keyshare_request", job_id]`

- Timeout: 5 minutes (300 seconds)
- Threshold: 3 responses required
- Status transitions: Pending → Ready → Completed

### KeyShareResponse

PDA: `["keyshare_response", request_id, validator]`

- One response per validator per request
- Encrypted for prover's public key
- Max share size: 256 bytes

## Protocol Flow

1. **Prover claims ZK job** (via zk-generator)
2. **Prover requests key shares** → Creates KeyShareRequest
3. **Validators monitor requests** off-chain
4. **Validators submit shares** → Creates KeyShareResponse
5. **Threshold met (3+)** → Request status becomes Ready
6. **Prover retrieves responses** off-chain
7. **Prover reconstructs key** off-chain using Shamir's Secret Sharing
8. **Prover decrypts witness** and generates proof

## Security

- Only active validators (staked in Bedrock) can submit shares
- Each validator can only respond once per request
- Requests expire after 5 minutes
- Shares are encrypted for the prover's public key (off-chain)
- On-chain program only coordinates protocol, doesn't handle cryptography

## Building

```bash
cargo build-sbf --manifest-path=threshold/Cargo.toml
```

## Testing

```bash
cargo test-sbf -p threshold
```
