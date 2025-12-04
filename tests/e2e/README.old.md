# E2E Tests for ZyberLink

Comprehensive end-to-end test suite for the witness encryption, backend storage, and prover system.

## Test Architecture

### Fast Unit Tests (Always Run)
Located in `tests/witness_flow_unit.rs` - Run in < 0.1 seconds

1. **test_witness_encrypt_decrypt_roundtrip** - Validates complete encryption/decryption cycle
2. **test_decrypt_with_wrong_key_fails** - Security: wrong key detection
3. **test_tampered_ciphertext_fails** - Security: AEAD authentication verification
4. **test_witness_validation_rules** - Business logic: witness validation constraints
5. **test_multiple_encryptions_produce_different_ciphertexts** - Security: randomness verification
6. **test_invalid_witness_rejected** - Edge case: invalid witness handling

### Backend Integration Tests (HTTP)
Located in `tests/backend_integration.rs` - Run in < 0.5 seconds

1. **test_backend_upload_download** - Complete HTTP workflow
2. **test_commitment_calculation** - Blake2b commitment correctness
3. **test_download_nonexistent_witness** - 404 error handling
4. **test_health_endpoint** - Health check endpoint
5. **test_concurrent_uploads_downloads** - Concurrent request handling
6. **test_invalid_commitment_format** - Input validation
7. **test_large_witness_upload** - Large file handling (1MB)

## Running Tests

### All Tests (Recommended)
```bash
cargo test -p e2e-tests
```

### Individual Test Suites
```bash
# Unit tests only
cargo test -p e2e-tests --test witness_flow_unit

# Backend integration tests only
cargo test -p e2e-tests --test backend_integration
```

### Quiet Mode (No Output)
```bash
cargo test -p e2e-tests --quiet
```

## Test Coverage

### Encryption & Witness Flow
- Encrypt/decrypt roundtrip correctness
- Wrong key detection
- Tampering detection (AEAD)
- Non-deterministic encryption (randomness)
- Witness validation rules

### Backend Storage
- Upload/download workflow
- Commitment calculation (Blake2b)
- HTTP error handling (404, 400)
- Concurrent access safety
- Large file support (up to 10MB)
- Health monitoring

### Security Properties Verified
- AEAD authentication (ChaCha20-Poly1305)
- Key exchange security (X25519)
- Commitment integrity (Blake2b)
- Input validation
- Tampering detection

## Performance Characteristics

- **Total execution time**: < 1 second (without compilation)
- **Unit tests**: < 0.01 seconds
- **Integration tests**: < 0.5 seconds
- **No external dependencies**: Tests are self-contained
- **Random port allocation**: No port conflicts

## Test Helpers

### Common Module
- `create_test_witness()` - Valid witness fixture
- `create_invalid_witness()` - Invalid witness for negative tests
- `TestBackend` - HTTP backend server helper (auto-cleanup)

### TestBackend Usage
```rust
use common::TestBackend;

#[tokio::test]
async fn my_test() {
    let backend = TestBackend::start().await.unwrap();
    let url = backend.url(); // http://127.0.0.1:RANDOM_PORT

    // Your test code...

    backend.shutdown().await.unwrap(); // Auto-cleanup
}
```

## Architecture Decisions

1. **Separate Package**: Tests live in `e2e-tests` workspace member
2. **Library Exposure**: Both `prover_node` and `witness_storage` expose library APIs
3. **No Mock Dependencies**: Tests use real implementations
4. **Fast & Deterministic**: All tests run quickly without external services
5. **Self-Contained**: No manual setup required

## Full E2E Integration Test

### Overview

The full integration test (`tests/full_integration_test.rs`) orchestrates **ALL** components for a complete end-to-end validation:

1. **Local Solana Validator** - Fresh test validator
2. **Program Deployment** - Deploys ZyberLink program
3. **Witness Storage Backend** - HTTP backend server
4. **Prover Node** - Prover service with job polling
5. **Client Workflow** - Complete job creation flow
6. **Job Processing** - Prover claims, processes, submits proof
7. **Verification** - Validates entire workflow

### Prerequisites

```bash
# 1. Install Solana CLI tools
solana --version
solana-test-validator --version

# 2. Build Rust components
cargo build --release

# 3. (Optional) Build Solana program
#    If not available, test will run in orchestration-only mode
cargo build-sbf  # Requires solana CLI tools

# 4. Stop any running validators
pkill -f solana-test-validator
```

### Running the Test

The test is marked `#[ignore]` to prevent running in normal test suites (it's heavyweight).

**Run explicitly:**

```bash
cd e2e-tests
cargo test --test full_integration_test -- --ignored --nocapture
```

**Flags:**
- `--test full_integration_test` - Run only this test file
- `--ignored` - Run ignored tests
- `--nocapture` - Show println! output

### Expected Output

```
==================================================
FULL E2E INTEGRATION TEST
==================================================

Starting Solana test validator...
Validator ready after 5 seconds

Deploying program to validator...
Program deployed: CyphLnk...

Starting witness storage backend...
Backend ready after 3 attempts

Starting prover node...
Prover node started

==================================================
Initializing marketplace...
==================================================

Marketplace initialized

==================================================
Running client workflow...
==================================================

Starting client workflow...
  Funding client account...
  Generating witness data...
  Witness generated (1024 bytes)
  Commitment: a3f5b2c1...
  Uploading witness to backend...
  Witness uploaded: abc123...
  Creating job on-chain...
  Client workflow completed!

Waiting for prover to process job...
  Job completed after 12 seconds

All components executed successfully!

Shutting down all components...
All components shut down

==================================================
FULL E2E TEST PASSED
==================================================
```

### Test Duration

**Expected runtime: 30-60 seconds**
- Validator startup: ~5-10s
- Program build/deploy: ~10-20s
- Component startup: ~5s
- Job processing: ~10-30s
- Cleanup: ~2s

### Troubleshooting

**Test hangs or times out:**
```bash
pkill -f solana-test-validator
lsof -i :8899  # Check validator port
lsof -i :3030  # Check backend port
```

**Build errors:**
```bash
cargo clean
cargo build --release
cargo build-sbf
```

### Architecture

```
┌─────────────────────────────────────────┐
│       E2EOrchestrator                    │
├─────────────────────────────────────────┤
│  ┌────────┐  ┌─────────┐  ┌─────────┐  │
│  │Validator│  │Backend  │  │Prover   │  │
│  │(Child)  │  │(Child)  │  │(Child)  │  │
│  └────────┘  └─────────┘  └─────────┘  │
│                                          │
│  Manages: lifecycle, deployment,         │
│           verification                   │
└─────────────────────────────────────────┘
```

## Quick Commands

```bash
# Run only unit tests (fast)
cargo test --lib

# Run backend integration tests
cargo test --test backend_integration

# Run witness flow tests
cargo test --test witness_flow_unit

# Run full E2E (slow, comprehensive)
cargo test --test full_integration_test -- --ignored --nocapture
```

## Coverage Summary

Total: **20 tests** covering:
- 6 unit tests (encryption, validation, security)
- 7 integration tests (HTTP, storage, concurrency)
- 5 orchestrator unit tests (configuration, timeouts, async flow)
- 1 full E2E orchestration test (validator + backend + prover + client - requires Solana tools)
- 1 orchestrator lifecycle test

### Test Environment Requirements

| Test Suite | Requires Solana Tools | Requires Network | Duration |
|------------|----------------------|------------------|----------|
| Unit tests (witness flow) | No | No | < 0.1s |
| Integration tests (backend) | No | No | < 0.5s |
| Orchestrator unit tests | No | No | < 0.1s |
| Full E2E orchestration | Yes | No | 30-60s |

**Quick validation (no Solana tools required):**
```bash
# Run all tests except full E2E
cargo test -p e2e-tests --lib
cargo test -p e2e-tests --test backend_integration
cargo test -p e2e-tests --test witness_flow_unit
cargo test -p e2e-tests --test orchestrator_unit_test
```

**Complete E2E validation (requires Solana tools):**
```bash
# Requires: solana-test-validator, solana CLI
cargo test -p e2e-tests --test full_integration_test -- --ignored --nocapture
```
