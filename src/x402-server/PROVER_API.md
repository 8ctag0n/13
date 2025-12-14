# Prover API Documentation

## Overview

The x402-server provides public endpoints for authenticated provers to interact with the ZK/FHE job system. Provers authenticate using Solana ed25519 signatures.

## Authentication

### Headers Required

All prover endpoints require the following headers:

```
X-Prover-Pubkey: <base58 encoded Solana pubkey>
X-Prover-Signature: <base58 encoded ed25519 signature>
X-Timestamp: <unix timestamp in seconds>
```

### Signature Generation

The message to sign is constructed as:

```
"{METHOD}:{PATH}:{TIMESTAMP}:{BODY_HASH}"
```

Where:
- `METHOD` = HTTP method (GET, POST, etc.)
- `PATH` = Request path (e.g., `/gateway/prover/witness/abc123`)
- `TIMESTAMP` = Unix timestamp from `X-Timestamp` header
- `BODY_HASH` = Hex-encoded Blake2s-256 hash of request body (empty string for GET)

### Example

```rust
use blake2::{Blake2s256, Digest};
use solana_sdk::signature::{Keypair, Signer};

let keypair = Keypair::new();
let method = "POST";
let path = "/gateway/prover/zk/123/submit";
let timestamp = chrono::Utc::now().timestamp();
let body = br#"{"proof":"...","public_inputs":["0x1"]}"#;

// Hash body
let mut hasher = Blake2s256::new();
hasher.update(body);
let body_hash = hex::encode(hasher.finalize());

// Build and sign message
let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
let signature = keypair.sign_message(message.as_bytes());

// Use in HTTP headers
// X-Prover-Pubkey: {keypair.pubkey()}
// X-Prover-Signature: {signature}
// X-Timestamp: {timestamp}
```

### Security Notes

- Timestamps must be within ±5 minutes of server time
- Each signature is single-use (replay protection via timestamp)
- Signatures are cryptographically bound to the exact request (method, path, body)

## Endpoints

### GET /gateway/prover/witness/{hash}

Download witness data for a claimed job.

**Path Parameters:**
- `hash` - Blake2s-256 hash of the witness data (hex encoded)

**Response:**
- Content-Type: `application/octet-stream`
- Body: Raw witness binary data

**Example:**

```bash
curl -X GET \
  -H "X-Prover-Pubkey: ${PROVER_PUBKEY}" \
  -H "X-Prover-Signature: ${SIGNATURE}" \
  -H "X-Timestamp: ${TIMESTAMP}" \
  http://localhost:8081/gateway/prover/witness/abc123...
```

**Error Responses:**
- `401 Unauthorized` - Invalid signature or missing headers
- `404 Not Found` - Witness not found
- `504 Gateway Timeout` - Blink server unavailable

---

### POST /gateway/prover/zk/{job_id}/submit

Submit a verified ZK proof for a job.

**Path Parameters:**
- `job_id` - Job ID (integer)

**Request Body:**
```json
{
  "proof": "base64_encoded_proof_data",
  "public_inputs": ["0x1", "0x2", "0x3"]
}
```

**Response:**
```json
{
  "success": true,
  "job_id": 123,
  "prover": "ProverPubkey..."
}
```

**Example:**

```bash
curl -X POST \
  -H "X-Prover-Pubkey: ${PROVER_PUBKEY}" \
  -H "X-Prover-Signature: ${SIGNATURE}" \
  -H "X-Timestamp: ${TIMESTAMP}" \
  -H "Content-Type: application/json" \
  -d '{"proof":"YmFzZTY0...","public_inputs":["0x1","0x2"]}' \
  http://localhost:8081/gateway/prover/zk/123/submit
```

**Error Responses:**
- `400 Bad Request` - Invalid proof data or empty proof
- `401 Unauthorized` - Invalid signature
- `404 Not Found` - Job not found
- `504 Gateway Timeout` - Blink server unavailable

---

### POST /gateway/prover/fhe/{job_id}/submit

Submit FHE computation result for a job.

**Path Parameters:**
- `job_id` - Job ID (integer)

**Request Body:**
- Content-Type: `application/octet-stream`
- Binary encrypted result data

**Response:**
```json
{
  "success": true,
  "job_id": 456,
  "prover": "ProverPubkey...",
  "result_size": 2048
}
```

**Example:**

```bash
curl -X POST \
  -H "X-Prover-Pubkey: ${PROVER_PUBKEY}" \
  -H "X-Prover-Signature: ${SIGNATURE}" \
  -H "X-Timestamp: ${TIMESTAMP}" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @result.bin \
  http://localhost:8081/gateway/prover/fhe/456/submit
```

**Error Responses:**
- `400 Bad Request` - Empty result data
- `401 Unauthorized` - Invalid signature
- `404 Not Found` - Job not found
- `504 Gateway Timeout` - Blink server unavailable

## Backend Integration

The x402-server proxies authenticated prover requests to blink-server internal endpoints:

```
GET  /gateway/prover/witness/{hash}       → GET  /internal/witness/{hash}
POST /gateway/prover/zk/{id}/submit       → POST /internal/zk/{id}/submit-proof
POST /gateway/prover/fhe/{id}/submit      → POST /internal/fhe-result?job_id={}&prover={}
```

The `X-Prover-Pubkey` header is forwarded to blink-server for authorization checks.

## Rate Limiting

Prover endpoints are not rate-limited at the x402 layer, but blink-server may enforce:
- Per-prover rate limits
- Job claim validation
- Proof verification requirements

## Testing

Run the authentication example:

```bash
cd src/x402-server
cargo run --example prover_auth_example
```

This shows how to construct and sign requests for all three endpoints.

## Client Libraries

### Rust

```rust
use reqwest::Client;
use solana_sdk::signature::{Keypair, Signer};
use blake2::{Blake2s256, Digest};

async fn download_witness(
    client: &Client,
    keypair: &Keypair,
    hash: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let method = "GET";
    let path = format!("/gateway/prover/witness/{}", hash);
    let timestamp = chrono::Utc::now().timestamp();

    // Build signature
    let message = format!("{}:{}:{}:", method, path, timestamp);
    let signature = keypair.sign_message(message.as_bytes());

    let response = client
        .get(format!("http://localhost:8081{}", path))
        .header("X-Prover-Pubkey", keypair.pubkey().to_string())
        .header("X-Prover-Signature", signature.to_string())
        .header("X-Timestamp", timestamp.to_string())
        .send()
        .await?;

    Ok(response.bytes().await?.to_vec())
}
```

### TypeScript/JavaScript

```typescript
import { Keypair } from '@solana/web3.js';
import { blake2s } from 'blake2';
import bs58 from 'bs58';

async function downloadWitness(
  keypair: Keypair,
  hash: string
): Promise<Uint8Array> {
  const method = 'GET';
  const path = `/gateway/prover/witness/${hash}`;
  const timestamp = Math.floor(Date.now() / 1000);

  // Build message
  const message = `${method}:${path}:${timestamp}:`;
  const signature = await keypair.sign(Buffer.from(message));

  const response = await fetch(`http://localhost:8081${path}`, {
    method: 'GET',
    headers: {
      'X-Prover-Pubkey': keypair.publicKey.toBase58(),
      'X-Prover-Signature': bs58.encode(signature),
      'X-Timestamp': timestamp.toString(),
    },
  });

  return new Uint8Array(await response.arrayBuffer());
}
```

## Production Deployment

### Environment Variables

```bash
X402_HOST=0.0.0.0
X402_PORT=8081
BLINK_URL=http://blink-server:8080
DATABASE_URL=postgresql://user:pass@db:5432/x402
```

### Network Architecture

```
Internet
   |
   v
x402-server:8081 (public)
   |
   | (internal network)
   v
blink-server:8080 (private)
```

### Security Considerations

1. x402-server is the only public-facing service
2. blink-server should only accept connections from x402-server
3. Use firewall rules to restrict access
4. Enable HTTPS/TLS in production
5. Consider adding DDoS protection at load balancer level

## Monitoring

Log entries include:
- Prover pubkey for all authenticated requests
- Job IDs for submissions
- Data sizes for downloads/uploads
- Error conditions and backend failures

Example log output:
```
INFO Prover 5Prover... requesting witness: abc123...
INFO Witness downloaded: 2048 bytes for hash abc123...
INFO Prover 5Prover... submitting ZK proof for job 123
INFO ZK proof submitted successfully for job 123
```
