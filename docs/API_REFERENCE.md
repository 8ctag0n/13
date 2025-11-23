# ZyberLink API Reference

**Complete REST API Documentation for ZyberLink Marketplace**

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Base URLs](#base-urls)
4. [Job Management Endpoints](#job-management-endpoints)
5. [Pricing Endpoints](#pricing-endpoints)
6. [Error Handling](#error-handling)
7. [Rate Limits](#rate-limits)
8. [Examples](#examples)

---

## Overview

The ZyberLink API provides RESTful endpoints for interacting with the decentralized compute marketplace. The API handles:

- Job validation and transaction building
- Cost estimation for FHE operations
- Job status tracking
- Compute data delivery for provers

**API Version:** v1
**Content-Type:** application/json
**Protocol:** HTTPS (production), HTTP (development)

---

## Authentication

Currently, the API does not require authentication for read-only endpoints. Write operations (job creation) require on-chain signatures via Solana transactions.

**Future:** OAuth2 or API key authentication may be added for rate limiting and analytics.

---

## Base URLs

| Environment | URL |
|-------------|-----|
| Local Development | `http://localhost:8080` |
| Testnet | `https://api-testnet.zyberlink.io` |
| Mainnet | `https://api.zyberlink.io` |

---

## Job Management Endpoints

### POST /api/jobs/validate-and-build

**Description:** Validates job data and builds an unsigned Solana transaction for job creation.

**Request Body:**
```json
{
  "creator_pubkey": "ABC123...",
  "encrypted_data": "base64_encoded_data",
  "server_key": "base64_encoded_key",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 3000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "SOL",
  "payment_token_mint": null,
  "signature": "signature_hex"
}
```

**Request Schema:**
```typescript
interface ValidateJobRequest {
  creator_pubkey: string;          // Base58 Solana pubkey
  encrypted_data: string;          // Base64 encoded ciphertext
  server_key: string;              // Base64 encoded FHE server key
  operation: string;               // "add" | "multiply" | "sum" | etc.
  operation_value: number;         // Operation parameter
  price_lamports: number;          // Total payment in lamports
  required_provers: number;        // 2-10 provers
  consensus_threshold: number;     // Min matching results
  payment_method: string;          // "SOL" | "wZEC"
  payment_token_mint?: string;     // Token mint if not SOL
  signature: string;               // Ed25519 signature
}
```

**Response (Success):**
```json
{
  "job_id": 12345,
  "transaction": "base64_encoded_transaction",
  "status": "pending_signature"
}
```

**Response Schema:**
```typescript
interface ValidateJobResponse {
  job_id: number;
  transaction: string;  // Base64 serialized Transaction
  status: string;       // "pending_signature"
}
```

**Status Codes:**
- `200 OK` - Job validated and transaction built
- `400 Bad Request` - Validation failed (see error message)
- `500 Internal Server Error` - Database or serialization error

**Example:**
```bash
curl -X POST http://localhost:8080/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": 3000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234..."
  }'
```

---

### GET /api/jobs/{job_id}/compute-data

**Description:** Retrieves encrypted compute data for provers. Only returns data if job status is "active".

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "job_id": 12345,
  "encrypted_data": "base64_encoded_data",
  "server_key": "base64_encoded_key",
  "operation": "add",
  "operation_value": 5
}
```

**Response Schema:**
```typescript
interface ComputeDataResponse {
  job_id: number;
  encrypted_data: string;  // Base64
  server_key: string;      // Base64
  operation: string;
  operation_value: number;
}
```

**Status Codes:**
- `200 OK` - Compute data returned
- `400 Bad Request` - Job not in "active" status
- `404 Not Found` - Job doesn't exist
- `500 Internal Server Error` - Database error

**Example:**
```bash
curl -X GET http://localhost:8080/api/jobs/12345/compute-data
```

---

### POST /api/jobs/{job_id}/confirm

**Description:** Confirms that a job transaction was successfully submitted on-chain. Updates job status from "pending_tx" to "active".

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Request Body:**
```json
{
  "signature": "transaction_signature_base58"
}
```

**Request Schema:**
```typescript
interface ConfirmJobRequest {
  signature: string;  // Transaction signature
}
```

**Response (Success):**
```json
{
  "job_id": 12345,
  "status": "active",
  "message": "Job confirmed and ready for provers"
}
```

**Status Codes:**
- `200 OK` - Job confirmed
- `404 Not Found` - Job doesn't exist
- `500 Internal Server Error` - Database update failed

**Example:**
```bash
curl -X POST http://localhost:8080/api/jobs/12345/confirm \
  -H "Content-Type: application/json" \
  -d '{"signature": "5J7Zx..."}'
```

**Note:** In production, this endpoint should verify the transaction signature on-chain before updating status.

---

### GET /api/jobs/{job_id}/status

**Description:** Get current status of a job.

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "job_id": 12345,
  "status": "active",
  "created_at": "2025-11-21T18:00:00Z"
}
```

**Response Schema:**
```typescript
interface JobStatusResponse {
  job_id: number;
  status: string;      // "pending_tx" | "active" | "completed" | "failed"
  created_at: string;  // ISO 8601 timestamp
}
```

**Status Codes:**
- `200 OK` - Status returned
- `404 Not Found` - Job doesn't exist
- `500 Internal Server Error` - Database error

**Example:**
```bash
curl -X GET http://localhost:8080/api/jobs/12345/status
```

---

### DELETE /api/jobs/{job_id}

**Description:** Delete job data. Only allowed if job is in terminal state (completed/failed).

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "message": "Job deleted successfully"
}
```

**Status Codes:**
- `200 OK` - Job deleted
- `400 Bad Request` - Job not in terminal state
- `404 Not Found` - Job doesn't exist
- `500 Internal Server Error` - Database error

**Example:**
```bash
curl -X DELETE http://localhost:8080/api/jobs/12345
```

---

## Pricing Endpoints

### POST /api/estimate-cost

**Description:** Estimate the cost and timeout for a given FHE operation. This is the **primary endpoint** for dynamic pricing integration.

**Request Body:**
```json
{
  "operation": "histogram",
  "operation_value": 0,
  "expected_count": null,
  "bins": 10,
  "required_provers": 3
}
```

**Request Schema:**
```typescript
interface EstimateCostRequest {
  operation: string;           // FHE operation name
  operation_value: number;     // Constant value (for Add, Multiply, Threshold)
  expected_count?: number;     // Item count (for Sum, Average, CountIf)
  bins?: number;               // Bin count (for Histogram)
  required_provers: number;    // 2-10 provers
}
```

**Supported Operations:**

| Operation | Required Fields | Optional Fields | Description |
|-----------|----------------|-----------------|-------------|
| `add` | `operation_value` | - | Add constant to encrypted value |
| `multiply` | `operation_value` | - | Multiply encrypted value by constant |
| `sum` | - | `expected_count` | Sum multiple encrypted values |
| `threshold` | `operation_value` | - | Check if value >= threshold |
| `range_check` | `operation_value` | - | Check if value in range [0, value] |
| `average` | - | `expected_count` | Average of encrypted values |
| `count_if` | `operation_value` | `expected_count` | Count values matching predicate |
| `histogram` | - | `bins` | Distribution across bins |

**Response (Success):**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 732050808,
  "min_payment_sol": 0.732050808,
  "total_min_payment_lamports": 2196152424,
  "total_min_payment_sol": 2.196152424,
  "timeout_seconds": 1600,
  "estimated_compute_ms": 30500
}
```

**Response Schema:**
```typescript
interface EstimateCostResponse {
  operation: string;                    // Operation name
  complexity_tier: number;              // 1-5 tier
  min_payment_lamports: number;         // Cost per prover (lamports)
  min_payment_sol: number;              // Cost per prover (SOL)
  total_min_payment_lamports: number;   // Total cost (lamports)
  total_min_payment_sol: number;        // Total cost (SOL)
  timeout_seconds: number;              // Dynamic timeout
  estimated_compute_ms: number;         // Estimated execution time
}
```

**Complexity Tiers:**

| Tier | Complexity | Base Cost/Prover | Example Operations |
|------|------------|------------------|-------------------|
| 1 | O(1) | 0.001 SOL | Add, Multiply |
| 2 | O(n) | 0.001 + n×0.0001 SOL | Sum |
| 3 | O(1) + bootstrap | 0.005 SOL | Threshold, RangeCheck |
| 4 | O(n) + predicates | Variable | Average, CountIf |
| 5 | O(n×m) | 0.1 + 0.02×m^1.5 SOL | Histogram |

**Status Codes:**
- `200 OK` - Cost estimate returned
- `400 Bad Request` - Invalid operation or parameters
- `500 Internal Server Error` - Calculation error

**Example 1: Simple Addition (Tier 1)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 5,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Add",
  "complexity_tier": 1,
  "min_payment_lamports": 1000000,
  "min_payment_sol": 0.001,
  "total_min_payment_lamports": 3000000,
  "total_min_payment_sol": 0.003,
  "timeout_seconds": 60,
  "estimated_compute_ms": 150
}
```

**Example 2: Age Verification (Tier 3)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Threshold",
  "complexity_tier": 3,
  "min_payment_lamports": 5000000,
  "min_payment_sol": 0.005,
  "total_min_payment_lamports": 15000000,
  "total_min_payment_sol": 0.015,
  "timeout_seconds": 300,
  "estimated_compute_ms": 300
}
```

**Example 3: Census Sum (Tier 2 - Scales with Count)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "sum",
    "expected_count": 10000,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Sum",
  "complexity_tier": 2,
  "min_payment_lamports": 1001000000,
  "min_payment_sol": 1.001,
  "total_min_payment_lamports": 3003000000,
  "total_min_payment_sol": 3.003,
  "timeout_seconds": 20060,
  "estimated_compute_ms": 500150
}
```

**Example 4: Voting Histogram (Tier 5)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 5,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 324193547,
  "min_payment_sol": 0.324193547,
  "total_min_payment_lamports": 972580641,
  "total_min_payment_sol": 0.972580641,
  "timeout_seconds": 1100,
  "estimated_compute_ms": 15500
}
```

**Integration Notes:**

1. **Always estimate before job creation** to prevent on-chain rejections
2. **Cache estimates** for 1 minute to reduce API calls
3. **Handle parameter changes** - re-estimate when user modifies provers/bins/counts
4. **Display warnings** for Tier 5 operations (> 0.5 SOL)
5. **Validate inputs** before sending request (operation name, value ranges)

**TypeScript Integration:**
```typescript
async function estimateAndCreateJob(params: JobParams) {
  // 1. Estimate cost
  const estimate = await fetch('/api/estimate-cost', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      operation: params.operation,
      operation_value: params.value,
      bins: params.bins,
      required_provers: params.provers,
    }),
  }).then(r => r.json());

  // 2. Warn user if expensive
  if (estimate.total_min_payment_sol > 0.5) {
    if (!confirm(`This will cost ${estimate.total_min_payment_sol} SOL. Continue?`)) {
      return;
    }
  }

  // 3. Create job with validated price
  await createJob({
    ...params,
    price_lamports: estimate.total_min_payment_lamports,
    timeout: estimate.timeout_seconds,
  });
}
```

---

## Error Handling

### Error Response Format

All error responses follow this format:

```json
{
  "error": "Human-readable error message"
}
```

### Common Error Codes

| Status Code | Meaning | Common Causes |
|-------------|---------|---------------|
| 400 | Bad Request | Invalid parameters, validation failure, underpriced job |
| 404 | Not Found | Job ID doesn't exist |
| 500 | Internal Server Error | Database error, serialization failure |
| 503 | Service Unavailable | Database connection lost, RPC node down |

### Example Error Responses

**Validation Error:**
```json
{
  "error": "Validation failed: Invalid signature"
}
```

**Price Too Low:**
```json
{
  "error": "Price too low for operation 'Histogram' (tier 5): 100000000 < 972580641 lamports"
}
```

**Unknown Operation:**
```json
{
  "error": "Unknown operation: substract"
}
```

**Job Not Active:**
```json
{
  "error": "Job not ready, status: pending_tx"
}
```

---

## Rate Limits

**Current:** No rate limits enforced

**Future:** Planned rate limits:
- 100 requests/minute per IP (cost estimation)
- 10 job creations/minute per creator pubkey
- 1000 status checks/minute per IP

Rate limit headers will be added:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1732213200
```

---

## Examples

### Complete Job Creation Flow

```bash
#!/bin/bash

# Step 1: Estimate cost
ESTIMATE=$(curl -s -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 5,
    "required_provers": 3
  }')

echo "Cost estimate: $ESTIMATE"

PRICE=$(echo $ESTIMATE | jq -r '.total_min_payment_lamports')
TIMEOUT=$(echo $ESTIMATE | jq -r '.timeout_seconds')

# Step 2: Validate and build transaction
VALIDATE=$(curl -s -X POST http://localhost:8080/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": '$PRICE',
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234"
  }')

echo "Validation response: $VALIDATE"

JOB_ID=$(echo $VALIDATE | jq -r '.job_id')
TX_BASE64=$(echo $VALIDATE | jq -r '.transaction')

# Step 3: Sign transaction (using Solana CLI)
echo $TX_BASE64 | base64 -d > unsigned_tx.bin
solana sign-transaction unsigned_tx.bin > signed_tx.sig

# Step 4: Submit transaction
# (In real scenario, use solana-cli or web3.js)

# Step 5: Confirm transaction
curl -X POST http://localhost:8080/api/jobs/$JOB_ID/confirm \
  -H "Content-Type: application/json" \
  -d '{"signature": "5J7Zx..."}'

# Step 6: Check status
curl -X GET http://localhost:8080/api/jobs/$JOB_ID/status

# Step 7: Provers fetch compute data
curl -X GET http://localhost:8080/api/jobs/$JOB_ID/compute-data
```

### Python Client Example

```python
import requests

class ZyberLinkClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url

    def estimate_cost(self, operation, **params):
        """Estimate cost for FHE operation."""
        response = requests.post(
            f"{self.base_url}/api/estimate-cost",
            json={"operation": operation, **params}
        )
        response.raise_for_status()
        return response.json()

    def create_job(self, job_data):
        """Validate and build job transaction."""
        response = requests.post(
            f"{self.base_url}/api/jobs/validate-and-build",
            json=job_data
        )
        response.raise_for_status()
        return response.json()

    def confirm_job(self, job_id, signature):
        """Confirm job transaction."""
        response = requests.post(
            f"{self.base_url}/api/jobs/{job_id}/confirm",
            json={"signature": signature}
        )
        response.raise_for_status()
        return response.json()

    def get_job_status(self, job_id):
        """Get job status."""
        response = requests.get(
            f"{self.base_url}/api/jobs/{job_id}/status"
        )
        response.raise_for_status()
        return response.json()

# Usage
client = ZyberLinkClient()

# Estimate
estimate = client.estimate_cost("add", operation_value=5, required_provers=3)
print(f"Cost: {estimate['total_min_payment_sol']} SOL")

# Create job
job = client.create_job({
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": estimate["total_min_payment_lamports"],
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234"
})

print(f"Job ID: {job['job_id']}")
```

---

## Related Documentation

- **[Dynamic Pricing Overview](./DYNAMIC_PRICING.md)** - System overview and pricing tiers
- **[Technical Implementation](./DYNAMIC_PRICING_TECHNICAL.md)** - Algorithm deep dive
- **[Usage Guide](./DYNAMIC_PRICING_USAGE.md)** - Integration examples
- **[wZEC Payment Guide](./WZEC_API.md)** - Token payment documentation

---

## Changelog

### Version 1.0.0 (2025-11-21)
- Initial API documentation
- Added `/api/estimate-cost` endpoint
- Added job management endpoints
- Added error handling documentation
- Added complete examples

---

**Questions or Issues?**
- GitHub: [github.com/yourorg/zyberlink](https://github.com/yourorg/zyberlink)
- Discord: [discord.gg/zyberlink](https://discord.gg/zyberlink)

---

**Last Updated:** 2025-11-21
**Version:** 1.0.0
**Maintained By:** ZyberLink Core Team
