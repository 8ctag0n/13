# ZyberLink FHE CLI Tools

Command-line tools for encrypting and decrypting data using Fully Homomorphic Encryption (FHE) with ZyberLink.

## Overview

These tools allow users to:
1. Encrypt data locally with FHE before sending to ZyberLink
2. Decrypt computation results after provers have processed encrypted data

**Privacy Guarantee:** Your data never leaves your machine unencrypted. The webapp and provers only see encrypted values.

## Installation

From the project root:

```bash
cd fhe-cli
cargo build --release
```

Binaries will be in `target/release/`:
- `fhe-encrypt` - Encrypt data and generate keys
- `fhe-decrypt` - Decrypt computation results

## Quick Start

### 1. Encrypt Your Data

```bash
cargo run --bin fhe-encrypt
```

**Example session:**
```
🔐 ZyberLink FHE Encryption Tool
================================

Enter value to encrypt (0-255): 42

⏳ Generating FHE keypair...
   (This may take 1-2 seconds)
✅ Keypair generated

🔒 Encrypting value: 42
✅ Value encrypted successfully

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 COPY THESE TO THE WEBAPP:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Encrypted Data (base64):
AQIDBAUGBwgJ... (shortened for example)

Server Key (base64):
SGVsbG8gd29y... (shortened for example)

⚠️  KEEP THIS SECRET - Client Key (base64):
cHJpdmF0ZSBr... (shortened for example)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💾 Client key saved to: /home/user/.zyberlink/client_keys/key_1234567890.txt

💡 Keep this file safe! You'll need it to decrypt the result.
```

### 2. Submit to Webapp

Copy the **Encrypted Data** and **Server Key** to the ZyberLink webapp.

**Important:** Keep the **Client Key** secret! Store it safely.

### 3. Decrypt Result

After provers compute your result, download the encrypted result and decrypt it:

```bash
cargo run --bin fhe-decrypt
```

**Example session:**
```
🔓 ZyberLink FHE Decryption Tool
=================================

Enter encrypted result (base64):
(Paste the encrypted result from the webapp)
> ZGVjcnlwdGVk...

Enter client key (base64) or path to key file:
(You can paste the key or provide a file path)
> ~/.zyberlink/client_keys/key_1234567890.txt

⏳ Loading client key from file...
✅ Client key loaded from file

🔓 Decrypting result...
✅ Result decrypted successfully

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 RESULT: 52
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Usage Details

### fhe-encrypt

Generates a fresh FHE keypair and encrypts a value.

**Input:**
- A number between 0 and 255

**Output:**
- **Encrypted Data:** Send to webapp
- **Server Key:** Send to webapp (provers use this to compute)
- **Client Key:** Keep secret! You need this to decrypt results

**Client Key Storage:**
Client keys are automatically saved to `~/.zyberlink/client_keys/` with timestamped filenames.

### fhe-decrypt

Decrypts an encrypted computation result.

**Input:**
- **Encrypted Result:** Base64 string from webapp
- **Client Key:** Either:
  - File path (e.g., `~/.zyberlink/client_keys/key_1234567890.txt`)
  - Base64 string (paste directly)

**Output:**
- Decrypted plaintext result (0-255)

## Technical Details

### FHE Parameters

- **Library:** TFHE-rs (Zama's Concrete)
- **Data Type:** FheUint8 (encrypted unsigned 8-bit integers)
- **Operations:** Addition, multiplication, subtraction (on encrypted data)

### Key Generation

Key generation takes 1-2 seconds. This is normal for FHE.

**Performance Note:** The initial encryption is slow because we generate a fresh keypair. In production, users might want to reuse keys across multiple jobs.

### Server Key Size

Server keys are large (~50-100 MB serialized). This is expected for FHE.

When copy-pasting, the base64 representation will be long. This is normal.

## Security

### What's Safe to Share

- Encrypted Data
- Server Key

### What to Keep Secret

- **Client Key** - Anyone with this can decrypt your results
- **Original plaintext value** - Don't tell anyone what you encrypted

### Where Keys are Stored

Client keys are saved to: `~/.zyberlink/client_keys/`

**File format:** Plain text base64 (for easy copy-paste)

**Permissions:** Ensure this directory has appropriate permissions (e.g., `chmod 700 ~/.zyberlink/client_keys`)

## Example Workflow

```bash
# 1. Encrypt a value
cargo run --bin fhe-encrypt
# Enter: 42
# Copy the encrypted data and server key

# 2. Go to webapp, paste the values, select operation (e.g., +10)

# 3. Wait for provers to compute (they work on encrypted data)

# 4. Download encrypted result from webapp

# 5. Decrypt the result
cargo run --bin fhe-decrypt
# Paste encrypted result
# Enter client key path: ~/.zyberlink/client_keys/key_1234567890.txt
# Result: 52 (because 42 + 10 = 52)
```

## Troubleshooting

### "Invalid base64 encoding"

- Make sure you copied the entire base64 string
- Remove any extra spaces or newlines
- Don't modify the base64 string

### "Failed to deserialize"

- The encrypted data might be corrupted
- Make sure you're using the correct client key for this data
- Try re-encrypting from scratch

### "Key generation is slow"

- This is normal! FHE key generation takes 1-2 seconds
- Be patient, the progress will complete

### "Client key not found"

- Check that the file path is correct
- Use `ls ~/.zyberlink/client_keys/` to see available keys
- You can also paste the base64 key directly instead

## Development

Run tests:

```bash
cargo test
```

Build release binaries:

```bash
cargo build --release
```

The binaries will be in `target/release/fhe-encrypt` and `target/release/fhe-decrypt`.

## License

MIT OR Apache-2.0
