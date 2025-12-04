# wZEC Payment API Reference

Complete API reference for integrating wZEC (Wrapped Zcash) payments into the ZyberLink marketplace.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Endpoints](#endpoints)
  - [POST /api/jobs/validate-and-build](#post-apijobsvalidate-and-build)
  - [POST /api/jobs/:id/confirm](#post-apijobsidconfirm)
  - [GET /api/jobs/:id](#get-apijobsid)
- [Data Types](#data-types)
- [Error Codes](#error-codes)
- [Rate Limits](#rate-limits)
- [Examples](#examples)
- [SDKs](#sdks)

## Overview

### Base URL

```
Production:  https://api.zyberlink.io
Testnet:     https://api.testnet.zyberlink.io
Local:       http://localhost:3001
```

### API Version

```
Version: 1.0.0
Released: 2025-11-20
```

### Content Type

All requests and responses use `application/json`.

```http
Content-Type: application/json
Accept: application/json
```

## Authentication

### Signature-Based Authentication

The API uses Ed25519 signature-based authentication instead of API keys:

1. Client signs a message with their Solana wallet
2. Server verifies signature matches creator public key
3. Message includes timestamp and nonce for replay protection

**Message Format:**
```
create_job:{job_id}:{timestamp}:{nonce}
```

**Example:**
```
create_job:12345:1732104000:e2e_wzec_5432_1732104000
```

**Signature Format:**
- Algorithm: Ed25519
- Encoding: base58 (Solana standard)
- Length: 88 characters

**Anti-Replay Protection:**
- Timestamp must be within 5 minutes of server time
- Nonce must be unique (stored in database)
- Nonce format: any string, recommended: `{prefix}_{job_id}_{timestamp}`

## Endpoints

### POST /api/jobs/validate-and-build

Creates a new FHE computation job with payment in SOL or wZEC.

#### Request

```http
POST /api/jobs/validate-and-build HTTP/1.1
Host: api.zyberlink.io
Content-Type: application/json

{
  "creator_pubkey": "string",
  "encrypted_data": "string",
  "server_key": "string",
  "message": "string",
  "signature": "string",
  "nonce": "string",
  "operation": "string",
  "operation_value": number,
  "price_lamports": number,
  "required_provers": number,
  "consensus_threshold": number,
  "payment_method": "string"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `creator_pubkey` | string | Yes | Solana public key (base58, 32-44 chars) |
| `encrypted_data` | string | Yes | Base64 encoded encrypted input (max 1 MB) |
| `server_key` | string | Yes | Base64 encoded TFHE server key (max 200 MB) |
| `message` | string | Yes | Signed message: `create_job:{job_id}:{timestamp}:{nonce}` |
| `signature` | string | Yes | Ed25519 signature (base58, 88 chars) |
| `nonce` | string | Yes | Anti-replay nonce (unique per request) |
| `operation` | string | Yes | Operation type: `add`, `multiply`, `subtract` |
| `operation_value` | number | Yes | Operation parameter (u8: 1-255) |
| `price_lamports` | number | Yes | Payment amount (u64: SOL lamports or wZEC zatoshis) |
| `required_provers` | number | Yes | Number of provers needed (u8: 1-255) |
| `consensus_threshold` | number | Yes | Minimum matching results (u8: 1-255, <= required_provers) |
| `payment_method` | string | No | Payment type: `SOL` or `wZEC` (default: `SOL`) |

#### Field Validation

**creator_pubkey:**
```regex
^[1-9A-HJ-NP-Za-km-z]{32,44}$
```
- Must be valid Solana base58 public key
- Example: `HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf`

**encrypted_data:**
```
- Encoding: Base64
- Max size: 1 MB (1,048,576 bytes)
- Must be valid TFHE encrypted data
```

**server_key:**
```
- Encoding: Base64
- Max size: 200 MB (209,715,200 bytes)
- Must be valid TFHE ServerKey
- Typical size: ~156 MB
```

**message:**
```
Format: create_job:{job_id}:{timestamp}:{nonce}
  - job_id: positive integer
  - timestamp: Unix timestamp (seconds)
  - nonce: any string
Example: create_job:12345:1732104000:abc123
```

**signature:**
```
- Algorithm: Ed25519
- Encoding: base58
- Length: exactly 88 characters
- Must be valid signature of message
```

**nonce:**
```
- Length: 1-256 characters
- Must be unique across all requests
- Recommended format: {prefix}_{job_id}_{timestamp}
- Example: e2e_wzec_12345_1732104000
```

**operation:**
```
Valid values:
  - "add"       - FHE addition
  - "multiply"  - FHE multiplication
  - "subtract"  - FHE subtraction
```

**operation_value:**
```
- Type: u8
- Range: 1-255
- Interpretation depends on operation type
```

**price_lamports:**
```
- Type: u64
- Range: 0-18,446,744,073,709,551,615
- Unit: lamports (SOL) or zatoshis (wZEC)
- 1 SOL = 1,000,000,000 lamports
- 1 wZEC = 100,000,000 zatoshis
```

**required_provers:**
```
- Type: u8
- Range: 1-255
- Typical: 3-5
- Must be >= consensus_threshold
```

**consensus_threshold:**
```
- Type: u8
- Range: 1-255
- Must be <= required_provers
- Typical: 2 (for 2-of-3 consensus)
```

**payment_method:**
```
Valid values:
  - "SOL"  - Native Solana payment (default)
  - "wZEC" - Wrapped Zcash payment
Case-sensitive, uppercase required
```

#### Response

**Success (200 OK):**

```json
{
  "job_id": 12345,
  "transaction": "base64_serialized_unsigned_transaction",
  "status": "pending_signature"
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `job_id` | number | Unique job identifier (i64) |
| `transaction` | string | Base64 serialized unsigned Solana transaction |
| `status` | string | Job status: `pending_signature` |

**Error (400/500):**

```json
{
  "error": "Human-readable error message"
}
```

#### Example Request (SOL Payment)

```bash
curl -X POST https://api.zyberlink.io/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
    "encrypted_data": "AQIDBA==",
    "server_key": "BQYHCA==",
    "message": "create_job:12345:1732104000:nonce123",
    "signature": "5J7XqG3K8H9L2M4N6P1Q3R5S7T9U2V4W6X8Y1Z3A5B7C9D2E4F6G8H1J3K5L7M9N2P4Q6R8S1T3U5V7W9X2Y4Z6",
    "nonce": "nonce123",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": 1000000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL"
  }'
```

#### Example Request (wZEC Payment)

```bash
curl -X POST https://api.zyberlink.io/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
    "encrypted_data": "AQIDBA==",
    "server_key": "BQYHCA==",
    "message": "create_job:12346:1732104100:nonce456",
    "signature": "2A4B6C8D1E3F5G7H9J2K4L6M8N1P3Q5R7S9T2U4V6W8X1Y3Z5A7B9C2D4E6F8G1H3J5K7L9M2N4P6Q8R1S3T5U7V9W",
    "nonce": "nonce456",
    "operation": "multiply",
    "operation_value": 3,
    "price_lamports": 500000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "wZEC"
  }'
```

#### Example Response

```json
{
  "job_id": 12345,
  "transaction": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQAHDQoFBgcICQsMDQ4PEBESEw==",
  "status": "pending_signature"
}
```

### POST /api/jobs/:id/confirm

Confirms job creation after transaction is signed and submitted to Solana network.

#### Request

```http
POST /api/jobs/12345/confirm HTTP/1.1
Host: api.zyberlink.io
Content-Type: application/json

{
  "signature": "string"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `signature` | string | Yes | Solana transaction signature (base58) |

#### Response

**Success (200 OK):**

```json
{
  "status": "confirmed",
  "job_id": 12345,
  "signature": "5J7XqG3K8H9L2M4N6P1Q3R5S7T9U2V4W6X8Y1Z3A5B7C9D2E4F6G8H1J3K5L7M9N2P4Q6R8S1T3U5V7W9X2Y4Z6"
}
```

**Error (400):**

```json
{
  "error": "Invalid signature format"
}
```

**Error (404):**

```json
{
  "error": "Job not found"
}
```

### GET /api/jobs/:id

Retrieves job status and details.

#### Request

```http
GET /api/jobs/12345 HTTP/1.1
Host: api.zyberlink.io
```

#### Response

**Success (200 OK):**

```json
{
  "job_id": 12345,
  "creator": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
  "status": "completed",
  "payment_method": "wZEC",
  "payment_token_mint": "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf",
  "price_lamports": 500000000,
  "operation": "add",
  "operation_value": 5,
  "required_provers": 3,
  "consensus_threshold": 2,
  "provers_claimed": 3,
  "results_submitted": 3,
  "consensus_reached": true,
  "created_at": "2025-11-20T10:30:00Z",
  "completed_at": "2025-11-20T10:30:45Z"
}
```

**Error (404):**

```json
{
  "error": "Job not found"
}
```

## Data Types

### PaymentMethod

```typescript
type PaymentMethod = "SOL" | "wZEC";
```

### CircuitType

```typescript
type CircuitType = "add" | "multiply" | "subtract";
```

### JobStatus

```typescript
type JobStatus =
  | "pending_signature"  // Waiting for user to sign transaction
  | "pending"            // Transaction confirmed, waiting for provers
  | "claimed"            // Provers have claimed the job
  | "computing"          // Computation in progress
  | "consensus"          // Checking consensus
  | "completed"          // Job completed successfully
  | "failed"             // Job failed (timeout or consensus failure)
  | "cancelled";         // Job cancelled by creator
```

### Transaction Structure

#### SOL Payment Transaction (5 Accounts)

```
Accounts:
  0. Creator (signer, writable) - Pays SOL
  1. Job PDA (writable) - Job state account
  2. Config PDA (readonly) - Marketplace config
  3. Escrow PDA (writable) - SOL escrow
  4. System Program (readonly) - Native SOL transfers

Instruction: CreateJob
Data: CircuitType, WitnessCommitment, WitnessSize, PriceLamports, TimeoutSeconds, FheConfig
```

#### wZEC Payment Transaction (9 Accounts)

```
Accounts:
  0. Creator (signer, writable) - Signs transaction
  1. Job PDA (writable) - Job state account
  2. Config PDA (writable) - Marketplace config
  3. Token Escrow PDA (writable) - wZEC escrow (auto-created)
  4. Creator Token Account (writable) - Creator's wZEC balance
  5. Token Mint (readonly) - wZEC mint address
  6. System Program (readonly) - Account creation
  7. Token Program (readonly) - SPL Token transfers
  8. Rent Sysvar (readonly) - Rent calculations

Instruction: CreateJobWithToken
Data: CircuitType, WitnessCommitment, WitnessSize, PriceLamports, TimeoutSeconds, FheConfig
```

### Constants

```typescript
// wZEC Token Mint
const WZEC_MINT = "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf";

// Program ID
const PROGRAM_ID = "7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR";

// Token Decimals
const SOL_DECIMALS = 9;  // 1 SOL = 1,000,000,000 lamports
const WZEC_DECIMALS = 8; // 1 wZEC = 100,000,000 zatoshis

// Limits
const MAX_ENCRYPTED_DATA_SIZE = 1_048_576;     // 1 MB
const MAX_SERVER_KEY_SIZE = 209_715_200;       // 200 MB
const MAX_NONCE_LENGTH = 256;                  // chars
const SIGNATURE_EXPIRY_SECONDS = 300;          // 5 minutes
```

## Error Codes

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request successful |
| 400 | Bad Request | Invalid request parameters |
| 401 | Unauthorized | Signature verification failed |
| 404 | Not Found | Resource not found |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Service temporarily unavailable |

### Error Messages

**Validation Errors (400):**

```
"Invalid payment_method: must be 'SOL' or 'wZEC'"
"Invalid creator_pubkey: must be valid Solana public key"
"Invalid signature: must be base58 encoded Ed25519 signature"
"Invalid encrypted_data: exceeds maximum size of 1 MB"
"Invalid server_key: exceeds maximum size of 200 MB"
"Invalid operation: must be 'add', 'multiply', or 'subtract'"
"Invalid consensus_threshold: must be <= required_provers"
"Nonce already used"
"Timestamp expired (must be within 5 minutes)"
```

**Authentication Errors (401):**

```
"Signature verification failed"
"Invalid signature format"
"Signature does not match creator_pubkey"
```

**Server Errors (500):**

```
"Database error: failed to store job data"
"Transaction build failed"
"Failed to derive PDA accounts"
"RPC error: failed to get account"
```

### Error Response Format

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": {
    "field": "field_name",
    "value": "invalid_value",
    "reason": "specific reason"
  }
}
```

## Rate Limits

### Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| POST /api/jobs/validate-and-build | 10 requests | per minute |
| POST /api/jobs/:id/confirm | 20 requests | per minute |
| GET /api/jobs/:id | 100 requests | per minute |

### Rate Limit Headers

```http
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 7
X-RateLimit-Reset: 1732104060
```

### Rate Limit Error

**429 Too Many Requests:**

```json
{
  "error": "Rate limit exceeded",
  "retry_after": 45
}
```

## Examples

### JavaScript/TypeScript

```typescript
import { Connection, PublicKey, Transaction } from '@solana/web3.js';
import bs58 from 'bs58';

const WZEC_MINT = new PublicKey('7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf');
const API_URL = 'https://api.zyberlink.io';

async function createJobWithWZEC(
  wallet: any,
  encryptedData: string,
  serverKey: string,
  priceLamports: number
): Promise<{ jobId: number; signature: string }> {
  // 1. Generate nonce and message
  const jobId = Math.floor(Math.random() * 1000000);
  const timestamp = Math.floor(Date.now() / 1000);
  const nonce = `wzec_${jobId}_${timestamp}`;
  const message = `create_job:${jobId}:${timestamp}:${nonce}`;

  // 2. Sign message
  const messageBytes = new TextEncoder().encode(message);
  const signatureBytes = await wallet.signMessage(messageBytes);
  const signature = bs58.encode(signatureBytes);

  // 3. Build request
  const response = await fetch(`${API_URL}/api/jobs/validate-and-build`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      creator_pubkey: wallet.publicKey.toString(),
      encrypted_data: encryptedData,
      server_key: serverKey,
      message,
      signature,
      nonce,
      operation: 'add',
      operation_value: 5,
      price_lamports: priceLamports,
      required_provers: 3,
      consensus_threshold: 2,
      payment_method: 'wZEC'
    })
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error);
  }

  const { job_id, transaction } = await response.json();

  // 4. Sign and send transaction
  const connection = new Connection('https://api.mainnet-beta.solana.com');
  const tx = Transaction.from(Buffer.from(transaction, 'base64'));
  const signedTx = await wallet.signTransaction(tx);
  const txSignature = await connection.sendRawTransaction(signedTx.serialize());

  await connection.confirmTransaction(txSignature);

  // 5. Confirm with backend
  await fetch(`${API_URL}/api/jobs/${job_id}/confirm`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ signature: txSignature })
  });

  return { jobId: job_id, signature: txSignature };
}
```

### Python

```python
import requests
import json
import base58
from nacl.signing import SigningKey
from solders.keypair import Keypair
from solders.transaction import Transaction

WZEC_MINT = "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf"
API_URL = "https://api.zyberlink.io"

def create_job_with_wzec(
    keypair: Keypair,
    encrypted_data: str,
    server_key: str,
    price_lamports: int
) -> dict:
    # 1. Generate nonce and message
    import time
    import random
    job_id = random.randint(1, 1000000)
    timestamp = int(time.time())
    nonce = f"wzec_{job_id}_{timestamp}"
    message = f"create_job:{job_id}:{timestamp}:{nonce}"

    # 2. Sign message
    message_bytes = message.encode('utf-8')
    signature_bytes = keypair.sign_message(message_bytes)
    signature = base58.b58encode(signature_bytes).decode('ascii')

    # 3. Build request
    payload = {
        "creator_pubkey": str(keypair.pubkey()),
        "encrypted_data": encrypted_data,
        "server_key": server_key,
        "message": message,
        "signature": signature,
        "nonce": nonce,
        "operation": "add",
        "operation_value": 5,
        "price_lamports": price_lamports,
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "wZEC"
    }

    response = requests.post(
        f"{API_URL}/api/jobs/validate-and-build",
        json=payload
    )

    if not response.ok:
        error = response.json()
        raise Exception(error["error"])

    result = response.json()
    job_id = result["job_id"]
    transaction = result["transaction"]

    # 4. Sign and send transaction
    # (Implementation depends on Solana Python library)

    return {"job_id": job_id, "signature": "..."}
```

### cURL

```bash
#!/bin/bash

KEYPAIR_PATH="$HOME/.config/solana/id.json"
API_URL="https://api.zyberlink.io"
WZEC_MINT="7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf"

# 1. Get creator pubkey
CREATOR=$(solana address -k "$KEYPAIR_PATH")

# 2. Generate message
JOB_ID=$RANDOM
TIMESTAMP=$(date +%s)
NONCE="curl_${JOB_ID}_${TIMESTAMP}"
MESSAGE="create_job:${JOB_ID}:${TIMESTAMP}:${NONCE}"

# 3. Sign message (requires Python helper)
SIGNATURE=$(python3 sign-message.py "$KEYPAIR_PATH" "$MESSAGE")

# 4. Load TFHE data
ENCRYPTED_DATA=$(cat test-keys/encrypted_data.b64)
SERVER_KEY=$(cat test-keys/server_key.b64)

# 5. Build JSON request
REQUEST=$(cat <<EOF
{
  "creator_pubkey": "$CREATOR",
  "encrypted_data": "$ENCRYPTED_DATA",
  "server_key": "$SERVER_KEY",
  "message": "$MESSAGE",
  "signature": "$SIGNATURE",
  "nonce": "$NONCE",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 500000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "wZEC"
}
EOF
)

# 6. Send request
RESPONSE=$(curl -s -X POST \
  "$API_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d "$REQUEST")

echo "$RESPONSE" | jq '.'
```

## SDKs

### Official SDKs

**JavaScript/TypeScript:**
```bash
npm install @zyberlink/sdk
```

**Rust:**
```toml
[dependencies]
zyberlink-sdk = "0.1.0"
```

**Python:**
```bash
pip install zyberlink-sdk
```

### Community SDKs

- **Go**: [zyberlink-go](https://github.com/zyberlink/zyberlink-go)
- **Ruby**: [zyberlink-ruby](https://github.com/zyberlink/zyberlink-ruby)
- **Java**: [zyberlink-java](https://github.com/zyberlink/zyberlink-java)

## Changelog

### Version 1.0.0 (2025-11-20)

**Added:**
- wZEC payment support via `payment_method` field
- `CreateJobWithToken` instruction for SPL token payments
- Automatic token account creation
- Token escrow PDA for wZEC payments

**Changed:**
- `payment_method` field is now optional (defaults to "SOL")
- Transaction structure varies based on payment method

**Fixed:**
- Signature format clarified: base58 (not base64)
- Token account creation now automatic

## Support

API support:

- **Documentation**: [docs.zyberlink.io](https://docs.zyberlink.io)
- **GitHub**: [Issues](https://github.com/zyberlink/zyberlink/issues)
- **Discord**: [#api-support](https://discord.gg/zyberlink)
- **Email**: api@zyberlink.io
- **Status**: [status.zyberlink.io](https://status.zyberlink.io)

---

**Last Updated**: 2025-11-21
**API Version**: 1.0.0
**wZEC Mint**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
