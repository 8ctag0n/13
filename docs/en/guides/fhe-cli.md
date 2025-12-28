# FHE CLI Guide

Command-line tools for encrypting data locally and decrypting computation results using Fully Homomorphic Encryption (FHE).

## Overview

The ZyberLink FHE CLI provides two essential tools for privacy-preserving computations:

1. **zyb fhe encrypt** - Encrypt your data locally before sending to ZyberLink
2. **zyb fhe decrypt** - Decrypt computation results after provers process your job

**Privacy Guarantee:** Your plaintext data never leaves your machine. Only encrypted ciphertext is uploaded to the platform.

### Why Use the CLI?

**Security Benefits:**
- Encryption happens locally (you control the keys)
- No trusted third party sees your data
- Client key never transmitted over network
- Full cryptographic control

**Flexibility:**
- Encrypt any values 0-255 (FheUint8)
- Supports batch encryption (multiple values)
- Compatible with all ZyberLink operations
- Works offline (no network required for encryption)

## Installation

### Prerequisites

- Rust 1.75 or higher
- Cargo package manager
- ~50 MB disk space (for compiled binary)

### Build from Source

```bash
# Clone ZyberLink repository
git clone https://github.com/8ctag0n/13.git zyberlink
cd zyberlink

# Navigate to FHE CLI directory
cd src/zyb-cli

# Build release version
cargo build --release

# Binaries located at:
# - target/release/zyb
```

**Verify installation:**
```bash
./target/release/zyb --version
# Expected: zyb 0.1.0
```

### Add to PATH (Optional)

```bash
# Copy binary to system path
sudo cp target/release/zyb /usr/local/bin/

# Now use from anywhere
zyb fhe --help
```

## Quick Start

### 1. Encrypt Data

```bash
cd src/zyb-cli

# Encrypt a single value
cargo run --release --bin zyb fhe encrypt --values 42

# Encrypt multiple values (comma-separated)
cargo run --release --bin zyb fhe encrypt --values 10,20,30,40,50
```

**Output:**
```
🔐 ZyberLink FHE Encryption Tool
================================

⏳ Generating FHE keypair...
✅ Keypair generated (took 1.2s)

🔒 Encrypting 5 values...
✅ Values encrypted successfully

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 FILES GENERATED:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📁 Output directory: ./fhe-output/

Files created:
  ✅ witness.bin (52.4 MB) - Upload to ZyberLink
  ✅ client_key.bin (0.5 MB) - KEEP SECRET!

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

⚠️  IMPORTANT: Keep client_key.bin safe!
   You need it to decrypt results.

📤 Next steps:
   1. Upload witness.bin to ZyberLink platform
   2. Submit computation job
   3. Download encrypted result
   4. Decrypt using client_key.bin
```

### 2. Upload to Platform

1. Navigate to https://demo.zyberlink.fun
2. Choose your operation (Analytics or Proof of Innocence)
3. Upload `witness.bin`
4. Submit job and wait for completion

### 3. Decrypt Result

After your job completes:

```bash
# Download encrypted result from platform (e.g., result.bin)

# Decrypt the result
cargo run --release --bin zyb fhe decrypt \
  --result-path ~/Downloads/result.bin \
  --client-key-path ./fhe-output/client_key.bin
```

**Output:**
```
🔓 ZyberLink FHE Decryption Tool
=================================

⏳ Loading client key...
✅ Client key loaded

⏳ Loading encrypted result...
✅ Encrypted result loaded (256 bytes)

🔓 Decrypting result...
✅ Result decrypted successfully

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 RESULT: 150
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Operation: Sum
Input values: 5 (encrypted)
Computation: 10 + 20 + 30 + 40 + 50 = 150

✅ Your data remained encrypted throughout!
```

## Command Reference

### encrypt - Encrypt Data

**Syntax:**
```bash
zyb fhe encrypt --values <VALUES> [--output <PATH>]
```

**Arguments:**

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--values` | String | Yes | Comma-separated values to encrypt (0-255) |
| `--output` | Path | No | Output directory (default: ./fhe-output) |

**Examples:**

```bash
# Single value
zyb fhe encrypt --values 42

# Multiple values
zyb fhe encrypt --values 10,20,30,40,50

# Custom output directory
zyb fhe encrypt --values 100,200 --output ~/encrypted-data/

# Large dataset
zyb fhe encrypt --values 5,15,25,35,45,55,65,75,85,95
```

**Output Files:**

1. **witness.bin** (~52 MB)
   - Contains: Encrypted data + Server key
   - Purpose: Upload to ZyberLink for computation
   - Safe to share: No plaintext information

2. **client_key.bin** (~0.5 MB)
   - Contains: Client decryption key
   - Purpose: Decrypt computation results
   - **KEEP SECRET:** Anyone with this can decrypt your results

**Performance:**

| Dataset Size | Key Generation | Encryption Time | Total Time |
|--------------|----------------|-----------------|------------|
| 1 value | 1-2 seconds | <0.1 seconds | ~2 seconds |
| 10 values | 1-2 seconds | 0.2 seconds | ~2 seconds |
| 100 values | 1-2 seconds | 1-2 seconds | ~4 seconds |

**Note:** Key generation dominates total time. Encrypting more values is nearly free once keys are generated.

### decrypt - Decrypt Result

**Syntax:**
```bash
zyb fhe decrypt --result-path <PATH> --client-key-path <PATH>
```

**Arguments:**

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--result-path` | Path | Yes | Path to encrypted result file |
| `--client-key-path` | Path | Yes | Path to client_key.bin |

**Examples:**

```bash
# Basic decryption
zyb fhe decrypt \
  --result-path result.bin \
  --client-key-path ./fhe-output/client_key.bin

# With absolute paths
zyb fhe decrypt \
  --result-path ~/Downloads/result_abc123.bin \
  --client-key-path ~/.zyberlink/keys/client_key.bin

# Decrypt result from specific job
zyb fhe decrypt \
  --result-path ./results/job_42_result.bin \
  --client-key-path ./fhe-output/client_key.bin
```

**Result Types:**

**Sum/Average (single value):**
```
🎯 RESULT: 150
```

**CountIf (count):**
```
🎯 RESULT: 3
(3 values matched the predicate)
```

**Histogram (multiple bins):**
```
🎯 RESULTS:
  Bin 0 (0-18): 5
  Bin 1 (19-35): 12
  Bin 2 (36-65): 8
  Bin 3 (66+): 2
```

## Complete Workflows

### Workflow 1: Private Analytics (Sum)

**Scenario:** Calculate total donations without revealing individual amounts

#### Step 1: Encrypt Donation Values

```bash
cd src/zyb-cli

# Encrypt donation amounts (in dollars)
cargo run --release --bin zyb fhe encrypt \
  --values 100,250,75,500,125

# Output:
# ✅ witness.bin created (52.4 MB)
# ✅ client_key.bin created (0.5 MB)
```

#### Step 2: Submit to ZyberLink

```
1. Open https://demo.zyberlink.fun/analytics
2. Connect Solana wallet
3. Upload witness.bin
4. Select operation: SUM
5. Set price: 0.0054 SOL (recommended)
6. Click "Submit Analytics Job"
7. Wait for completion (~60-120 seconds)
```

#### Step 3: Download and Decrypt Result

```bash
# Download result.bin from platform

# Decrypt
cargo run --release --bin zyb fhe decrypt \
  --result-path ~/Downloads/result.bin \
  --client-key-path ./fhe-output/client_key.bin

# Output:
# 🎯 RESULT: 1050
# (Total donations: $1,050)
```

**Privacy Preserved:**
- Individual amounts never exposed
- Only encrypted sum revealed to you
- Provers never saw plaintext values

### Workflow 2: Proof of Innocence (CountIf)

**Scenario:** Prove you haven't interacted with sanctioned addresses

#### Step 1: Map Transaction History to Indices

```python
# Example: Convert addresses to indices
transactions = [
    "0xABC...123",  # → Index 10
    "0xDEF...456",  # → Index 25
    "0xGHI...789",  # → Index 50
    "0xJKL...012",  # → Index 75
]

indices = [10, 25, 50, 75]

# Sanctioned list: [66, 77, 88, 99]
# Expected result: 0 matches (innocent)
```

#### Step 2: Encrypt Transaction Indices

```bash
cd src/zyb-cli

# Encrypt your transaction indices
cargo run --release --bin zyb fhe encrypt \
  --values 10,25,50,75

# Output:
# ✅ witness.bin created
# ✅ client_key.bin created
```

#### Step 3: Submit Proof of Innocence Job

```
1. Open https://demo.zyberlink.fun/proof-of-innocence
2. Connect wallet
3. Upload witness.bin
4. Select sanctioned index: 66
5. Set price: 0.0054 SOL
6. Click "Verify Innocence"
7. Wait for completion
```

#### Step 4: Decrypt Verification Result

```bash
# Download result from platform

cargo run --release --bin zyb fhe decrypt \
  --result-path result.bin \
  --client-key-path ./fhe-output/client_key.bin

# Output:
# 🎯 RESULT: 0
# (No sanctioned interactions detected)
# ✅ Proof of Innocence verified!
```

### Workflow 3: Batch Processing Multiple Jobs

**Scenario:** Analyze different metrics on the same dataset

#### Step 1: Encrypt Dataset Once

```bash
# Encrypt your dataset
cargo run --release --bin zyb fhe encrypt \
  --values 15,22,28,35,42,48,55,62,68,75

# Save client key location
CLIENT_KEY=./fhe-output/client_key.bin
```

#### Step 2: Submit Multiple Jobs

```
Using same witness.bin:

Job 1: SUM - Total of all values
Job 2: AVERAGE - Mean value
Job 3: COUNTIF (>= 50) - Values above threshold
Job 4: HISTOGRAM - Distribution analysis
```

#### Step 3: Decrypt All Results

```bash
# Decrypt job 1 result (Sum)
zyb fhe decrypt --result-path result_job1.bin --client-key-path $CLIENT_KEY
# Result: 450

# Decrypt job 2 result (Average)
zyb fhe decrypt --result-path result_job2.bin --client-key-path $CLIENT_KEY
# Result: 45 (average)

# Decrypt job 3 result (CountIf)
zyb fhe decrypt --result-path result_job3.bin --client-key-path $CLIENT_KEY
# Result: 5 (values >= 50)

# Decrypt job 4 result (Histogram)
zyb fhe decrypt --result-path result_job4.bin --client-key-path $CLIENT_KEY
# Results: [3, 4, 3] (bins)
```

## Advanced Usage

### Custom Output Directories

Organize encrypted data by project:

```bash
# Create project directory
mkdir -p ~/projects/analytics-demo/encrypted

# Encrypt with custom output
zyb fhe encrypt \
  --values 10,20,30 \
  --output ~/projects/analytics-demo/encrypted

# Files created:
# ~/projects/analytics-demo/encrypted/witness.bin
# ~/projects/analytics-demo/encrypted/client_key.bin
```

### Encrypting Maximum Values

FHE supports values 0-255 (8-bit unsigned integers):

```bash
# Minimum value
zyb fhe encrypt --values 0

# Maximum value
zyb fhe encrypt --values 255

# Mix of values
zyb fhe encrypt --values 0,50,100,150,200,255

# Invalid (will error)
zyb fhe encrypt --values 256  # ❌ Out of range
zyb fhe encrypt --values -1   # ❌ Negative not supported
```

**Workaround for larger values:**
Use scaling:

```python
# Original values
values = [1000, 2000, 3000]

# Scale down to 0-255
scale_factor = 10
scaled = [v // scale_factor for v in values]
# scaled = [100, 200, 255] (capped at 255)

# Encrypt scaled values
# After computation, scale result back up
```

### Batch Encryption Script

Automate encryption of multiple datasets:

```bash
#!/bin/bash
# encrypt-batch.sh

datasets=(
  "10,20,30,40,50"
  "15,25,35,45,55"
  "20,30,40,50,60"
)

for i in "${!datasets[@]}"; do
  echo "Encrypting dataset $((i+1))..."

  zyb fhe encrypt \
    --values "${datasets[$i]}" \
    --output "./encrypted/dataset_$i"

  echo "✅ Dataset $((i+1)) encrypted"
done

echo "✅ All datasets encrypted"
ls -lh ./encrypted/
```

### Key Management Best Practices

```bash
# Create secure key storage directory
mkdir -p ~/.zyberlink/keys
chmod 700 ~/.zyberlink/keys

# Move client keys to secure location
mv fhe-output/client_key.bin ~/.zyberlink/keys/project1_$(date +%Y%m%d).bin
chmod 400 ~/.zyberlink/keys/project1_*.bin

# Backup keys encrypted
gpg --symmetric --cipher-algo AES256 \
  ~/.zyberlink/keys/project1_20251204.bin

# Store .gpg backup on external drive
cp ~/.zyberlink/keys/project1_20251204.bin.gpg /media/usb/backups/

# Never commit keys to git
echo "client_key.bin" >> .gitignore
echo "*.bin" >> .gitignore
```

## Troubleshooting

### Encryption Fails

**Error:** "Failed to generate keypair"

**Cause:** Insufficient memory or interrupted process

**Solution:**
```bash
# Check available memory
free -h
# Need at least 2GB free

# Close other applications
# Retry encryption

# If persists, reboot system
sudo reboot
```

### Values Out of Range

**Error:** "Value X out of range (must be 0-255)"

**Cause:** Input values too large for FheUint8

**Solution:**
```bash
# Use scaling
# Original: 1000 → Scaled: 100 (divide by 10)
# After computation, multiply result by 10

# Or use multiple encrypted values
# 1000 = encrypt(255) + encrypt(255) + encrypt(255) + encrypt(235)
```

### Decryption Fails

**Error:** "Failed to deserialize client key"

**Cause:** Corrupted key file or wrong file format

**Solution:**
```bash
# Verify file exists and is readable
ls -lh client_key.bin

# Check file size (should be ~0.5 MB)
du -h client_key.bin

# Try with different client key
# (if you encrypted multiple times)

# Re-encrypt if necessary
zyb fhe encrypt --values 10,20,30
```

### Wrong Decryption Result

**Error:** Result doesn't match expected value

**Cause:** Using wrong client key (from different encryption)

**Solution:**
```bash
# Client keys are tied to specific witness files
# Ensure you're using the matching client_key.bin

# Check timestamps
ls -lt fhe-output/
# witness.bin and client_key.bin should have same timestamp

# If unsure, re-encrypt and resubmit job
```

### "Witness.bin Not Found"

**Error:** Platform can't find uploaded witness

**Cause:** Upload failed or incomplete

**Solution:**
```bash
# Verify file size before upload
ls -lh witness.bin
# Should be ~52 MB

# Check internet connection
# Retry upload

# Re-encrypt if file corrupted
zyb fhe encrypt --values <ORIGINAL_VALUES>
```

## Technical Details

### FHE Parameters

**Library:** TFHE-rs (Zama's Concrete) version 0.10

**Configuration:**
- **Data type:** FheUint8 (8-bit encrypted unsigned integers)
- **Security:** 128-bit security level
- **Noise budget:** Sufficient for ~10 FHE operations
- **Supported operations:** Addition, multiplication, comparison

**Key Sizes:**
- **Client key:** ~500 KB
- **Server key:** ~50 MB (included in witness.bin)
- **Encrypted value:** ~256 bytes per value

### Witness File Format

```
[Witness.bin Structure]
  ├─ Length prefix (4 bytes, little-endian u32)
  ├─ Encrypted data (variable size)
  │   ├─ Bincode-serialized Vec<Vec<u8>>
  │   └─ Each element: Serialized FheUint8
  └─ Server key (~50 MB)
      └─ Serialized TFHE ServerKey
```

**Example:**
```
Total size: 52,428,800 bytes (~52 MB)
- Length prefix: 4 bytes
- Encrypted data: 1,256 bytes (5 values × ~256 bytes each)
- Server key: 52,427,540 bytes
```

### Security Considerations

**What's secure:**
- ✅ Client key never transmitted (stays local)
- ✅ Plaintext never exposed (encrypted locally)
- ✅ Server key can be public (no secret info)
- ✅ Encrypted values indistinguishable from random

**What to protect:**
- 🔒 **client_key.bin** - Anyone with this can decrypt your results
- 🔒 Original plaintext values (don't tell anyone)

**What's safe to share:**
- ✅ witness.bin (encrypted data + server key)
- ✅ Server key (enables computation, no decryption)
- ✅ Encrypted results (until you decrypt)

## Performance Optimization

### Minimize Key Generation Overhead

```bash
# Key generation is slow (1-2 seconds)
# Encrypt multiple values at once to amortize cost

# Inefficient (generates keys 3 times)
zyb fhe encrypt --values 10
zyb fhe encrypt --values 20
zyb fhe encrypt --values 30

# Efficient (generates keys once)
zyb fhe encrypt --values 10,20,30
```

### Parallel Encryption (Future)

Currently, encryption is sequential. Future versions may support:

```bash
# Hypothetical parallel encryption
zyb fhe encrypt --values 1,2,3,4,5,6,7,8,9,10 --parallel

# Would encrypt values in parallel using multiple CPU cores
```

## Integration with Applications

### Programmatic Usage (Rust)

```rust
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, generate_keys, FheUint8};

fn encrypt_values(values: Vec<u8>) -> Result<(Vec<Vec<u8>>, Vec<u8>), Box<dyn std::error::Error>> {
    // Generate keys
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    // Encrypt values
    let encrypted: Vec<Vec<u8>> = values
        .iter()
        .map(|&v| {
            let encrypted = FheUint8::encrypt(v, &client_key);
            bincode::serialize(&encrypted).unwrap()
        })
        .collect();

    // Serialize server key
    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted, server_key_bytes))
}
```

### API Integration (JavaScript/TypeScript)

```javascript
import { exec } from 'child_process';
import { promisify } from 'util';

const execPromise = promisify(exec);

async function encryptData(values) {
  const valuesStr = values.join(',');

  const { stdout } = await execPromise(
    `zyb fhe encrypt --values ${valuesStr} --output ./encrypted`
  );

  return {
    witness: './encrypted/witness.bin',
    clientKey: './encrypted/client_key.bin'
  };
}

async function decryptResult(resultPath, clientKeyPath) {
  const { stdout } = await execPromise(
    `zyb fhe decrypt --result-path ${resultPath} --client-key-path ${clientKeyPath}`
  );

  // Parse result from stdout
  const match = stdout.match(/RESULT: (\d+)/);
  return match ? parseInt(match[1]) : null;
}
```

## Next Steps

Now that you understand the FHE CLI:

- **[Analytics Guide](analytics-guide.md)** - Use encrypted data for statistics
- **[Proof of Innocence Guide](proof-of-innocence-guide.md)** - Verify compliance privately
- **[Prover Setup](prover-setup.md)** - Run your own computation node
- **[SDK Integration](sdk-integration.md)** - Build FHE into your app

## Support

For FHE CLI questions:

- **Documentation:** https://docs.zyberlink.fun
- **GitHub Issues:** https://github.com/8ctag0n/13/issues
- **Discord:** [Join community]
- **Email:** support@zyberlink.fun

## Related Documentation

- [Zyb CLI Source](/src/zyb-cli/) - Implementation details
- [TFHE-rs Documentation](https://docs.zama.ai/tfhe-rs) - FHE library
- [FHE Operations Guide](/docs/book/en/concepts/fhe-operations.md) - Supported operations

---

**FHE CLI:** Encrypt locally. Compute on ciphertext. Decrypt privately. Your data never exposed.
