# Futarchy E2E Bet Flow

## Overview

Two-step flow to place encrypted bets on Futarchy markets with anti-spam protection:

1. **Prepare**: Client sends bet details → server builds unsigned transaction
2. **Submit**: Client signs transaction → server validates hash → sends to Solana → stores ciphertext only if TX confirms

## Endpoints

### POST /api/futarchy/bet/prepare

Build unsigned transaction for placing a bet.

**Request:**
```json
{
  "market_id": 1,
  "bettor": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
  "side": true,
  "amount_lamports": 1000000000,
  "ciphertext_hash": "a1b2c3d4e5f6...",
  "proof": "base64_encoded_proof",
  "public_inputs": "base64_encoded_public_inputs",
  "circuit_type": 1
}
```

**Response:**
```json
{
  "unsigned_transaction": "base64_serialized_tx",
  "market_id": 1,
  "signers": ["9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"]
}
```

### POST /api/futarchy/bet/submit

Submit signed transaction and store ciphertext.

**Request:**
```json
{
  "signed_tx": "base64_serialized_signed_tx",
  "ciphertext": "base64_encoded_ciphertext",
  "server_key": "base64_encoded_server_key"
}
```

**Response (Success):**
```json
{
  "tx_signature": "5J4k3j2h1g...",
  "status": "confirmed",
  "ciphertext_hash": "a1b2c3d4e5f6..."
}
```

**Response (TX Failed):**
```json
{
  "error": "Transaction failed: insufficient funds",
  "status": "failed"
}
```

## Flow Diagram

```
Client                     Blink Server              Solana Blockchain
  |                             |                           |
  |--prepare bet request------->|                           |
  |                             |--build_place_bet_ix()     |
  |                             |--prepare_unsigned_tx()    |
  |<--unsigned_transaction------|                           |
  |                             |                           |
  | sign TX locally             |                           |
  |                             |                           |
  |--submit signed TX---------->|                           |
  |   + ciphertext              |                           |
  |   + server_key              |                           |
  |                             |--validate hash(ciphertext)|
  |                             |                           |
  |                             |--send_and_confirm_tx()-->|
  |                             |                           |--process TX
  |                             |<--confirmation-----------|
  |                             |                           |
  |                             |--save_ciphertext_with_tx()|
  |                             |--mark_confirmed()         |
  |                             |                           |
  |<--tx_signature + status-----|                           |
```

## Anti-Spam Protection

1. Ciphertext is NOT stored until TX is confirmed on-chain
2. If TX fails, no DB record is created (garbage collection happens automatically)
3. Hash validation ensures ciphertext matches the commitment in the TX
4. Server key is stored separately for FHE computation

## Database Schema

```sql
-- Ciphertext storage (only after TX confirms)
INSERT INTO futarchy_ciphertexts (
  hash,
  ciphertext,
  server_key_hash,
  tx_signature,
  status,         -- 'pending' → 'confirmed' | 'failed'
  created_by
);

-- Position tracking
INSERT INTO futarchy_positions (
  market_id,
  bettor,
  side,
  amount_lamports,
  encrypted_amount_hash,
  tx_signature,
  status
);
```

## Error Handling

- **Invalid bettor pubkey**: 400 Bad Request
- **Invalid ciphertext_hash**: 400 Bad Request (must be 64 hex chars)
- **Invalid proof/public_inputs**: 400 Bad Request (must be valid base64)
- **TX submission failed**: 400 Bad Request + cleanup attempt
- **DB save failed after TX**: 500 Internal Server Error (TX confirmed but data not stored)

## Security Considerations

1. Hash validation prevents malicious ciphertext injection
2. Only confirmed transactions result in stored data
3. Server key is optional (only needed for first bet by user)
4. Proof validation happens on-chain via ZK generator program
