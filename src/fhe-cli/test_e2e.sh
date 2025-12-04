#!/bin/bash
#
# FHE CLI End-to-End Test
#
# Tests both encryption and decryption to ensure the tools work correctly.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "  ZyberLink FHE CLI End-to-End Test"
echo "=========================================="
echo ""

# Check binaries exist
if [ ! -f "./target/release/fhe-encrypt" ]; then
    echo -e "${RED}Error: fhe-encrypt binary not found${NC}"
    echo "Run: cargo build -p fhe-cli --release"
    exit 1
fi

if [ ! -f "./target/release/fhe-decrypt" ]; then
    echo -e "${RED}Error: fhe-decrypt binary not found${NC}"
    echo "Run: cargo build -p fhe-cli --release"
    exit 1
fi

# Test values
TEST_VALUES=(42 100 255 0 128)

for VALUE in "${TEST_VALUES[@]}"; do
    echo -e "${YELLOW}Testing value: $VALUE${NC}"

    # Step 1: Encrypt
    echo -n "  [1/2] Encrypting... "
    ENCRYPT_OUTPUT=$(echo "$VALUE" | timeout 30 ./target/release/fhe-encrypt 2>&1)

    # Extract components
    ENCRYPTED_DATA=$(echo "$ENCRYPT_OUTPUT" | awk '/Encrypted Data \(base64\):/{flag=1; next} /Server Key \(base64\):/{flag=0} flag' | tr -d '\n ' | tr -d '\r')
    CLIENT_KEY=$(echo "$ENCRYPT_OUTPUT" | awk '/Client Key \(base64\):/{flag=1; next} /━/{if(flag==1) flag=0} flag' | tr -d '\n ' | tr -d '\r')

    if [ -z "$ENCRYPTED_DATA" ] || [ -z "$CLIENT_KEY" ]; then
        echo -e "${RED}FAILED${NC}"
        echo "    Could not extract encrypted data or client key"
        exit 1
    fi

    echo -e "${GREEN}OK${NC}"
    echo "    Encrypted: ${#ENCRYPTED_DATA} chars"
    echo "    Key: ${#CLIENT_KEY} chars"

    # Step 2: Decrypt
    echo -n "  [2/2] Decrypting... "
    DECRYPT_OUTPUT=$(printf "%s\n\n%s\n\n" "$ENCRYPTED_DATA" "$CLIENT_KEY" | timeout 30 ./target/release/fhe-decrypt 2>&1)

    # Extract result
    RESULT=$(echo "$DECRYPT_OUTPUT" | grep "RESULT:" | awk '{print $NF}' | tr -d '\n')

    if [ "$RESULT" != "$VALUE" ]; then
        echo -e "${RED}FAILED${NC}"
        echo "    Expected: $VALUE"
        echo "    Got: $RESULT"
        echo ""
        echo "Decrypt output:"
        echo "$DECRYPT_OUTPUT"
        exit 1
    fi

    echo -e "${GREEN}OK${NC}"
    echo "    Result: $RESULT"
    echo ""
done

# Test with file-based key
echo -e "${YELLOW}Testing file-based client key${NC}"
echo -n "  [1/3] Encrypting value 77... "

ENCRYPT_OUTPUT=$(echo "77" | timeout 30 ./target/release/fhe-encrypt 2>&1)
ENCRYPTED_DATA=$(echo "$ENCRYPT_OUTPUT" | awk '/Encrypted Data \(base64\):/{flag=1; next} /Server Key \(base64\):/{flag=0} flag' | tr -d '\n ' | tr -d '\r')
KEY_FILE=$(ls -t ~/.zyberlink/client_keys/*.txt | head -1)

echo -e "${GREEN}OK${NC}"
echo "    Key saved to: $KEY_FILE"

echo -n "  [2/3] Decrypting with key file... "
DECRYPT_OUTPUT=$(printf "%s\n\n%s\n\n" "$ENCRYPTED_DATA" "$KEY_FILE" | timeout 30 ./target/release/fhe-decrypt 2>&1)
RESULT=$(echo "$DECRYPT_OUTPUT" | grep "RESULT:" | awk '{print $NF}' | tr -d '\n')

if [ "$RESULT" != "77" ]; then
    echo -e "${RED}FAILED${NC}"
    echo "    Expected: 77, Got: $RESULT"
    exit 1
fi

echo -e "${GREEN}OK${NC}"
echo "    Result: $RESULT"
echo ""

echo "=========================================="
echo -e "  ${GREEN}All tests passed!${NC}"
echo "=========================================="
echo ""
echo "The FHE CLI tools are working correctly."
echo ""
echo "Usage:"
echo "  Encrypt: ./target/release/fhe-encrypt"
echo "  Decrypt: ./target/release/fhe-decrypt"
echo ""
echo "See fhe-cli/README.md for full documentation."
