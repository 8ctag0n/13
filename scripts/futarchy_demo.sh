#!/bin/bash
# Futarchy MVP Demo Script
# Tests the basic futarchy endpoints

set -e

BASE_URL="${FUTARCHY_URL:-http://localhost:8080}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "======================================"
echo "  Futarchy MVP Demo"
echo "  Base URL: $BASE_URL"
echo "======================================"
echo ""

# Check if server is running
echo -e "${YELLOW}[0/6] Checking server health...${NC}"
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Server not responding at $BASE_URL${NC}"
    echo "Make sure blink-server is running"
    exit 1
fi
echo -e "${GREEN}Server is healthy${NC}"
echo ""

# Step 1: Create a market
echo -e "${YELLOW}[1/6] Creating prediction market...${NC}"
MARKET_RESPONSE=$(curl -s -X POST "$BASE_URL/api/futarchy/markets" \
    -H "Content-Type: application/json" \
    -d '{
        "question": "Will BTC reach $100k by end of 2025?",
        "creator": "DemoCreator11111111111111111111111111111111",
        "oracle": "DemoOracle111111111111111111111111111111111",
        "resolution_window_secs": 86400,
        "max_bet_lamports": 1000000000
    }')

if echo "$MARKET_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    echo -e "${RED}ERROR creating market:${NC}"
    echo "$MARKET_RESPONSE" | jq .
    exit 1
fi

MARKET_ID=$(echo "$MARKET_RESPONSE" | jq -r '.market.id')
echo -e "${GREEN}Created market: $MARKET_ID${NC}"
echo "$MARKET_RESPONSE" | jq '.market | {id, question, status, yes_pool_lamports, no_pool_lamports}'
echo ""

# Step 2: List markets
echo -e "${YELLOW}[2/6] Listing markets...${NC}"
MARKETS_RESPONSE=$(curl -s "$BASE_URL/api/futarchy/markets")
MARKET_COUNT=$(echo "$MARKETS_RESPONSE" | jq '.count')
echo -e "${GREEN}Found $MARKET_COUNT market(s)${NC}"
echo "$MARKETS_RESPONSE" | jq '.markets[] | {id, question, status}'
echo ""

# Step 3: Get market details
echo -e "${YELLOW}[3/6] Getting market details...${NC}"
MARKET_DETAIL=$(curl -s "$BASE_URL/api/futarchy/markets/$MARKET_ID")
echo "$MARKET_DETAIL" | jq '.market | {id, question, creator, oracle, status, yes_pool_lamports, no_pool_lamports}'
echo ""

# Step 4: Place YES bet
echo -e "${YELLOW}[4/6] Placing YES bet (100 lamports)...${NC}"
YES_BET_RESPONSE=$(curl -s -X POST "$BASE_URL/api/futarchy/markets/$MARKET_ID/bet" \
    -H "Content-Type: application/json" \
    -d '{
        "bettor": "YesBettor111111111111111111111111111111111",
        "side": "yes",
        "amount_lamports": 100
    }')

if echo "$YES_BET_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    echo -e "${RED}ERROR placing YES bet:${NC}"
    echo "$YES_BET_RESPONSE" | jq .
    exit 1
fi
echo -e "${GREEN}YES bet placed successfully${NC}"
echo "$YES_BET_RESPONSE" | jq '.bet'
echo ""

# Step 5: Place NO bet
echo -e "${YELLOW}[5/6] Placing NO bet (50 lamports)...${NC}"
NO_BET_RESPONSE=$(curl -s -X POST "$BASE_URL/api/futarchy/markets/$MARKET_ID/bet" \
    -H "Content-Type: application/json" \
    -d '{
        "bettor": "NoBettor1111111111111111111111111111111111",
        "side": "no",
        "amount_lamports": 50
    }')

if echo "$NO_BET_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    echo -e "${RED}ERROR placing NO bet:${NC}"
    echo "$NO_BET_RESPONSE" | jq .
    exit 1
fi
echo -e "${GREEN}NO bet placed successfully${NC}"
echo "$NO_BET_RESPONSE" | jq '.bet'
echo ""

# Check pool totals after bets
echo "Checking pool totals..."
MARKET_AFTER_BETS=$(curl -s "$BASE_URL/api/futarchy/markets/$MARKET_ID")
echo "$MARKET_AFTER_BETS" | jq '.market | {yes_pool_lamports, no_pool_lamports}'
echo ""

# Step 6: Settle market (YES wins)
echo -e "${YELLOW}[6/6] Settling market (YES wins)...${NC}"
SETTLE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/futarchy/markets/$MARKET_ID/settle" \
    -H "Content-Type: application/json" \
    -d '{
        "oracle": "DemoOracle111111111111111111111111111111111",
        "outcome": true
    }')

if echo "$SETTLE_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    echo -e "${RED}ERROR settling market:${NC}"
    echo "$SETTLE_RESPONSE" | jq .
    exit 1
fi
echo -e "${GREEN}Market settled!${NC}"
echo "$SETTLE_RESPONSE" | jq .
echo ""

# Final verification
echo "Final market state:"
FINAL_MARKET=$(curl -s "$BASE_URL/api/futarchy/markets/$MARKET_ID")
echo "$FINAL_MARKET" | jq '.market | {id, status, outcome, yes_pool_lamports, no_pool_lamports, settled_at}'
echo ""

# Get positions
echo "Market positions:"
POSITIONS=$(curl -s "$BASE_URL/api/futarchy/markets/$MARKET_ID/positions")
echo "$POSITIONS" | jq '.positions'
echo ""

echo "======================================"
echo -e "${GREEN}  Demo completed successfully!${NC}"
echo "======================================"
echo ""
echo "Summary:"
echo "  - Market ID: $MARKET_ID"
echo "  - YES pool: $(echo "$FINAL_MARKET" | jq '.market.yes_pool_lamports') lamports"
echo "  - NO pool: $(echo "$FINAL_MARKET" | jq '.market.no_pool_lamports') lamports"
echo "  - Outcome: YES wins"
echo ""
echo "Try the health endpoint:"
echo "  curl $BASE_URL/api/futarchy/health"
