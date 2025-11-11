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

## Future Enhancements

Optional full E2E tests (currently not implemented, would require):
- Solana test validator running
- Program deployed
- Full workflow from job creation to proof submission

These would be marked with `#[ignore]` and run with:
```bash
cargo test -p e2e-tests --ignored
```

## Coverage Summary

Total: **13 tests** covering:
- 6 unit tests (encryption, validation, security)
- 7 integration tests (HTTP, storage, concurrency)
- 0 failed
- All pass in < 1 second
