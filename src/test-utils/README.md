# Test Utilities for ZyberLink

Utilities for testing, CI/CD, and local development.

## TFHE Key Generator

Generate TFHE test keys for E2E testing without manual key generation.

### Usage

#### Basic usage (generates to `./test-keys/`):
```bash
cargo run --release -p test-utils --bin generate-tfhe-keys
```

#### Custom output directory:
```bash
cargo run --release -p test-utils --bin generate-tfhe-keys -- --output-dir my-keys
```

#### Compact mode (faster for CI):
```bash
cargo run --release -p test-utils --bin generate-tfhe-keys -- --compact
```

#### Full options:
```bash
cargo run --release -p test-utils --bin generate-tfhe-keys --help
```

### Output Files

The generator creates:
- `server_key.b64` - Base64 encoded ServerKey (for API requests)
- `server_key.bin` - Binary ServerKey (for faster loading)
- `encrypted_data.b64` - Base64 encrypted test value
- `encrypted_data.bin` - Binary encrypted test value
- `README.md` - Metadata and usage instructions

### E2E Testing

Use generated keys in E2E tests:

```bash
# Generate keys once
cargo run --release -p test-utils --bin generate-tfhe-keys

# Load in E2E test script
SERVER_KEY=$(cat test-keys/server_key.b64)
ENCRYPTED_DATA=$(cat test-keys/encrypted_data.b64)

# Use in API request
curl -X POST http://localhost:3001/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d "{
    \"server_key\": \"$SERVER_KEY\",
    \"encrypted_data\": \"$ENCRYPTED_DATA\",
    ...
  }"
```

### CI/CD Integration

#### GitHub Actions Example:
```yaml
- name: Generate TFHE test keys
  run: cargo run --release -p test-utils --bin generate-tfhe-keys

- name: Run E2E tests
  run: ./scripts/e2e-test.sh
  env:
    SERVER_KEY_PATH: test-keys/server_key.b64
```

#### Pre-commit keys (faster CI):
```bash
# Generate once, commit to repo
cargo run --release -p test-utils --bin generate-tfhe-keys --output-dir .github/test-keys

# Add to .gitignore exceptions
echo "!.github/test-keys/" >> .gitignore

# Commit
git add .github/test-keys/
git commit -m "chore: add pre-generated TFHE test keys for CI"
```

### Performance

- **Generation time:** 3-7 minutes (CPU dependent)
- **Server key size:** ~40-120 MB
- **Recommended:** Pre-generate for CI/CD to save build time

## Development

### Adding new test utilities

1. Create new binary in `src/bin/`
2. Add dependencies to `Cargo.toml`
3. Document in this README
4. Test with `cargo run -p test-utils --bin <name>`

### Dependencies

- `tfhe` - TFHE library (same version as blink-server)
- `bincode` - Serialization
- `base64` - Encoding
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `chrono` - Timestamps

## License

MIT OR Apache-2.0
