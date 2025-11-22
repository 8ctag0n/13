#!/bin/bash
#
# Run all localnet tests
#

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo -e "${YELLOW}Running ZyberLink Tests...${NC}"
echo ""

# Check system is running
if ! curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    echo -e "${RED}[FAIL] Backend not running. Start with: ./start-localnet.sh${NC}"
    exit 1
fi

# Test 1: Backend API tests
echo -e "${YELLOW}[1/2]${NC} Backend API tests (14 tests)..."
if ./localnet-test-api.sh > /tmp/test-api.log 2>&1; then
    echo -e "${GREEN}  [OK] All API tests passed${NC}"
else
    echo -e "${RED}  [FAIL] API tests failed. Check: tail -20 /tmp/test-api.log${NC}"
    exit 1
fi

# Test 2: E2E pricing validation
echo -e "${YELLOW}[2/2]${NC} Dynamic pricing validation..."
RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/estimate-cost \
    -H "Content-Type: application/json" \
    -d '{"operation":"add","operation_value":5,"required_provers":3}')

TIER=$(echo $RESPONSE | jq -r '.complexity_tier')
if [ "$TIER" == "1" ]; then
    echo -e "${GREEN}  [OK] Pricing validation passed${NC}"
else
    echo -e "${RED}  [FAIL] Pricing validation failed${NC}"
    exit 1
fi

echo ""
echo "==========================================="
echo "  All Tests Passed!"
echo "==========================================="
echo ""
