# wZEC Payment Integration API Documentation

## Overview

The ZyberLink backend now supports **wZEC (Wrapped Zcash)** payments in addition to native SOL payments for FHE computation jobs. This document describes the API contract for creating jobs with wZEC payments.

## API Endpoint

### POST /api/jobs/validate-and-build

Creates a new FHE computation job with payment in either SOL or wZEC.

#### Request Schema

```json
{
  "creator_pubkey": "string",        // Solana public key (base58)
  "encrypted_data": "string",        // Base64 encoded encrypted input
  "server_key": "string",            // Base64 encoded TFHE server key
  "message": "string",               // Format: "create_job:{job_id}:{timestamp}:{nonce}"
  "signature": "string",             // Base64 encoded signature (64 bytes)
  "nonce": "string",                 // Anti-replay nonce
  "operation": "string",             // "add" | "multiply"
  "operation_value": number,         // u8 (1-255)
  "price_lamports": number,          // u64 (price in lamports for SOL, zatoshis for wZEC)
  "required_provers": number,        // u8 (number of provers needed)
  "consensus_threshold": number,     // u8 (minimum matching results)
  "payment_method": "string"         // "SOL" | "wZEC" (defaults to "SOL")
}
```

#### Response Schema

```json
{
  "job_id": number,                  // i64
  "transaction": "string",           // Base64 serialized unsigned transaction
  "status": "pending_signature"      // Job status
}
```

#### Error Response

```json
{
  "error": "string"                  // Human-readable error message
}
```

## Payment Methods

### SOL (Native Payment)

- **payment_method**: `"SOL"`
- **payment_token_mint**: Not used (auto-set to `null`)
- **price_lamports**: Amount in lamports (1 SOL = 1,000,000,000 lamports)
- **Instruction**: Uses `CreateJob` on-chain instruction
- **Escrow**: Native SOL escrow account (PDA)

### wZEC (Token Payment)

- **payment_method**: `"wZEC"`
- **payment_token_mint**: Automatically set to `sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ`
- **price_lamports**: Amount in zatoshis (1 wZEC = 100,000,000 zatoshis)
- **Instruction**: Uses `CreateJobWithToken` on-chain instruction
- **Escrow**: SPL token escrow account (PDA)

## Constants

```javascript
const WZEC_MINT = "sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ";
const PROGRAM_ID = "7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR";
const ZATOSHIS_PER_WZEC = 100_000_000;
```

## Database Schema

The backend stores the following additional fields:

```sql
ALTER TABLE temp_job_data
ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'SOL',
ADD COLUMN payment_token_mint TEXT NULL;
```

- **payment_method**: `"SOL"` or `"wZEC"`
- **payment_token_mint**: SPL token mint address (only for wZEC)

## Example Usage

### Creating a job with SOL payment

```javascript
const response = await fetch('http://localhost:8080/api/jobs/validate-and-build', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    creator_pubkey: "YourPublicKeyHere...",
    encrypted_data: "base64_encoded_data",
    server_key: "base64_encoded_server_key",
    message: "create_job:123:1732104000:random_nonce",
    signature: "base64_encoded_signature",
    nonce: "random_nonce",
    operation: "add",
    operation_value: 5,
    price_lamports: 1000000000,  // 1 SOL
    required_provers: 3,
    consensus_threshold: 2,
    payment_method: "SOL"  // Optional, defaults to SOL
  })
});

const result = await response.json();
console.log('Job ID:', result.job_id);
console.log('Transaction:', result.transaction);
```

### Creating a job with wZEC payment

```javascript
const response = await fetch('http://localhost:8080/api/jobs/validate-and-build', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    creator_pubkey: "YourPublicKeyHere...",
    encrypted_data: "base64_encoded_data",
    server_key: "base64_encoded_server_key",
    message: "create_job:124:1732104100:random_nonce_2",
    signature: "base64_encoded_signature",
    nonce: "random_nonce_2",
    operation: "multiply",
    operation_value: 3,
    price_lamports: 500000000,  // 5 wZEC (in zatoshis)
    required_provers: 2,
    consensus_threshold: 2,
    payment_method: "wZEC"  // NEW: Specify wZEC payment
  })
});

const result = await response.json();
console.log('Job ID:', result.job_id);
console.log('Transaction:', result.transaction);

// Transaction will include wZEC token transfer to escrow
// Frontend must sign and submit this transaction
```

## Transaction Structure

### SOL Payment Transaction

Accounts (5):
1. Creator (signer, writable)
2. Job PDA (writable)
3. Config PDA (readonly)
4. Escrow PDA (writable)
5. System Program (readonly)

### wZEC Payment Transaction

Accounts (9):
1. Creator (signer, writable)
2. Job PDA (writable)
3. Config PDA (writable)
4. Token Escrow PDA (writable)
5. Creator's Token Account (writable)
6. Token Mint (readonly)
7. System Program (readonly)
8. Token Program (readonly)
9. Rent Sysvar (readonly)

## Validation Rules

### Payment Method Validation

```rust
match payment_method {
    "SOL" => Ok(("SOL", None)),
    "wZEC" => Ok(("wZEC", Some("sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ"))),
    _ => Err("Invalid payment_method: must be 'SOL' or 'wZEC'")
}
```

### Additional Validations

All existing validations still apply:
- Signature verification
- Timestamp expiry (5 minutes)
- Nonce anti-replay
- Encrypted data size limits
- Server key format validation
- Operation validation
- Consensus config validation

## Error Codes

| HTTP Code | Error | Reason |
|-----------|-------|--------|
| 200 | Success | Job created successfully |
| 400 | Invalid payment_method | payment_method not "SOL" or "wZEC" |
| 400 | Validation failed | Signature, nonce, or data validation failed |
| 500 | Database error | Failed to store job data |
| 500 | Transaction build failed | Failed to build transaction |

## Frontend Integration Guide

### Required Steps

1. **Check Token Account**: Before creating a wZEC job, verify the user has an associated token account for wZEC:
   ```javascript
   const tokenAccount = await getAssociatedTokenAddress(
     new PublicKey(WZEC_MINT),
     wallet.publicKey
   );
   ```

2. **Build Request**: Call `/api/jobs/validate-and-build` with `payment_method: "wZEC"`

3. **Deserialize Transaction**: Parse the base64 transaction returned by the API

4. **Sign Transaction**: Use wallet to sign the transaction

5. **Submit Transaction**: Send to Solana network

6. **Confirm Job**: Call `/api/jobs/{job_id}/confirm` with the transaction signature

### Code Example

```typescript
import { Connection, Transaction } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';

async function createWZECJob(wallet, jobData) {
  // 1. Verify token account exists
  const tokenAccount = await getAssociatedTokenAddress(
    new PublicKey(WZEC_MINT),
    wallet.publicKey
  );

  // 2. Call backend API
  const response = await fetch('/api/jobs/validate-and-build', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ...jobData,
      payment_method: 'wZEC',
      creator_pubkey: wallet.publicKey.toString(),
    })
  });

  const { job_id, transaction } = await response.json();

  // 3. Deserialize and sign transaction
  const txBytes = Buffer.from(transaction, 'base64');
  const tx = Transaction.from(txBytes);
  const signedTx = await wallet.signTransaction(tx);

  // 4. Submit to network
  const connection = new Connection(RPC_URL);
  const signature = await connection.sendRawTransaction(signedTx.serialize());
  await connection.confirmTransaction(signature);

  // 5. Confirm with backend
  await fetch(`/api/jobs/${job_id}/confirm`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ signature })
  });

  return { job_id, signature };
}
```

## Testing

Run the E2E test script:

```bash
chmod +x test_wzec_payment.sh
./test_wzec_payment.sh
```

Test coverage:
- ✅ wZEC job creation
- ✅ SOL job creation (backwards compatibility)
- ✅ Invalid payment_method rejection
- ✅ Database storage verification

## Migration Notes

### Backwards Compatibility

The integration is **fully backwards compatible**:

- Existing requests without `payment_method` default to `"SOL"`
- All SOL payment flows unchanged
- No breaking changes to existing API contracts

### Database Migration

Migration already applied: `20250120000007_add_wzec_payment_support.sql`

```sql
ALTER TABLE temp_job_data
ADD COLUMN payment_token_mint TEXT NULL,
ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'SOL';

ALTER TABLE temp_job_data
ADD CONSTRAINT check_payment_method
CHECK (payment_method IN ('SOL', 'wZEC'));
```

## Support

For issues or questions:
- Check program logs: `/var/log/zyberlink/program.log`
- Check API logs: `/var/log/zyberlink/api.log`
- Review transaction on Solana Explorer
- Verify wZEC token account has sufficient balance

## Changelog

### v0.1.0 (2025-11-20)

- ✅ Added `payment_method` field to API request
- ✅ Implemented `CreateJobWithToken` instruction builder
- ✅ Added wZEC mint validation
- ✅ Updated database schema
- ✅ Created E2E test script
- ✅ Maintained SOL payment backwards compatibility
