# ZyberLink Marketplace Backend

Backend server for the ZyberLink FHE marketplace with hybrid Web2/Web3 architecture.

## Architecture

**Flow: Backend-First (Validate-Then-Submit)**

```
User → Backend (validate + store) → Build unsigned TX → User signs → Submit to Solana → Provers fetch from backend
```

This approach validates data **before** paying gas, preventing wasteful on-chain transactions.

## Features

- Job validation (signature, size, format, nonce)
- PostgreSQL storage for temporary job data
- Anti-replay attack protection (nonce tracking)
- Automatic cleanup of expired data
- REST API for job creation and prover queries
- Legacy Solana Blinks support

## Prerequisites

- Rust 1.70+
- PostgreSQL 15
- Docker & Docker Compose (optional but recommended)

## Quick Start

### 1. Start PostgreSQL

```bash
# Using Docker Compose
docker-compose up -d

# Or use your own PostgreSQL instance
# Update DATABASE_URL in .env accordingly
```

### 2. Configure Environment

Copy and edit `.env` if needed:

```bash
# Default values are set, edit if needed
cat .env
```

### 3. Build and Run

```bash
# Build
cargo build -p zyberlink-blink-server

# Run
cargo run -p zyberlink-blink-server

# Or with custom port
PORT=3000 cargo run -p zyberlink-blink-server
```

The server will:
- Connect to PostgreSQL
- Run migrations automatically
- Start cleanup background job
- Listen on configured port (default: 3000)

## API Endpoints

### Health Check

```bash
GET /health
```

### Job Creation Flow

#### 1. Validate and Build Transaction

```bash
POST /api/jobs/validate-and-build
Content-Type: application/json

{
  "creator_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "encrypted_data": "base64_encoded_fhe_ciphertext",
  "server_key": "base64_encoded_fhe_server_key",
  "message": "create_job:1:1737213600:nonce123",
  "signature": "base64_encoded_ed25519_signature",
  "nonce": "nonce123",
  "operation": "add",
  "operation_value": 10,
  "price_lamports": 9000000000,
  "required_provers": 3,
  "consensus_threshold": 2
}
```

Response:
```json
{
  "job_id": 1,
  "transaction": "base64_serialized_unsigned_transaction",
  "status": "pending_signature"
}
```

#### 2. Confirm Transaction (after user signs and submits)

```bash
POST /api/jobs/{job_id}/confirm
Content-Type: application/json

{
  "signature": "solana_transaction_signature"
}
```

### Prover Endpoints

#### Get Compute Data

```bash
GET /api/jobs/{job_id}/compute-data
```

Returns encrypted data and server key for FHE computation (only if job status is "active").

#### Get Job Status

```bash
GET /api/jobs/{job_id}/status
```

### Cleanup

#### Delete Job Data

```bash
DELETE /api/jobs/{job_id}
```

Only allowed for completed/failed jobs.

## Database Schema

### temp_job_data

Stores temporary job data before and after on-chain submission.

```sql
CREATE TABLE temp_job_data (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL UNIQUE,
    creator_pubkey TEXT NOT NULL,
    encrypted_data BYTEA NOT NULL,
    server_key BYTEA NOT NULL,
    operation TEXT NOT NULL,
    operation_value SMALLINT NOT NULL,
    price_lamports BIGINT NOT NULL,
    required_provers SMALLINT NOT NULL,
    consensus_threshold SMALLINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_tx',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL DEFAULT NOW() + INTERVAL '24 hours'
);
```

**Status values:**
- `pending_tx` - Data validated, waiting for on-chain submission
- `active` - Job confirmed on-chain, ready for provers
- `claimed` - Prover claimed the job
- `computing` - Prover is computing
- `completed` - Job finished successfully
- `failed` - Job failed

### used_nonces

Tracks used nonces for anti-replay attack protection.

```sql
CREATE TABLE used_nonces (
    nonce TEXT PRIMARY KEY,
    used_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

## Validation Rules

### Signature Validation
- Message format: `create_job:{job_id}:{timestamp}:{nonce}`
- ED25519 signature verification
- Timestamp must be within 5 minutes

### Size Limits
- Encrypted data: max 10 KB
- Server key: 40 MB - 120 MB

### Nonce Validation
- Must be unique (not previously used)
- Max length: 128 characters

### Consensus Config
- required_provers > 0
- consensus_threshold > 0
- consensus_threshold <= required_provers

## Configuration

Environment variables (see `.env`):

```bash
# Database
DATABASE_URL=postgresql://zyberlink:dev_password@localhost:5432/zyberlink

# Solana
SOLANA_RPC_URL=http://localhost:8899
PROGRAM_ID=ZyberLinkProgram11111111111111111111111111

# Server
HOST=0.0.0.0
PORT=3000

# Background Tasks
CLEANUP_INTERVAL_SECS=3600  # Cleanup expired jobs every hour

# Logging
RUST_LOG=info
```

## Development

### Run Tests

```bash
cargo test -p zyberlink-blink-server
```

### Check Compilation

```bash
cargo check -p zyberlink-blink-server
```

### Run with Logs

```bash
RUST_LOG=debug cargo run -p zyberlink-blink-server
```

## Project Structure

```
blink-server/
├── src/
│   ├── main.rs              # Server entry point
│   ├── api_handlers.rs      # REST API endpoints
│   ├── validators.rs        # Job validation logic
│   ├── cleanup.rs           # Background cleanup service
│   ├── db/
│   │   ├── mod.rs          # Database module
│   │   ├── models.rs       # Data models
│   │   ├── queries.rs      # SQL queries
│   │   └── connection.rs   # Connection pool
│   ├── actions.rs          # Legacy Blinks (Solana Actions)
│   └── tx_builder.rs       # Transaction helpers
├── migrations/
│   └── 20250118_001_create_temp_job_data.sql
├── Cargo.toml
└── README.md
```

## Troubleshooting

### Database Connection Failed

```bash
# Check if PostgreSQL is running
docker-compose ps

# Check logs
docker-compose logs postgres

# Restart
docker-compose restart postgres
```

### Migration Errors

Migrations run automatically on startup. If they fail, check:

```bash
# Connect to database
psql postgresql://zyberlink:dev_password@localhost:5432/zyberlink

# Check existing tables
\dt

# Drop tables if needed (CAUTION: deletes data)
DROP TABLE temp_job_data;
DROP TABLE used_nonces;
```

### Port Already in Use

```bash
# Change port in .env
PORT=3001

# Or kill process using port
lsof -ti:3000 | xargs kill
```

## Security Considerations

For production deployment:

1. Enable HTTPS/TLS
2. Implement rate limiting
3. Add authentication for admin endpoints
4. Verify transaction signatures on-chain (not just trust client)
5. Use secure database credentials
6. Enable database connection encryption
7. Add monitoring and alerting
8. Implement proper error handling without leaking sensitive info

## License

Same as ZyberLink project
