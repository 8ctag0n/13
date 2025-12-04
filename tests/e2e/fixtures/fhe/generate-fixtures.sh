#!/bin/bash
#
# Generate FHE test fixtures for E2E tests
#
# This script creates deterministic encrypted data files for testing
# the CreateJob wizard without depending on real FHE encryption.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR"

echo "Generating FHE test fixtures..."
echo "Output directory: $OUTPUT_DIR"

# Generate encrypted_data.json (small, deterministic)
cat > "$OUTPUT_DIR/encrypted_data.json" <<'EOF'
{
  "version": "1.0",
  "algorithm": "TFHE",
  "data": {
    "ciphertext": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyAhIiMkJSYnKCkqKywtLi8wMTIzNDU2Nzg5Ojs8PT4/QEFCQ0RFRkdISUpLTE1OT1BRUlNUVVZXWFlaW1xdXl9gYWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXp7fH1+fw==",
    "size_bytes": 128,
    "encrypted_values": 5,
    "padding": "PKCS7"
  },
  "metadata": {
    "created_at": "2025-01-24T00:00:00Z",
    "purpose": "e2e_testing",
    "original_size": 40
  }
}
EOF

# Generate server_key.bin (minimal valid structure, ~500KB for performance)
# In production this would be 2-3MB, but for tests we keep it smaller
echo "Creating server_key.bin (500KB)..."
dd if=/dev/urandom of="$OUTPUT_DIR/server_key.bin" bs=1024 count=500 2>/dev/null

# Generate client_key.bin (smaller, ~100KB)
echo "Creating client_key.bin (100KB)..."
dd if=/dev/urandom of="$OUTPUT_DIR/client_key.bin" bs=1024 count=100 2>/dev/null

# Generate invalid files for error testing
echo "Creating invalid fixtures..."

# Invalid JSON (syntax error)
cat > "$OUTPUT_DIR/invalid_data.json" <<'EOF'
{
  "version": "1.0",
  "data": {
    "ciphertext": "invalid_base64_!@#$%"
    // missing comma - syntax error
    "size_bytes": 0
  }
EOF

# Empty file
touch "$OUTPUT_DIR/empty_file.bin"

# File too large (simulate >10MB file)
cat > "$OUTPUT_DIR/README.md" <<'EOF'
# FHE Test Fixtures

## Files

### Valid Fixtures
- `encrypted_data.json` (5KB) - Encrypted data with mock ciphertext
- `server_key.bin` (500KB) - TFHE server key (randomized but deterministic size)
- `client_key.bin` (100KB) - TFHE client key

### Invalid Fixtures (for error testing)
- `invalid_data.json` - JSON with syntax errors
- `empty_file.bin` - Empty file (0 bytes)

## Usage

```typescript
import { test } from '@playwright/test';

test('upload FHE files', async ({ page }) => {
  await page.setInputFiles('input[type="file"]', [
    'e2e-tests/fixtures/fhe/encrypted_data.json',
    'e2e-tests/fixtures/fhe/server_key.bin'
  ]);
});
```

## Regenerate

```bash
cd e2e-tests/fixtures/fhe
./generate-fixtures.sh
```

## Git LFS

Large binary files (*.bin) should be tracked with Git LFS:

```bash
git lfs track "*.bin"
```
EOF

# Create checksums for validation
echo "Generating checksums..."
sha256sum encrypted_data.json server_key.bin client_key.bin > checksums.txt

echo ""
echo "✓ Fixtures generated successfully!"
echo ""
echo "Files created:"
ls -lh "$OUTPUT_DIR"/*.{json,bin,md,txt} 2>/dev/null || true
echo ""
echo "Total size:"
du -sh "$OUTPUT_DIR"
