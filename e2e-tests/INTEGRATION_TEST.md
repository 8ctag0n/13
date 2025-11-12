# Complete Prover Integration Test

This document explains how to run the complete end-to-end integration test that validates the entire CypherLink marketplace workflow with a real prover node.

## Overview

The integration test (`test_prover_integration.rs`) orchestrates all components:

1. **Witness Storage Backend** - HTTP server for encrypted witness data
2. **Prover Node** - Autonomous daemon that polls, claims, and processes jobs
3. **Marketplace Program** - On-chain Solana program
4. **Client** - Creates jobs and uploads witness data

## Prerequisites

### 1. Solana Test Validator

The test requires a running Solana test validator:

```bash
# Start validator in a separate terminal
solana-test-validator

# Verify it's running
solana cluster-version
```

### 2. Deploy CypherLink Program

Deploy the marketplace program to the test validator:

```bash
# Build the program
cd programs
cargo build-sbf

# Deploy (returns program ID)
solana program deploy target/deploy/cypherlink.so

# Note the program ID for the test
```

### 3. Initialize Marketplace

Initialize the marketplace with an authority:

```bash
# Create a test authority keypair (if not exists)
solana-keygen new -o ~/.config/solana/test-authority.json --no-bip39-passphrase

# Fund the authority
solana airdrop 10 ~/.config/solana/test-authority.json

# Initialize marketplace (requires SDK client or custom script)
# See initialize test for reference
```

## Running the Test

The integration test is marked with `#[ignore]` because it requires external services. Run it explicitly:

```bash
# From the e2e-tests directory
cargo test --test test_prover_integration -- --ignored --nocapture

# Or from the root
cargo test -p e2e-tests --test test_prover_integration -- --ignored --nocapture
```

### Test Flow

The test will:

1. **Setup (5-10 seconds)**
   - Start witness storage backend on port 3031
   - Register a new prover with 5 SOL stake
   - Start prover node daemon

2. **Job Creation (2 seconds)**
   - Client creates a job on-chain (2 SOL price)
   - Client uploads encrypted witness to backend

3. **Prover Processing (10-30 seconds)**
   - Prover node detects pending job
   - Prover claims the job
   - Prover downloads encrypted witness
   - Prover generates real Halo2 proof (slow!)
   - Prover submits proof on-chain

4. **Verification (1 second)**
   - Verify escrow account closed
   - Verify prover received payment
   - Verify job marked as completed

5. **Cleanup**
   - Stop background services
   - Remove temporary files

### Expected Output

```
================================================================================
INTEGRATION TEST: Complete Prover Node Workflow
================================================================================

Setup: Starting Background Services
--------------------------------------------------------------------------------
  Starting witness storage backend...
  ✓ Witness backend started on http://localhost:3031
  Starting prover node...
  ✓ Prover node started

Setup: Register Prover
--------------------------------------------------------------------------------
  Prover: <pubkey>
  ✓ Prover registered with 5 SOL stake

STEP 1: Client Creates Job
--------------------------------------------------------------------------------
  Client: <pubkey>
  Client balance: 10 SOL
  ✓ Job created on-chain
    Job ID: 0
    Job PDA: <pubkey>
    Price: 2 SOL
    Signature: <sig>

STEP 2: Upload Encrypted Witness
--------------------------------------------------------------------------------
  ✓ Witness uploaded to backend
    Commitment: <hex>

STEP 3: Waiting for Prover Node to Process Job
--------------------------------------------------------------------------------
  Prover node will:
    1. Detect the pending job
    2. Claim the job
    3. Download encrypted witness
    4. Generate Halo2 proof (this takes ~10-30 seconds)
    5. Submit proof on-chain

  Waiting... (max 60 seconds)
    Still processing... (10 seconds elapsed)
    Still processing... (20 seconds elapsed)
  ✓ Job completed! (after 24 seconds)

Verification
--------------------------------------------------------------------------------
  ✓ Escrow account closed (funds distributed)
  ✓ Prover balance after: 6.99 SOL

Cleanup
--------------------------------------------------------------------------------
  ✓ Background services stopped

================================================================================
✅ COMPLETE PROVER INTEGRATION TEST PASSED!
================================================================================

Summary:
  - Job created by client
  - Prover detected and claimed job
  - Prover generated real Halo2 proof
  - Proof submitted on-chain
  - Payment distributed automatically
```

## Troubleshooting

### Test Timeout

If the test times out (job not completed within 60 seconds):

1. **Check validator logs**: Look for program errors
2. **Check prover node**: May have crashed or failed to initialize
3. **Check witness backend**: Verify it's responding
4. **Increase timeout**: Edit `test_prover_integration.rs` and increase polling duration

### Port Conflicts

If you see "Address already in use" errors:

```bash
# Kill any existing processes on port 3031
pkill -f witness-storage

# Or use a different port in the test
```

### Program Not Deployed

If you see "Invalid program ID" errors:

1. Deploy the program: `solana program deploy target/deploy/cypherlink.so`
2. Update the program ID in the test

### Halo2 Proof Generation Fails

If proof generation fails:

1. Check available memory (Halo2 is memory-intensive)
2. Check CPU (proof generation is CPU-intensive)
3. Reduce circuit complexity in test if needed

## Performance Notes

- **Witness Backend**: Starts in ~3 seconds
- **Prover Node**: Initializes in ~5 seconds (Halo2 setup)
- **Job Creation**: ~2 seconds (on-chain transaction)
- **Proof Generation**: ~10-30 seconds (depends on circuit complexity and CPU)
- **Proof Submission**: ~2 seconds (on-chain transaction)

**Total test time**: ~30-60 seconds

## Manual Testing

You can also test the components manually:

### 1. Start Services

```bash
# Terminal 1: Start validator
solana-test-validator

# Terminal 2: Start witness backend
cargo run --release --bin witness-storage -- --port 3031

# Terminal 3: Start prover node
cargo run --release --bin cypherlink-prover -- \
  --rpc-url http://localhost:8899 \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  --witness-backend-url http://localhost:3031
```

### 2. Register Prover

```bash
cargo run --release --bin cypherlink-prover -- \
  --rpc-url http://localhost:8899 \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  register --stake-amount 5000000000
```

### 3. Create Job

Use the SDK or run individual tests:

```bash
cargo test --test test_create_job -- --nocapture
```

### 4. Watch Prover Logs

The prover node will output logs as it processes jobs:

```
[Job 0] Starting processing
[Job 0] Claiming job...
[Job 0] Claimed successfully
[Job 0] Downloading encrypted witness...
[Job 0] Generating proof...
[Job 0] Proof submitted successfully
[Job 0] Completed! 🎉
```

## Next Steps

After successful integration testing:

1. **Deploy to devnet**: Test on Solana devnet
2. **Stress test**: Multiple concurrent jobs
3. **Network conditions**: Test with network delays
4. **Edge cases**: Timeouts, failures, slashing
5. **Production deployment**: Deploy to mainnet-beta

## Related Documentation

- [E2E Tests README](README.md) - Individual instruction tests
- [Prover Node Documentation](../prover-node/README.md) - Prover configuration
- [Witness Storage Documentation](../witness-storage/README.md) - Backend API
- [SDK Documentation](../sdk/README.md) - Client SDK usage
