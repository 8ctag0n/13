# Local Proof Generation for Market Claims

This document explains how to use the CLI to generate market claim proofs locally using snarkjs.

## Overview

The CLI now supports two modes for claiming market winnings:

1. **Delegated Mode** (default): Submit claim to ZK server which generates the proof
2. **Local Mode** (new): Generate the proof locally using snarkjs and submit directly to claim endpoint

## Prerequisites

For local proof generation, you need:

1. **snarkjs** installed (via npm/npx)
2. **Circuit artifacts**:
   - `market_claim.wasm` - Circuit WASM file
   - `market_claim_final.zkey` - Circuit proving key
3. **Bet witness file** containing:
   - `market_id`: Market ID (u64)
   - `secret`: 32-byte secret (hex string)
   - `blinding`: 32-byte blinding factor (hex string)
   - `bet_amount`: Bet amount in lamports (u64)
   - `bet_side`: 0 = NO, 1 = YES (u8)
   - `bet_commitment`: 32-byte commitment (hex string)

## Usage

### Local Proof Generation

```bash
zyb market claim \
  --market-id 1234567890 \
  --bet-id 0 \
  --claimer <PUBKEY> \
  --local-proof \
  --bet-witness ./bet_witness.json \
  --circuit-wasm circuits/market/market_claim_js/market_claim.wasm \
  --circuit-zkey circuits/market/market_claim_final.zkey
```

### Delegated Proof Generation (Original)

```bash
zyb market claim \
  --market-id 1234567890 \
  --bet-id 42 \
  --claimer <PUBKEY> \
  --keypair ~/.config/solana/id.json
```

## Bet Witness Format

Example `bet_witness.json`:

```json
{
  "market_id": 1234567890,
  "secret": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "blinding": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
  "bet_amount": 1000000000,
  "bet_side": 1,
  "bet_commitment": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

## Flow Comparison

### Delegated Mode
1. CLI builds witness
2. CLI submits ZK job to server
3. Server generates proof
4. Server submits claim transaction
5. CLI polls for completion

### Local Mode
1. CLI loads bet witness from file
2. CLI fetches market data from server (`GET /api/futarchy/markets/{id}`)
3. CLI calculates payout amount
4. CLI generates nullifier
5. CLI generates proof locally with snarkjs (10-30s)
6. CLI submits proof directly to claim endpoint (`POST /api/futarchy/claim`)

## Server Endpoints

### GET /api/futarchy/markets/{id}

Returns market data needed for claim calculation:

```json
{
  "market_id": 1234567890,
  "total_pool": 5000000000,
  "winning_pool": 3000000000,
  "resolution": 1,
  "total_yes_bets": 3000000000,
  "total_no_bets": 2000000000
}
```

### POST /api/futarchy/claim

Accepts claim with locally generated proof:

```json
{
  "market_id": 1234567890,
  "proof": "<base64-encoded-256-bytes>",
  "public_inputs": "<base64-encoded-105-bytes>",
  "claimer": "<PUBKEY>",
  "nullifier": "<hex-64-chars>",
  "payout_amount": 1666666666
}
```

## Advantages of Local Proof Generation

1. **Privacy**: Secrets never leave your machine
2. **Control**: You generate and verify the proof yourself
3. **No ZK job fees**: Direct submission to claim endpoint
4. **Transparency**: Full visibility into proof generation

## Limitations

1. **Requires circuit artifacts**: Must have WASM and zkey files locally
2. **Slower**: Proof generation takes 10-30s on local machine
3. **Requires snarkjs**: Must have Node.js/npm installed
4. **Manual witness management**: User must save and manage bet witness files

## Future Enhancements

- [ ] Auto-save bet witness during `place_bet` command
- [ ] Support for verifying proof before submission
- [ ] Batch claiming multiple bets
- [ ] Witness encryption/decryption
