#!/bin/bash
# ===========================================
# ZyberLink Wallet E2E Test Runner
# ===========================================
#
# Corre los tests e2e en contenedores Docker
# con el testnet local (Solana, Starknet, Zcash)
#
# Usage:
#   ./run-e2e-tests.sh           # Corre todos los tests
#   ./run-e2e-tests.sh wallet    # Solo tests de wallet
#   ./run-e2e-tests.sh zcash     # Solo tests de Zcash shielded
#   ./run-e2e-tests.sh testnet   # Solo tests de testnet
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}"
echo "╔══════════════════════════════════════════╗"
echo "║   ZyberLink Wallet E2E Tests             ║"
echo "╚══════════════════════════════════════════╝"
echo -e "${NC}"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}[cleanup] Stopping containers...${NC}"
    docker-compose -f docker-compose.test.yml down --volumes --remove-orphans 2>/dev/null || true
}

trap cleanup EXIT

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}[error] Docker not installed${NC}"
    exit 1
fi

if ! docker info &> /dev/null; then
    echo -e "${RED}[error] Docker daemon not running${NC}"
    exit 1
fi

# Build and run
echo -e "${YELLOW}[1/3] Building extension and test container...${NC}"
docker-compose -f docker-compose.test.yml build e2e-tests

echo -e "${YELLOW}[2/3] Starting testnet services...${NC}"
docker-compose -f docker-compose.test.yml up -d solana starknet zcash

echo -e "${YELLOW}[3/3] Waiting for services to be healthy...${NC}"
sleep 5

# Check services
echo -n "  Solana: "
docker-compose -f docker-compose.test.yml exec -T solana curl -sf http://localhost:8899/health && echo -e "${GREEN}OK${NC}" || echo -e "${RED}FAIL${NC}"

echo -n "  Starknet: "
docker-compose -f docker-compose.test.yml exec -T starknet curl -sf http://localhost:5050 && echo -e "${GREEN}OK${NC}" || echo -e "${RED}(starting)${NC}"

echo -n "  Zcash: "
docker-compose -f docker-compose.test.yml exec -T zcash zcash-cli -regtest -rpcuser=zyberlink -rpcpassword=testpass123 getblockcount && echo -e "${GREEN}OK${NC}" || echo -e "${YELLOW}(mining first blocks)${NC}"

echo -e "\n${GREEN}[run] Starting e2e tests...${NC}\n"

# Determine which tests to run
TEST_FILTER=""
case "${1:-all}" in
    wallet)
        TEST_FILTER="wallet.spec.ts"
        ;;
    zcash)
        TEST_FILTER="zcash-shielded.spec.ts"
        ;;
    testnet)
        TEST_FILTER="testnet-integration.spec.ts"
        ;;
    all|*)
        TEST_FILTER=""
        ;;
esac

# Run tests
if [ -n "$TEST_FILTER" ]; then
    docker-compose -f docker-compose.test.yml run --rm e2e-tests sh -c "Xvfb :99 -screen 0 1280x720x24 & sleep 2 && npm test -- $TEST_FILTER --reporter=list"
else
    docker-compose -f docker-compose.test.yml run --rm e2e-tests sh -c "Xvfb :99 -screen 0 1280x720x24 & sleep 2 && npm test -- --reporter=list"
fi

EXIT_CODE=$?

echo -e "\n${GREEN}════════════════════════════════════════════${NC}"
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}[done] All tests passed!${NC}"
else
    echo -e "${RED}[fail] Some tests failed (exit code: $EXIT_CODE)${NC}"
fi
echo -e "${GREEN}════════════════════════════════════════════${NC}"

exit $EXIT_CODE
