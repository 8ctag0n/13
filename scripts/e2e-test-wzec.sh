#!/bin/bash
# E2E Test for wZEC Payment Integration
# Complete end-to-end test with real TFHE cryptography

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "========================================="
echo "E2E Test: wZEC Payment Integration"
echo "========================================="
echo ""

# Configuration
API_URL="${API_URL:-http://localhost:3001}"
WZEC_MINT="${WZEC_MINT:-7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf}"
TEST_KEYS_DIR="${PROJECT_ROOT}/test-keys"
KEYPAIR_PATH="${KEYPAIR_PATH:-$HOME/.config/solana/id.json}"

echo "Configuration:"
echo "  Project Root: $PROJECT_ROOT"
echo "  API URL: $API_URL"
echo "  wZEC Mint: $WZEC_MINT"
echo "  Test Keys: $TEST_KEYS_DIR"
echo "  Keypair: $KEYPAIR_PATH"
echo ""

# Step 1: Generate TFHE keys if not exist
if [ ! -f "$TEST_KEYS_DIR/server_key.b64" ]; then
    echo "========================================="
    echo "Step 1: Generating TFHE Test Keys"
    echo "========================================="
    echo ""
    echo "⚠️  This will take 3-7 minutes..."
    echo ""

    cd "$PROJECT_ROOT"
    cargo run --release -p test-utils --bin generate-tfhe-keys -- --output-dir "$TEST_KEYS_DIR"

    echo ""
    echo "✅ TFHE keys generated successfully!"
    echo ""
else
    echo "✅ Using existing TFHE keys from: $TEST_KEYS_DIR"
    echo ""
fi

# Step 2: Load TFHE data
echo "========================================="
echo "Step 2: Loading TFHE Test Data"
echo "========================================="
echo ""

SERVER_KEY=$(cat "$TEST_KEYS_DIR/server_key.b64")
ENCRYPTED_DATA=$(cat "$TEST_KEYS_DIR/encrypted_data.b64")

echo "✅ Loaded TFHE data:"
echo "  Server key: ${#SERVER_KEY} chars"
echo "  Encrypted data: ${#ENCRYPTED_DATA} chars"
echo ""

# Step 3: Prepare request data
echo "========================================="
echo "Step 3: Preparing Request"
echo "========================================="
echo ""

CREATOR_PUBKEY=$(solana address -k "$KEYPAIR_PATH")
TIMESTAMP=$(date +%s)
JOB_ID=$((RANDOM % 10000 + 1000))
NONCE="e2e_wzec_${JOB_ID}_${TIMESTAMP}"
MESSAGE="create_job:${JOB_ID}:${TIMESTAMP}:${NONCE}"

echo "  Creator: $CREATOR_PUBKEY"
echo "  Job ID: $JOB_ID"
echo "  Nonce: $NONCE"
echo "  Message: $MESSAGE"
echo ""

# Step 4: Sign message (raw, no prefix)
echo "Signing message..."
SIGNATURE=$(source "$SCRIPT_DIR/.venv/bin/activate" && python3 "$SCRIPT_DIR/sign-message-raw.py" "$KEYPAIR_PATH" "$MESSAGE")
echo "SIGNATURE:"
echo $SIGNATURE

echo "✅ Message signed (raw, base58)"
echo ""

# Step 5: Build JSON request (save to file due to size)
echo "========================================="
echo "Step 4: Building Request Payload"
echo "========================================="
echo ""

REQUEST_FILE=$(mktemp)
cat > "$REQUEST_FILE" <<EOF
{
  "creator_pubkey": "$CREATOR_PUBKEY",
  "encrypted_data": "$ENCRYPTED_DATA",
  "server_key": "$SERVER_KEY",
  "message": "$MESSAGE",
  "signature": "$SIGNATURE",
  "nonce": "$NONCE",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 1000000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "wZEC"
}
EOF

REQUEST_SIZE=$(wc -c < "$REQUEST_FILE")
echo "✅ Request payload saved to temp file"
echo "  Size: $REQUEST_SIZE bytes (~$((REQUEST_SIZE / 1024 / 1024)) MB)"
echo ""

# Step 6: Send request
echo "========================================="
echo "Step 5: Testing wZEC Payment Endpoint"
echo "========================================="
echo ""
echo "Sending request to: $API_URL/api/jobs/validate-and-build"
echo "⚠️  This may take 10-30 seconds due to large payload..."
echo ""

RESPONSE=$(curl -s -w "\n%{http_code}" -X POST \
  "$API_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  --data-binary "@$REQUEST_FILE")

# Cleanup temp file
rm -f "$REQUEST_FILE"

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

echo "HTTP Status: $HTTP_CODE"
echo ""
echo "Response:"
echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
echo ""

# Step 7: Verify result
echo "========================================="
echo "Step 6: Verification"
echo "========================================="
echo ""

if [ "$HTTP_CODE" -eq 200 ]; then
    echo "✅ HTTP 200 OK"

    # Check response has required fields
    JOB_ID_RESPONSE=$(echo "$BODY" | jq -r '.job_id // empty')
    TRANSACTION=$(echo "$BODY" | jq -r '.transaction // empty')

    if [ -n "$JOB_ID_RESPONSE" ]; then
        echo "✅ Job ID returned: $JOB_ID_RESPONSE"
    else
        echo "❌ No job_id in response"
        exit 1
    fi

    if [ -n "$TRANSACTION" ] && [ "$TRANSACTION" != "null" ]; then
        TX_LEN=${#TRANSACTION}
        echo "✅ Transaction built: ${TX_LEN} chars"
    else
        echo "❌ No transaction in response"
        exit 1
    fi

    echo ""
    echo "========================================="
    echo "✅ E2E TEST PASSED!"
    echo "========================================="
    echo ""
    echo "wZEC payment integration verified:"
    echo "  ✅ TFHE ServerKey validated"
    echo "  ✅ Encrypted data processed"
    echo "  ✅ Signature verified"
    echo "  ✅ Payment method 'wZEC' accepted"
    echo "  ✅ Transaction built successfully"
    echo "  ✅ Job ID: $JOB_ID_RESPONSE"
    echo ""
    echo "Backend integration is fully functional! 🎉"
    echo ""

    exit 0
else
    echo "❌ HTTP $HTTP_CODE (expected 200)"
    echo ""

    ERROR=$(echo "$BODY" | jq -r '.error // empty')
    if [ -n "$ERROR" ]; then
        echo "Error message: $ERROR"
    fi

    echo ""
    echo "========================================="
    echo "❌ E2E TEST FAILED"
    echo "========================================="
    echo ""

    exit 1
fi
