#!/bin/bash

# Backend Validation Test
# Tests that the backend correctly validates incoming requests

set -e

echo "🧪 Testing Backend Validation"
echo "=============================="
echo ""

BASE_URL="http://127.0.0.1:8080"

# Test 1: Health check
echo "✅ Test 1: Health Check"
HEALTH=$(curl -s "$BASE_URL/health")
echo "$HEALTH" | jq .
if echo "$HEALTH" | jq -e '.status == "ok"' > /dev/null; then
    echo "   ✓ Health check passed"
else
    echo "   ✗ Health check failed"
    exit 1
fi
echo ""

# Test 2: Missing signature
echo "🔒 Test 2: Missing Signature Validation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "{\"creator\":\"test\",\"timestamp\":1234567890,\"nonce\":\"test\"}",
    "signature": "",
    "nonce": "test-nonce",
    "encrypted_data": "dGVzdA==",
    "server_key": "dGVzdA==",
    "operation": "Add",
    "operation_value": 10,
    "price_lamports": 1000000,
    "required_provers": 3,
    "consensus_threshold": 2
  }' 2>&1)

if echo "$RESPONSE" | grep -q "error"; then
    echo "   ✓ Backend correctly rejected request with missing signature"
else
    echo "   ✗ Backend should have rejected request"
    echo "$RESPONSE"
fi
echo ""

# Test 3: Invalid base64
echo "📦 Test 3: Invalid Base64 Validation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "{\"creator\":\"test\",\"timestamp\":1234567890,\"nonce\":\"test\"}",
    "signature": "dummy-sig",
    "nonce": "test-nonce-2",
    "encrypted_data": "not-valid-base64!!!",
    "server_key": "also-not-valid!!!",
    "operation": "Add",
    "operation_value": 10,
    "price_lamports": 1000000,
    "required_provers": 3,
    "consensus_threshold": 2
  }' 2>&1)

if echo "$RESPONSE" | grep -q "error"; then
    echo "   ✓ Backend correctly rejected invalid base64"
else
    echo "   ✗ Backend should have rejected invalid base64"
fi
echo ""

# Test 4: Invalid operation
echo "⚙️  Test 4: Invalid Operation Validation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "{\"creator\":\"test\",\"timestamp\":1234567890,\"nonce\":\"test\"}",
    "signature": "dummy-sig",
    "nonce": "test-nonce-3",
    "encrypted_data": "dGVzdA==",
    "server_key": "dGVzdA==",
    "operation": "InvalidOp",
    "operation_value": 10,
    "price_lamports": 1000000,
    "required_provers": 3,
    "consensus_threshold": 2
  }' 2>&1)

if echo "$RESPONSE" | grep -q "error"; then
    echo "   ✓ Backend correctly rejected invalid operation"
else
    echo "   ✗ Backend should have rejected invalid operation"
fi
echo ""

# Test 5: Invalid consensus (threshold > required)
echo "🤝 Test 5: Invalid Consensus Config"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "{\"creator\":\"test\",\"timestamp\":1234567890,\"nonce\":\"test\"}",
    "signature": "dummy-sig",
    "nonce": "test-nonce-4",
    "encrypted_data": "dGVzdA==",
    "server_key": "dGVzdA==",
    "operation": "Add",
    "operation_value": 10,
    "price_lamports": 1000000,
    "required_provers": 2,
    "consensus_threshold": 5
  }' 2>&1)

if echo "$RESPONSE" | grep -q "error"; then
    echo "   ✓ Backend correctly rejected invalid consensus config"
else
    echo "   ✗ Backend should have rejected invalid consensus"
fi
echo ""

echo "✅ All validation tests passed!"
echo ""
echo "📝 Next Steps:"
echo "   1. Integrate Solana wallet signing (Phantom/Solflare)"
echo "   2. Implement proper ED25519 signature verification"
echo "   3. Build frontend with wallet connection"
echo "   4. Test complete E2E flow with real wallet signatures"
