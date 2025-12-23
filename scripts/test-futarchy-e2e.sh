#!/bin/bash
# Futarchy E2E Test Script
# Tests the complete flow: market creation -> bet -> FHE job detection

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

API_URL="${API_URL:-http://localhost:9000}"
BLINK_URL="${BLINK_URL:-http://localhost:8080}"

echo -e "${BLUE}=== Futarchy E2E Test ===${NC}"
echo ""

# Step 1: Health checks
echo -e "${BLUE}Step 1: Health checks${NC}"

echo -n "  public-api (9000): "
if curl -sf "$API_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  Run: make start-public-api"
    exit 1
fi

echo -n "  blink (8080): "
if curl -sf "$BLINK_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  Run: make c1"
    exit 1
fi

echo -n "  validator: "
if solana cluster-version > /dev/null 2>&1; then
    echo -e "${GREEN}OK ($(solana cluster-version))${NC}"
else
    echo -e "${RED}FAILED${NC}"
    exit 1
fi

echo ""

# Step 2: Create market
echo -e "${BLUE}Step 2: Creating futarchy market${NC}"

MARKET_PAYLOAD=$(cat <<EOF
{
    "question": "E2E Test Market $(date +%s)",
    "creator": "FdMVQxVLxGhYd8hyBE5hKoCVzYAuXioTeMV2u1VP1Wcj",
    "oracle": "FdMVQxVLxGhYd8hyBE5hKoCVzYAuXioTeMV2u1VP1Wcj",
    "resolution_window_secs": 3600,
    "max_bet_lamports": 1000000000
}
EOF
)

echo "  Payload: $MARKET_PAYLOAD"
echo ""

MARKET_RESP=$(curl -sf -X POST "$API_URL/api/futarchy/markets" \
    -H "Content-Type: application/json" \
    -d "$MARKET_PAYLOAD" 2>&1) || {
    echo -e "${RED}Failed to create market${NC}"
    echo "Response: $MARKET_RESP"
    exit 1
}

echo "  Response: $MARKET_RESP"
MARKET_ID=$(echo "$MARKET_RESP" | jq -r '.market.id // .id // .market_id // empty' 2>/dev/null || echo "")

if [ -z "$MARKET_ID" ]; then
    echo -e "${YELLOW}Warning: Could not extract market ID${NC}"
    MARKET_ID="1"
fi

echo -e "${GREEN}  Market created: $MARKET_ID${NC}"
echo ""

# Step 3: List markets
echo -e "${BLUE}Step 3: Listing markets${NC}"
MARKETS=$(curl -sf "$API_URL/api/futarchy/markets" 2>&1) || echo "[]"
echo "  Markets: $MARKETS" | head -c 500
echo ""
echo ""

# Step 4: Place bet (DB-only endpoint, NOT on-chain)
echo -e "${BLUE}Step 4: Placing bet on market $MARKET_ID${NC}"
echo -e "${YELLOW}  NOTE: This uses /markets/{id}/bet which stores in DB only${NC}"
echo -e "${YELLOW}  For on-chain: use /bet/prepare + sign + /bet/submit${NC}"

BET_PAYLOAD=$(cat <<EOF
{
    "bettor": "FdMVQxVLxGhYd8hyBE5hKoCVzYAuXioTeMV2u1VP1Wcj",
    "side": "yes",
    "amount_lamports": 100000000
}
EOF
)

echo "  Payload: $BET_PAYLOAD"
echo ""

BET_RESP=$(curl -sf -X POST "$API_URL/api/futarchy/markets/$MARKET_ID/bet" \
    -H "Content-Type: application/json" \
    -d "$BET_PAYLOAD" 2>&1) || {
    echo -e "${YELLOW}Bet request returned error (may be expected)${NC}"
    BET_RESP="error"
}

echo "  Response: $BET_RESP"
echo ""

# Step 4b: Show how to use on-chain flow
echo -e "${BLUE}Step 4b: On-chain bet flow (requires Rust client)${NC}"
echo "  1. POST /api/futarchy/bet/prepare"
echo "     Required: bettor, market_id, side, amount_lamports,"
echo "               ciphertext_hash (64 hex), proof (base64), public_inputs (base64)"
echo "  2. Client signs returned unsigned_transaction"
echo "  3. POST /api/futarchy/bet/submit"
echo "     Required: signed_tx (base64), ciphertext (base64), server_key (base64)"
echo "  4. TX confirms -> FHE job created on-chain -> Prover detects it"
echo ""

# Step 5: Check for FHE jobs
echo -e "${BLUE}Step 5: Checking pending FHE jobs${NC}"
FHE_JOBS=$(curl -sf "$API_URL/api/futarchy/fhe-jobs/pending" 2>&1) || echo "[]"
echo "  Pending jobs: $FHE_JOBS"
echo ""

# Step 6: Check prover logs
echo -e "${BLUE}Step 6: Checking prover logs${NC}"

PROVER_LOGS=$(ls /tmp/prover-*.log 2>/dev/null | head -1)
if [ -n "$PROVER_LOGS" ]; then
    echo "  Last 10 lines from $PROVER_LOGS:"
    tail -10 "$PROVER_LOGS" 2>/dev/null | sed 's/^/    /'

    echo ""
    echo "  Searching for FHE/futarchy mentions:"
    grep -i "fhe\|futarchy\|pool\|job" "$PROVER_LOGS" 2>/dev/null | tail -5 | sed 's/^/    /' || echo "    (none found)"
else
    echo -e "${YELLOW}  No prover logs found at /tmp/prover-*.log${NC}"
    echo "  Run: make c3"
fi

echo ""
echo -e "${BLUE}=== Test Complete ===${NC}"
echo ""
echo "Summary:"
echo "  - API: $API_URL"
echo "  - Market ID: $MARKET_ID"
echo "  - Bet placed: $([ "$BET_RESP" != "error" ] && echo "yes" || echo "no/error")"
echo ""
echo "Next steps:"
echo "  1. If provers not running: make c3"
echo "  2. Monitor prover logs: tail -f /tmp/prover-*.log"
echo "  3. Check blockchain for FHE jobs"
