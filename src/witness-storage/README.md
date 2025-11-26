# ZyberLink Witness Storage

Simple HTTP backend for storing encrypted witness data.

## Overview

This is a lightweight HTTP server that provides temporary storage for encrypted witness data. Clients encrypt their witness using the prover's X25519 public key, upload it here, and receive a commitment (Blake2b hash) to include in their on-chain job.

## API Endpoints

### POST /witness
Upload encrypted witness data.

**Request:**
- Body: Binary data (encrypted witness)
- Content-Type: application/octet-stream
- Max size: 10MB

**Response:**
```json
{
  "commitment": "abc123...",
  "size": 1234,
  "status": "stored"
}
```

### GET /witness/:commitment
Download encrypted witness by commitment.

**Response:**
- Body: Binary data (encrypted witness)
- Status: 200 OK or 404 Not Found

### GET /health
Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "witnesses_stored": 42,
  "total_bytes": 123456
}
```

## Running the Server

```bash
cd witness-storage
RUST_LOG=info cargo run
```

Server will listen on `http://0.0.0.0:3030`

## Testing

```bash
cargo test
```

## Storage Backend

Currently uses in-memory HashMap (data lost on restart). For production, replace with:
- Redis (for ephemeral storage)
- PostgreSQL (for persistent storage)
- S3/IPFS (for decentralized storage)

## Security Notes

- No authentication (public upload/download)
- No rate limiting (add in production)
- No encryption at rest (data is already encrypted client-side)
- No CORS restrictions (permissive for development)
