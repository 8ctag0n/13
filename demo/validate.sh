#!/bin/bash
# Quick validation script - checks if everything is ready for demo

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo ""
echo -e "${BLUE}🔍 Zyberlink Demo Validation${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

check_ok=true

# Check binaries
echo -e "${BLUE}[1/5]${NC} Checking required binaries..."
for binary in solana solana-test-validator cargo curl; do
    if command -v $binary &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} $binary"
    else
        echo -e "  ${RED}✗${NC} $binary (not found)"
        check_ok=false
    fi
done

# Check project structure
echo ""
echo -e "${BLUE}[2/5]${NC} Checking project structure..."
for dir in programs/cypherlink witness-storage prover-node sdk; do
    if [ -d "$PROJECT_ROOT/$dir" ]; then
        echo -e "  ${GREEN}✓${NC} $dir/"
    else
        echo -e "  ${RED}✗${NC} $dir/ (not found)"
        check_ok=false
    fi
done

# Check if programs build
echo ""
echo -e "${BLUE}[3/5]${NC} Checking if programs compile..."
cd "$PROJECT_ROOT/programs/cypherlink"
if cargo build-sbf --quiet 2>&1 | grep -q "Finished"; then
    echo -e "  ${GREEN}✓${NC} cypherlink program builds"
else
    echo -e "  ${RED}✗${NC} cypherlink program has build errors"
    check_ok=false
fi

# Check witness storage
cd "$PROJECT_ROOT/witness-storage"
if cargo check --quiet 2>&1; then
    echo -e "  ${GREEN}✓${NC} witness-storage compiles"
else
    echo -e "  ${RED}✗${NC} witness-storage has compile errors"
    check_ok=false
fi

# Check prover node
cd "$PROJECT_ROOT/prover-node"
if cargo check --quiet 2>&1; then
    echo -e "  ${GREEN}✓${NC} prover-node compiles"
else
    echo -e "  ${RED}✗${NC} prover-node has compile errors"
    check_ok=false
fi

# Check keypair
echo ""
echo -e "${BLUE}[4/5]${NC} Checking Solana configuration..."
KEYPAIR_PATH="$HOME/.config/solana/id.json"
if [ -f "$KEYPAIR_PATH" ]; then
    echo -e "  ${GREEN}✓${NC} Keypair exists at $KEYPAIR_PATH"
else
    echo -e "  ${YELLOW}!${NC} Keypair not found (will be generated)"
fi

# Check disk space
echo ""
echo -e "${BLUE}[5/5]${NC} Checking system resources..."
available_space=$(df -h "$PROJECT_ROOT" | tail -1 | awk '{print $4}')
echo -e "  ${GREEN}✓${NC} Available disk space: $available_space"

# Summary
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if [ "$check_ok" = true ]; then
    echo -e "${GREEN}✅ All checks passed! Ready to run demo.${NC}"
    echo ""
    echo -e "Run:  ${BLUE}./demo/run-demo.sh${NC}"
    exit 0
else
    echo -e "${RED}❌ Some checks failed. Please fix the issues above.${NC}"
    exit 1
fi
