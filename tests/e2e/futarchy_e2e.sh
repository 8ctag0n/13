#!/bin/bash
set -e

# Test E2E para Futarchy
# Este script prueba el flujo completo de una apuesta en Futarchy

BLINK_SERVER_CONTAINER="wt-infra_blink_1"
VALIDATOR="http://localhost:8899"
PROGRAM_ID="AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij"

echo "=========================================="
echo "FUTARCHY E2E TEST"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    exit 1
}

info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

# 1. HEALTH CHECKS
echo "1. Health Checks"
echo "----------------"

info "Checking blink-server health..."
HEALTH=$(podman exec ${BLINK_SERVER_CONTAINER} curl -s http://localhost:8080/health 2>&1 || echo "ERROR")
if echo "$HEALTH" | grep -q "ERROR"; then
    fail "blink-server not responding"
fi
pass "blink-server is up"

info "Checking validator..."
VALIDATOR_HEALTH=$(curl -s -X POST ${VALIDATOR} \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok" && echo "OK" || echo "ERROR")
if [ "$VALIDATOR_HEALTH" == "ERROR" ]; then
    fail "validator not responding"
fi
pass "validator is up"

info "Checking program deployed..."
PROGRAM_CHECK=$(solana program show ${PROGRAM_ID} --url ${VALIDATOR} 2>&1 | grep -q "Program Id" && echo "OK" || echo "ERROR")
if [ "$PROGRAM_CHECK" == "ERROR" ]; then
    fail "program not deployed"
fi
pass "program ${PROGRAM_ID} is deployed"

echo ""

# 2. PREPARE BET TEST
echo "2. Prepare Bet Flow"
echo "-------------------"

# Mock keypair for test
TEST_USER_PUBKEY="11111111111111111111111111111111"
TEST_MARKET="FutarchyMarket$(date +%s)"
TEST_AMOUNT=1000000

info "Preparing unsigned transaction..."
PREPARE_PAYLOAD=$(cat <<EOF
{
  "user_pubkey": "${TEST_USER_PUBKEY}",
  "market_id": "${TEST_MARKET}",
  "amount": ${TEST_AMOUNT},
  "side": "yes"
}
EOF
)

PREPARE_RESPONSE=$(podman exec ${BLINK_SERVER_CONTAINER} curl -s -X POST http://localhost:8080/api/futarchy/bet/prepare \
  -H "Content-Type: application/json" \
  -d "${PREPARE_PAYLOAD}")

echo "Prepare response (first 200 chars):"
echo "$PREPARE_RESPONSE" | head -c 200
echo ""

# Check for unsigned_transaction in response
if echo "$PREPARE_RESPONSE" | grep -q "unsigned_transaction"; then
    pass "prepare endpoint returned unsigned_transaction"
else
    fail "prepare endpoint did not return unsigned_transaction"
fi

# Extract job_id for tracking
JOB_ID=$(echo "$PREPARE_RESPONSE" | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)
if [ -z "$JOB_ID" ]; then
    info "No job_id in response (might be ok if FHE disabled)"
else
    pass "job_id created: ${JOB_ID}"
fi

echo ""

# 3. DATABASE STATE CHECK
echo "3. Database State Check"
echo "-----------------------"

info "Checking pending bets table..."
# This assumes we have psql access via podman
BET_COUNT=$(podman exec wt-infra_postgres_1 psql -U wt -d wt -t -c "SELECT COUNT(*) FROM futarchy_pending_bets;" 2>/dev/null | xargs || echo "ERROR")
if [ "$BET_COUNT" == "ERROR" ]; then
    info "Cannot access database (might be permissions)"
else
    info "Pending bets in DB: ${BET_COUNT}"
fi

echo ""

# 4. SDK INTEGRATION TEST
echo "4. SDK Integration Test"
echo "-----------------------"

info "Running SDK unit tests..."
cd /home/deploy/2025q4/wt-infra
TEST_OUTPUT=$(cargo test -p futarchy-sdk --quiet 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    pass "SDK tests passed"
else
    fail "SDK tests failed"
fi

echo ""

# 5. PROVER NODE CHECK (if running)
echo "5. Prover Node Check"
echo "--------------------"

info "Checking prover-node status..."
PROVER_RUNNING=$(pgrep -f prover-node >/dev/null && echo "RUNNING" || echo "NOT_RUNNING")
if [ "$PROVER_RUNNING" == "RUNNING" ]; then
    pass "prover-node is running"

    # Check if it can poll
    info "Prover should be polling blockchain..."
    # This is informational only
else
    info "prover-node not running (start manually if needed)"
fi

echo ""

# SUMMARY
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo ""
echo "Stack Status: UP"
echo "Program ID: ${PROGRAM_ID}"
echo "Tests Passed: ALL BASIC CHECKS"
echo ""
echo "NEXT STEPS:"
echo "1. Start prover-node to test FHE job processing"
echo "2. Complete bet with /bet/submit endpoint"
echo "3. Verify position created on-chain"
echo "4. Check encrypted_amount in Position account"
echo ""
echo "To run full E2E with real wallet:"
echo "  1. Generate test keypair: solana-keygen new -o test-key.json"
echo "  2. Airdrop: solana airdrop 2 -k test-key.json --url ${VALIDATOR}"
echo "  3. Run: cargo run --example prepare_transaction"
echo ""
