#!/bin/bash
#
# Automated API tests for dynamic pricing
#

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

BACKEND_URL="http://127.0.0.1:8080"
PASSED=0
FAILED=0

log_test() {
    echo -e "\n${YELLOW}[TEST]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}  ✓ PASS${NC} $1"
    PASSED=$((PASSED + 1))
}

log_fail() {
    echo -e "${RED}  ✗ FAIL${NC} $1"
    FAILED=$((FAILED + 1))
}

# Check backend is running
if ! curl -s $BACKEND_URL/health >/dev/null 2>&1; then
    echo "ERROR: Backend not running at $BACKEND_URL"
    echo "Start it with: cd blink-server && ./target/release/blink-server"
    exit 1
fi

echo "🧪 ZyberLink Dynamic Pricing API Tests"
echo "========================================"

# ============================================================================
# Test 1: Tier 1 (Add) Cost Estimation
# ============================================================================
log_test "Test 1: Tier 1 (Add) - 3 provers"

RESPONSE=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{
        "operation": "add",
        "operation_value": 5,
        "required_provers": 3
    }')

TIER=$(echo $RESPONSE | jq -r '.complexity_tier')
MIN_PAYMENT=$(echo $RESPONSE | jq -r '.min_payment_lamports')
TOTAL=$(echo $RESPONSE | jq -r '.total_min_payment_lamports')
TIMEOUT=$(echo $RESPONSE | jq -r '.timeout_seconds')

if [ "$TIER" == "1" ] && [ "$MIN_PAYMENT" == "1000000" ] && [ "$TOTAL" == "3000000" ] && [ "$TIMEOUT" == "60" ]; then
    log_pass "Tier 1 pricing correct (0.001 SOL/prover × 3 = 0.003 SOL, 60s timeout)"
else
    log_fail "Expected tier:1, min:1M, total:3M, timeout:60, got tier:$TIER, min:$MIN_PAYMENT, total:$TOTAL, timeout:$TIMEOUT"
fi

# ============================================================================
# Test 2: Tier 3 (Threshold) Cost Estimation
# ============================================================================
log_test "Test 2: Tier 3 (Threshold) - 3 provers"

RESPONSE=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{
        "operation": "threshold",
        "operation_value": 18,
        "required_provers": 3
    }')

TIER=$(echo $RESPONSE | jq -r '.complexity_tier')
MIN_PAYMENT=$(echo $RESPONSE | jq -r '.min_payment_lamports')
TOTAL=$(echo $RESPONSE | jq -r '.total_min_payment_lamports')
TIMEOUT=$(echo $RESPONSE | jq -r '.timeout_seconds')

if [ "$TIER" == "3" ] && [ "$MIN_PAYMENT" == "5000000" ] && [ "$TOTAL" == "15000000" ] && [ "$TIMEOUT" == "300" ]; then
    log_pass "Tier 3 pricing correct (0.005 SOL/prover × 3 = 0.015 SOL, 300s timeout)"
else
    log_fail "Expected tier:3, min:5M, total:15M, timeout:300, got tier:$TIER, min:$MIN_PAYMENT, total:$TOTAL, timeout:$TIMEOUT"
fi

# ============================================================================
# Test 3: Tier 5 (Histogram) Cost Estimation
# ============================================================================
log_test "Test 3: Tier 5 (Histogram 10 bins) - 3 provers"

RESPONSE=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{
        "operation": "histogram",
        "bins": 10,
        "required_provers": 3
    }')

TIER=$(echo $RESPONSE | jq -r '.complexity_tier')
TOTAL=$(echo $RESPONSE | jq -r '.total_min_payment_lamports')
TIMEOUT=$(echo $RESPONSE | jq -r '.timeout_seconds')

# Histogram with 10 bins should be tier 5, total > 2 SOL, timeout 1600s
if [ "$TIER" == "5" ] && [ "$TOTAL" -gt "2000000000" ] && [ "$TIMEOUT" == "1600" ]; then
    log_pass "Tier 5 pricing correct (>2 SOL total, 1600s timeout)"
else
    log_fail "Expected tier:5, total>2B, timeout:1600, got tier:$TIER, total:$TOTAL, timeout:$TIMEOUT"
fi

# ============================================================================
# Test 4: Pricing Scales with Prover Count
# ============================================================================
log_test "Test 4: Pricing scales with prover count (Add)"

# 3 provers
RESPONSE_3=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{
        "operation": "add",
        "operation_value": 5,
        "required_provers": 3
    }')
TOTAL_3=$(echo $RESPONSE_3 | jq -r '.total_min_payment_lamports')

# 5 provers
RESPONSE_5=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{
        "operation": "add",
        "operation_value": 5,
        "required_provers": 5
    }')
TOTAL_5=$(echo $RESPONSE_5 | jq -r '.total_min_payment_lamports')

if [ "$TOTAL_3" == "3000000" ] && [ "$TOTAL_5" == "5000000" ]; then
    log_pass "Pricing scales correctly (3 provers = 3M, 5 provers = 5M)"
else
    log_fail "Expected 3M and 5M, got $TOTAL_3 and $TOTAL_5"
fi

# ============================================================================
# Test 5: All Operation Types
# ============================================================================
log_test "Test 5: All operation types return valid responses"

OPERATIONS=("add" "multiply" "threshold" "sum" "average" "histogram")
ALL_VALID=true

for OP in "${OPERATIONS[@]}"; do
    if [ "$OP" == "histogram" ]; then
        PAYLOAD='{"operation":"histogram","bins":5,"required_provers":3}'
    elif [ "$OP" == "sum" ] || [ "$OP" == "average" ]; then
        PAYLOAD='{"operation":"'$OP'","expected_count":100,"required_provers":3}'
    else
        PAYLOAD='{"operation":"'$OP'","operation_value":10,"required_provers":3}'
    fi

    RESPONSE=$(curl -s -X POST $BACKEND_URL/api/estimate-cost \
        -H "Content-Type: application/json" \
        -d "$PAYLOAD")

    TIER=$(echo $RESPONSE | jq -r '.complexity_tier')

    if [ "$TIER" == "null" ] || [ -z "$TIER" ]; then
        echo "    ✗ $OP failed"
        ALL_VALID=false
    else
        echo "    ✓ $OP → Tier $TIER"
    fi
done

if [ "$ALL_VALID" == "true" ]; then
    log_pass "All operation types work"
else
    log_fail "Some operations failed"
fi

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "========================================"
echo "📊 Test Summary"
echo "========================================"
echo -e "${GREEN}✓ Passed: $PASSED${NC}"

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}✗ Failed: $FAILED${NC}"
    echo ""
    echo "⚠️  Some tests failed. Check backend logs:"
    echo "    tail -f /tmp/blink-server.log"
    exit 1
else
    echo -e "${GREEN}✗ Failed: 0${NC}"
    echo ""
    echo "🎉 All API tests passed!"
fi
