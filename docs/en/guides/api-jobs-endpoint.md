# GET /api/jobs - Job Listing Endpoint

## Overview

Endpoint for listing jobs in the ZyberLink marketplace with optional status filtering.

**URL:** `GET /api/jobs`

**Purpose:** Allows web clients to view and monitor jobs in the marketplace dynamically.

---

## Query Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `status` | string | No | Filter jobs by status. Valid values: `pending_tx`, `active`, `completed`, `failed` |

---

## Response Format

```json
{
  "jobs": [
    {
      "job_id": 123,
      "creator_pubkey": "FdMVQxVLxGhYd8hyBE5hKoCVzYAuXioTeMV2u1VP1Wcj",
      "operation": "add",
      "operation_value": 10,
      "price_lamports": 3000000,
      "required_provers": 3,
      "consensus_threshold": 2,
      "status": "active",
      "payment_method": "SOL",
      "created_at": "2025-11-22T22:00:00Z"
    }
  ],
  "count": 1
}
```

---

## Examples

### 1. List all jobs

```bash
curl http://127.0.0.1:8080/api/jobs
```

**Response:**
```json
{
  "jobs": [...],
  "count": 42
}
```

### 2. List only active jobs

```bash
curl "http://127.0.0.1:8080/api/jobs?status=active"
```

**Response:**
```json
{
  "jobs": [
    {
      "job_id": 123,
      "status": "active",
      ...
    }
  ],
  "count": 5
}
```

### 3. List pending jobs (awaiting blockchain confirmation)

```bash
curl "http://127.0.0.1:8080/api/jobs?status=pending_tx"
```

### 4. List completed jobs

```bash
curl "http://127.0.0.1:8080/api/jobs?status=completed"
```

### 5. Invalid status (returns 400 error)

```bash
curl "http://127.0.0.1:8080/api/jobs?status=invalid"
```

**Response:**
```json
{
  "error": "Invalid status: invalid. Valid values: pending_tx, active, completed, failed"
}
```

---

## Job Status Values

| Status | Description |
|--------|-------------|
| `pending_tx` | Job created in backend, awaiting blockchain transaction confirmation |
| `active` | Job confirmed on-chain, ready for provers to claim and compute |
| `completed` | Job computation finished, results submitted |
| `failed` | Job failed (timeout, invalid proof, etc.) |

---

## Use Cases

### 1. Web UI Dashboard
Display live marketplace activity:
```javascript
fetch('http://127.0.0.1:8080/api/jobs?status=active')
  .then(res => res.json())
  .then(data => {
    console.log(`${data.count} active jobs in marketplace`);
    data.jobs.forEach(job => {
      console.log(`Job ${job.job_id}: ${job.operation} - ${job.price_lamports} lamports`);
    });
  });
```

### 2. Monitor Job Progress
Poll for status changes:
```bash
# Watch jobs transitioning to completed
watch -n 5 'curl -s "http://127.0.0.1:8080/api/jobs?status=completed" | jq ".count"'
```

### 3. Analytics
Calculate marketplace metrics:
```bash
# Total jobs across all statuses
curl -s http://127.0.0.1:8080/api/jobs | jq ".count"

# Jobs by status
for status in pending_tx active completed failed; do
  count=$(curl -s "http://127.0.0.1:8080/api/jobs?status=$status" | jq ".count")
  echo "$status: $count"
done
```

---

## Architecture Notes

### Why this endpoint?

While **provers discover jobs directly on-chain** via `find_pending_jobs()` (source of truth = blockchain), this endpoint provides:

1. **Web UI/UX** - Display marketplace dynamically
2. **Monitoring** - Track system health and activity
3. **Analytics** - Gather insights on job patterns
4. **Debugging** - Inspect job states during development

### Provers don't need this endpoint

Provers use the SDK's `find_pending_jobs()` which queries Solana directly:

```rust
// Provers use this (on-chain discovery)
let jobs = find_pending_jobs(&rpc_client, &program_id)?;
```

This endpoint is purely for **web clients** and **monitoring tools**.

---

## Performance Considerations

- **Database Query:** Fetches from PostgreSQL `temp_job_data` table
- **No pagination:** Currently returns all jobs for the given status
- **Future improvement:** Add pagination for large result sets (`?limit=100&offset=0`)

---

## Error Responses

### 400 Bad Request - Invalid Status
```json
{
  "error": "Invalid status: xyz. Valid values: pending_tx, active, completed, failed"
}
```

### 500 Internal Server Error - Database Issue
```json
{
  "error": "Database error: connection failed"
}
```

---

## Related Endpoints

- `POST /api/jobs/validate-and-build` - Create a new job
- `GET /api/jobs/{job_id}/status` - Get specific job status
- `GET /api/jobs/{job_id}/compute-data` - Get FHE compute data for provers
- `POST /api/jobs/{job_id}/confirm` - Confirm job transaction
- `DELETE /api/jobs/{job_id}` - Delete completed/failed job

---

## Implementation Details

**File:** `blink-server/src/api_handlers.rs:377`

**Database Query:** Uses `JobQueries::get_jobs_by_status()` which queries:
```sql
SELECT * FROM temp_job_data WHERE status = $1 ORDER BY created_at DESC
```

**Response Format:** Converts database `TempJobData` models to `JobListItem` DTOs.

---

## Testing

```bash
# Start backend
cd blink-server
cargo run --release

# In another terminal
curl http://127.0.0.1:8080/api/jobs | jq .

# Test with filters
curl "http://127.0.0.1:8080/api/jobs?status=active" | jq .
```

---

## Changelog

### 2025-11-22
- ✨ Initial implementation of `GET /api/jobs` endpoint
- ✅ Optional `status` query parameter with validation
- ✅ Returns job list with count
- ✅ Error handling for invalid status values
