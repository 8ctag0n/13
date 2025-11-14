#!/bin/bash
# Quick test script - minimal demo without full orchestration
# Good for development/debugging

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RPC_URL="http://localhost:8899"

echo -e "${CYAN}🧪 Zyberlink Quick Test${NC}"
echo ""

# Check if validator is running
echo -e "${CYAN}[1/3]${NC} Checking Solana validator..."
if curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' $RPC_URL | grep -q "ok"; then
    echo -e "  ${GREEN}✓${NC} Validator is running"
else
    echo -e "  ${RED}✗${NC} Validator not running. Start with:"
    echo "      solana-test-validator --rpc-port 8899"
    exit 1
fi

# Check witness storage
echo -e "${CYAN}[2/3]${NC} Checking witness storage..."
if curl -s http://localhost:3030/health | grep -q "healthy"; then
    echo -e "  ${GREEN}✓${NC} Witness storage is running"
else
    echo -e "  ${RED}✗${NC} Witness storage not running. Start with:"
    echo "      cd witness-storage && cargo run --release"
    exit 1
fi

# Build check
echo -e "${CYAN}[3/3]${NC} Quick build check..."
cd "$PROJECT_ROOT/programs/cypherlink"
if cargo check --quiet 2>&1; then
    echo -e "  ${GREEN}✓${NC} Program compiles"
else
    echo -e "  ${RED}✗${NC} Program has errors"
    exit 1
fi

echo ""
echo -e "${GREEN}✅ Basic components are ready!${NC}"
echo ""
echo "Next steps:"
echo "  1. Deploy program:  solana program deploy target/deploy/cypherlink.so"
echo "  2. Start prover:    cd prover-node && cargo run --release"
echo "  3. Or run full demo: ./demo/run-demo.sh"
