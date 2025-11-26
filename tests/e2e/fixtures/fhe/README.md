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
