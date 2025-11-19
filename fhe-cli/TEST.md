# FHE CLI Testing Guide

## Quick Test

Run the end-to-end test to verify both encryption and decryption work:

```bash
cd /home/deploy/experimental/zyberlink-demo

# Build release binaries
cargo build -p fhe-cli --release

# Run quick test
./fhe-cli/test_e2e.sh
```

## Manual Testing

### Test Encryption

```bash
# Encrypt value 100
echo "100" | ./target/release/fhe-encrypt
```

**Expected output:**
- Encrypted Data (base64) - very long string (~87K characters)
- Server Key (base64) - very long string
- Client Key (base64) - long string (~31K characters)
- File saved to `~/.zyberlink/client_keys/key_<timestamp>.txt`

### Test Decryption (Piped Input)

```bash
# Get the encrypted data and key from encryption
OUTPUT=$(echo "100" | ./target/release/fhe-encrypt 2>&1)
ENCRYPTED=$(echo "$OUTPUT" | awk '/Encrypted Data/{flag=1;next}/Server Key/{flag=0}flag' | tr -d '\n ')
CLIENT_KEY=$(echo "$OUTPUT" | awk '/Client Key/{flag=1;next}/━/{if(flag) flag=0}flag' | tr -d '\n ')

# Decrypt
printf "%s\n\n%s\n\n" "$ENCRYPTED" "$CLIENT_KEY" | ./target/release/fhe-decrypt
```

**Expected output:**
- Result: 100

### Test Decryption (Interactive)

```bash
./target/release/fhe-decrypt
```

Then paste:
1. Encrypted data (from previous encryption)
2. Press Enter
3. Client key (or path to saved key file)
4. Press Enter

## Test Results

All tests should pass with:
- Encryption completes in ~1-2 seconds
- Decryption completes in ~1-2 seconds
- Decrypted value matches original encrypted value

## Common Issues

### "Client key cannot be empty"
- Make sure you're providing both encrypted data AND client key
- Check that empty lines are properly formatted in input

### Decryption timeout
- FHE operations are slow, wait at least 30 seconds
- Check that you're using the correct client key for the encrypted data

### "Failed to deserialize"
- Make sure the entire base64 string is copied (no truncation)
- Remove any extra whitespace or newlines from the base64 string
