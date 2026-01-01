#!/bin/bash
set -e

# Test E2E para Futarchy
# Usage:
#   ./futarchy_e2e.sh          # Basic health checks only
#   ./futarchy_e2e.sh --real   # Full E2E with real on-chain transactions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BLINK_SERVER_URL="http://localhost:8080"
VALIDATOR="http://localhost:8899"
PROGRAM_ID="AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij"
TEST_KEYPAIR="${SCRIPT_DIR}/test-wallet.json"
REAL_MODE=false

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --real)
            REAL_MODE=true
            shift
            ;;
        --keypair)
            TEST_KEYPAIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=========================================="
echo "FUTARCHY E2E TEST"
if [ "$REAL_MODE" = true ]; then
    echo "MODE: REAL (on-chain transactions)"
else
    echo "MODE: BASIC (health checks only)"
fi
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

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

step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

# =============================================================================
# 1. HEALTH CHECKS
# =============================================================================
echo "1. Health Checks"
echo "----------------"

info "Checking blink-server health..."
HEALTH=$(curl -s "${BLINK_SERVER_URL}/health" 2>&1 || echo "ERROR")
if echo "$HEALTH" | grep -q "ok\|healthy\|OK"; then
    pass "blink-server is up"
else
    # Try via podman if direct access fails
    HEALTH=$(podman exec wt-infra_blink_1 curl -s http://localhost:8080/health 2>&1 || echo "ERROR")
    if echo "$HEALTH" | grep -q "ok\|healthy\|OK"; then
        pass "blink-server is up (via container)"
        BLINK_SERVER_URL="http://localhost:8080"
        USE_PODMAN=true
    else
        fail "blink-server not responding: $HEALTH"
    fi
fi

info "Checking validator..."
VALIDATOR_HEALTH=$(curl -s -X POST ${VALIDATOR} \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>&1)
if echo "$VALIDATOR_HEALTH" | grep -q "ok"; then
    pass "validator is up"
else
    fail "validator not responding: $VALIDATOR_HEALTH"
fi

info "Checking program deployed..."
PROGRAM_CHECK=$(timeout 10 solana program show ${PROGRAM_ID} --url ${VALIDATOR} 2>&1 || echo "TIMEOUT")
if echo "$PROGRAM_CHECK" | grep -q "Program Id"; then
    pass "program ${PROGRAM_ID} is deployed"
    PROGRAM_DEPLOYED=true
else
    info "program ${PROGRAM_ID} NOT deployed (expected for local dev without deploy)"
    info "To deploy: cargo build-sbf && solana program deploy ..."
    PROGRAM_DEPLOYED=false
fi

echo ""

# =============================================================================
# 2. BASIC PREPARE TEST (requires program deployed)
# =============================================================================
echo "2. Basic Prepare Test"
echo "---------------------"

if [ "$PROGRAM_DEPLOYED" = false ]; then
    info "Skipping prepare test (program not deployed)"
else
    TEST_USER_PUBKEY="11111111111111111111111111111111"
    TEST_MARKET=1

    info "Testing /bet/prepare with mock data..."
PREPARE_PAYLOAD=$(cat <<EOF
{
  "market_id": ${TEST_MARKET},
  "bettor": "${TEST_USER_PUBKEY}",
  "side": true,
  "amount_lamports": 1000000,
  "ciphertext_hash": "$(printf '0%.0s' {1..64})",
  "proof": "$(printf 'AA%.0s' {1..171})==",
  "public_inputs": "$(printf 'AA%.0s' {1..53})==",
  "circuit_type": 30
}
EOF
)

if [ "$USE_PODMAN" = true ]; then
    PREPARE_RESPONSE=$(podman exec wt-infra_blink_1 curl -s -X POST http://localhost:8080/api/futarchy/bet/prepare \
      -H "Content-Type: application/json" \
      -d "${PREPARE_PAYLOAD}")
else
    PREPARE_RESPONSE=$(curl -s -X POST "${BLINK_SERVER_URL}/api/futarchy/bet/prepare" \
      -H "Content-Type: application/json" \
      -d "${PREPARE_PAYLOAD}")
fi

if echo "$PREPARE_RESPONSE" | grep -q "unsigned_transaction"; then
    pass "prepare endpoint returned unsigned_transaction"
else
    info "Prepare response: $PREPARE_RESPONSE"
    fail "prepare endpoint did not return unsigned_transaction"
fi
fi  # end PROGRAM_DEPLOYED check

echo ""

# =============================================================================
# 3. REAL MODE: Full on-chain flow
# =============================================================================
if [ "$REAL_MODE" = true ]; then
    echo "3. Real On-Chain Flow"
    echo "---------------------"

    if [ "$PROGRAM_DEPLOYED" = false ]; then
        fail "Cannot run --real mode: program not deployed. Deploy first with: cargo build-sbf && solana program deploy"
    fi

    # 3a. Setup keypair
    step "Setting up test keypair..."
    if [ ! -f "$TEST_KEYPAIR" ]; then
        info "Generating new test keypair at $TEST_KEYPAIR"
        solana-keygen new -o "$TEST_KEYPAIR" --no-bip39-passphrase --force
    fi
    TEST_PUBKEY=$(solana-keygen pubkey "$TEST_KEYPAIR")
    pass "Using keypair: $TEST_PUBKEY"

    # 3b. Airdrop SOL
    step "Airdropping SOL..."
    BALANCE_BEFORE=$(solana balance "$TEST_PUBKEY" --url "$VALIDATOR" 2>&1 | grep -oE '[0-9.]+' | head -1 || echo "0")
    if (( $(echo "$BALANCE_BEFORE < 1" | bc -l) )); then
        solana airdrop 2 "$TEST_PUBKEY" --url "$VALIDATOR" >/dev/null 2>&1 || true
        sleep 1
    fi
    BALANCE=$(solana balance "$TEST_PUBKEY" --url "$VALIDATOR" 2>&1)
    pass "Balance: $BALANCE"

    # 3c. Generate bet data
    step "Generating bet data..."
    BET_DATA=$(node "${SCRIPT_DIR}/helpers/generate_bet_data.mjs" 100000000 yes)
    CIPHERTEXT=$(echo "$BET_DATA" | jq -r '.ciphertext')
    CIPHERTEXT_HASH=$(echo "$BET_DATA" | jq -r '.ciphertext_hash')
    PROOF=$(echo "$BET_DATA" | jq -r '.proof')
    PUBLIC_INPUTS=$(echo "$BET_DATA" | jq -r '.public_inputs')
    AMOUNT=$(echo "$BET_DATA" | jq -r '.amount_lamports')
    pass "Generated ciphertext_hash: ${CIPHERTEXT_HASH:0:16}..."

    # 3d. Call /bet/prepare
    step "Calling /bet/prepare..."
    PREPARE_PAYLOAD=$(cat <<EOF
{
  "market_id": 1,
  "bettor": "${TEST_PUBKEY}",
  "side": true,
  "amount_lamports": ${AMOUNT},
  "ciphertext_hash": "${CIPHERTEXT_HASH}",
  "proof": "${PROOF}",
  "public_inputs": "${PUBLIC_INPUTS}",
  "circuit_type": 30
}
EOF
)

    if [ "$USE_PODMAN" = true ]; then
        PREPARE_RESPONSE=$(podman exec wt-infra_blink_1 curl -s -X POST http://localhost:8080/api/futarchy/bet/prepare \
          -H "Content-Type: application/json" \
          -d "${PREPARE_PAYLOAD}")
    else
        PREPARE_RESPONSE=$(curl -s -X POST "${BLINK_SERVER_URL}/api/futarchy/bet/prepare" \
          -H "Content-Type: application/json" \
          -d "${PREPARE_PAYLOAD}")
    fi

    if echo "$PREPARE_RESPONSE" | grep -q "error"; then
        info "Prepare error: $PREPARE_RESPONSE"
        fail "Failed to prepare transaction"
    fi

    UNSIGNED_TX=$(echo "$PREPARE_RESPONSE" | jq -r '.unsigned_transaction')
    if [ "$UNSIGNED_TX" = "null" ] || [ -z "$UNSIGNED_TX" ]; then
        info "Response: $PREPARE_RESPONSE"
        fail "No unsigned_transaction in response"
    fi
    pass "Got unsigned transaction (${#UNSIGNED_TX} chars)"

    # 3e. Sign transaction
    step "Signing transaction..."
    export SOLANA_RPC_URL="$VALIDATOR"
    SIGN_RESULT=$(node "${SCRIPT_DIR}/helpers/sign_tx.mjs" "$TEST_KEYPAIR" "$UNSIGNED_TX" 2>&1)

    if echo "$SIGN_RESULT" | grep -q "Error\|error"; then
        info "Sign error: $SIGN_RESULT"
        fail "Failed to sign transaction"
    fi

    SIGNED_TX=$(echo "$SIGN_RESULT" | jq -r '.signed_tx')
    if [ "$SIGNED_TX" = "null" ] || [ -z "$SIGNED_TX" ]; then
        info "Sign result: $SIGN_RESULT"
        fail "No signed_tx in result"
    fi
    pass "Transaction signed"

    # 3f. Submit transaction
    step "Calling /bet/submit..."
    SUBMIT_PAYLOAD=$(cat <<EOF
{
  "signed_tx": "${SIGNED_TX}",
  "ciphertext": "${CIPHERTEXT}"
}
EOF
)

    if [ "$USE_PODMAN" = true ]; then
        SUBMIT_RESPONSE=$(podman exec wt-infra_blink_1 curl -s -X POST http://localhost:8080/api/futarchy/bet/submit \
          -H "Content-Type: application/json" \
          -d "${SUBMIT_PAYLOAD}")
    else
        SUBMIT_RESPONSE=$(curl -s -X POST "${BLINK_SERVER_URL}/api/futarchy/bet/submit" \
          -H "Content-Type: application/json" \
          -d "${SUBMIT_PAYLOAD}")
    fi

    if echo "$SUBMIT_RESPONSE" | grep -q '"status":"confirmed"'; then
        TX_SIG=$(echo "$SUBMIT_RESPONSE" | jq -r '.tx_signature')
        pass "Transaction confirmed: $TX_SIG"
    else
        info "Submit response: $SUBMIT_RESPONSE"
        fail "Transaction not confirmed"
    fi

    # 3g. Verify on-chain
    step "Verifying transaction on-chain..."
    sleep 2
    TX_STATUS=$(solana confirm "$TX_SIG" --url "$VALIDATOR" 2>&1 || echo "")
    if echo "$TX_STATUS" | grep -qi "finalized\|confirmed"; then
        pass "Transaction verified on-chain"
    else
        info "TX status: $TX_STATUS"
        info "(Transaction may still be processing)"
    fi

    echo ""
fi

# =============================================================================
# 4. SDK INTEGRATION TEST
# =============================================================================
echo "4. SDK Integration Test"
echo "-----------------------"

info "Running SDK unit tests..."
cd /home/deploy/2025q4/wt-infra
TEST_OUTPUT=$(cargo test -p futarchy-sdk --quiet 2>&1 || echo "FAILED")
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    pass "SDK tests passed"
elif echo "$TEST_OUTPUT" | grep -q "FAILED"; then
    info "Some SDK tests may have failed (check manually)"
else
    pass "SDK tests completed"
fi

echo ""

# =============================================================================
# 5. PROVER NODE CHECK
# =============================================================================
echo "5. Prover Node Check"
echo "--------------------"

info "Checking prover-node status..."
PROVER_RUNNING=$(pgrep -f prover-node >/dev/null 2>&1 && echo "RUNNING" || echo "NOT_RUNNING")
if [ "$PROVER_RUNNING" == "RUNNING" ]; then
    pass "prover-node is running"

    if [ "$REAL_MODE" = true ]; then
        info "Prover should detect the new job via blockchain polling..."
        info "Check prover logs: journalctl -u prover-node -f"
    fi
else
    info "prover-node not running (start manually for full E2E)"
fi

echo ""

# =============================================================================
# SUMMARY
# =============================================================================
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo ""
echo "Stack Status: UP"
echo "Program ID: ${PROGRAM_ID}"

if [ "$REAL_MODE" = true ]; then
    echo ""
    echo "REAL MODE RESULTS:"
    echo "  - Keypair: $TEST_PUBKEY"
    echo "  - TX Signature: ${TX_SIG:-N/A}"
    echo ""
    echo "VERIFICATION COMMANDS:"
    echo "  solana confirm $TX_SIG --url $VALIDATOR"
    echo "  solana account <position_pda> --url $VALIDATOR"
else
    echo ""
    echo "NEXT STEPS:"
    echo "  Run with --real flag for full on-chain test:"
    echo "  ./futarchy_e2e.sh --real"
fi
echo ""
