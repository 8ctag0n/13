#!/bin/bash
# Integration tests for ZyberLink webapp and backend
# Tests the changes made to connect real data to the frontend

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

BACKEND_URL="${BACKEND_URL:-http://127.0.0.1:8080}"
WEBAPP_DIR="$(dirname "$0")/../src/webapp"

echo ""
echo -e "${CYAN}=========================================${NC}"
echo -e "${CYAN}  ZyberLink Integration Tests${NC}"
echo -e "${CYAN}=========================================${NC}"
echo ""

PASSED=0
FAILED=0

# Helper function
test_result() {
    if [ $1 -eq 0 ]; then
        echo -e "  ${GREEN}[PASS]${NC} $2"
        ((PASSED++))
    else
        echo -e "  ${RED}[FAIL]${NC} $2"
        ((FAILED++))
    fi
}

# ============================================================================
# BACKEND API TESTS
# ============================================================================
echo -e "${YELLOW}[1] Backend API Tests${NC}"

# Test 1.1: Health endpoint
curl -s --max-time 5 "$BACKEND_URL/health" > /dev/null 2>&1
test_result $? "Backend health endpoint responds"

# Test 1.2: Network stats endpoint exists
STATS_RESPONSE=$(curl -s --max-time 5 "$BACKEND_URL/api/stats/network" 2>/dev/null)
if [ -n "$STATS_RESPONSE" ]; then
    test_result 0 "Network stats endpoint responds"
else
    test_result 1 "Network stats endpoint responds"
fi

# Test 1.3: Network stats has required fields
if echo "$STATS_RESPONSE" | grep -q '"active_provers"' && \
   echo "$STATS_RESPONSE" | grep -q '"jobs_total"' && \
   echo "$STATS_RESPONSE" | grep -q '"uptime_percent"'; then
    test_result 0 "Network stats has required fields"
else
    test_result 1 "Network stats has required fields"
fi

# Test 1.4: Jobs endpoint works
JOBS_RESPONSE=$(curl -s --max-time 5 "$BACKEND_URL/api/jobs" 2>/dev/null)
if echo "$JOBS_RESPONSE" | grep -q '"jobs"'; then
    test_result 0 "Jobs endpoint returns job list"
else
    test_result 1 "Jobs endpoint returns job list"
fi

echo ""

# ============================================================================
# FRONTEND CODE TESTS - NO EMOJIS
# ============================================================================
echo -e "${YELLOW}[2] Frontend Code Tests - Emoji Removal${NC}"

# Test for specific emojis we removed
EMOJI_LIST="📋|💡|⚠️|✅|👛|🔐|✍️|🔬|⚡|📊|🔄|📜|📡"

# Test 2.1: GlobalNavigation.svelte - no emojis
if grep -E "$EMOJI_LIST" "$WEBAPP_DIR/src/lib/components/GlobalNavigation.svelte" > /dev/null 2>&1; then
    test_result 1 "GlobalNavigation.svelte has no emojis"
else
    test_result 0 "GlobalNavigation.svelte has no emojis"
fi

# Test 2.2: CreateJob.svelte - no emojis
if grep -E "$EMOJI_LIST" "$WEBAPP_DIR/src/lib/pages/CreateJob.svelte" > /dev/null 2>&1; then
    test_result 1 "CreateJob.svelte has no emojis"
else
    test_result 0 "CreateJob.svelte has no emojis"
fi

# Test 2.3: Timeline.svelte - no emojis
if grep -E "$EMOJI_LIST" "$WEBAPP_DIR/src/lib/components/Timeline.svelte" > /dev/null 2>&1; then
    test_result 1 "Timeline.svelte has no emojis"
else
    test_result 0 "Timeline.svelte has no emojis"
fi

# Test 2.4: StatsBar.svelte - no emojis
if grep -E "$EMOJI_LIST" "$WEBAPP_DIR/src/lib/components/StatsBar.svelte" > /dev/null 2>&1; then
    test_result 1 "StatsBar.svelte has no emojis"
else
    test_result 0 "StatsBar.svelte has no emojis"
fi

echo ""

# ============================================================================
# FRONTEND CODE TESTS - REAL DATA CONNECTIONS
# ============================================================================
echo -e "${YELLOW}[3] Frontend Code Tests - Real Data Integration${NC}"

# Test 3.1: StatsBar fetches from API
if grep -q "api/stats/network" "$WEBAPP_DIR/src/lib/components/StatsBar.svelte"; then
    test_result 0 "StatsBar.svelte fetches from /api/stats/network"
else
    test_result 1 "StatsBar.svelte fetches from /api/stats/network"
fi

# Test 3.2: Landing.svelte fetches provers
if grep -q "api/stats/network" "$WEBAPP_DIR/src/lib/pages/Landing.svelte"; then
    test_result 0 "Landing.svelte fetches network stats"
else
    test_result 1 "Landing.svelte fetches network stats"
fi

# Test 3.3: Dashboard.svelte gets wallet balance
if grep -q "getBalance" "$WEBAPP_DIR/src/lib/pages/Dashboard.svelte"; then
    test_result 0 "Dashboard.svelte gets real wallet balance"
else
    test_result 1 "Dashboard.svelte gets real wallet balance"
fi

# Test 3.4: No hardcoded "47_ACTIVE" provers
if grep -q "47_ACTIVE" "$WEBAPP_DIR/src/lib/pages/Landing.svelte"; then
    test_result 1 "Landing.svelte uses dynamic prover count"
else
    test_result 0 "Landing.svelte uses dynamic prover count"
fi

# Test 3.5: No hardcoded "2.456_SOL" balance
if grep -q "2.456_SOL" "$WEBAPP_DIR/src/lib/pages/Dashboard.svelte"; then
    test_result 1 "Dashboard.svelte uses dynamic wallet balance"
else
    test_result 0 "Dashboard.svelte uses dynamic wallet balance"
fi

echo ""

# ============================================================================
# FRONTEND CODE TESTS - TEXT CONTRAST
# ============================================================================
echo -e "${YELLOW}[4] Frontend Code Tests - Text Contrast${NC}"

# Test 4.1: Improved text-muted color
if grep -q "zyber-text-muted: #94A3B8" "$WEBAPP_DIR/src/styles/tui-system.css"; then
    test_result 0 "Text muted color improved to #94A3B8"
else
    test_result 1 "Text muted color improved to #94A3B8"
fi

echo ""

# ============================================================================
# FRONTEND CODE TESTS - NAVIGATION
# ============================================================================
echo -e "${YELLOW}[5] Frontend Code Tests - App Navigation${NC}"

# Test 5.1: GlobalNavigation has route navigation
if grep -q "type: 'route'" "$WEBAPP_DIR/src/lib/components/GlobalNavigation.svelte"; then
    test_result 0 "GlobalNavigation supports route navigation"
else
    test_result 1 "GlobalNavigation supports route navigation"
fi

# Test 5.2: GlobalNavigation has dashboard link
if grep -q "dashboard" "$WEBAPP_DIR/src/lib/components/GlobalNavigation.svelte"; then
    test_result 0 "GlobalNavigation has dashboard/app link"
else
    test_result 1 "GlobalNavigation has dashboard/app link"
fi

# Test 5.3: GlobalNavigation imports navigateTo
if grep -q "import.*navigateTo" "$WEBAPP_DIR/src/lib/components/GlobalNavigation.svelte"; then
    test_result 0 "GlobalNavigation imports navigateTo"
else
    test_result 1 "GlobalNavigation imports navigateTo"
fi

echo ""

# ============================================================================
# SUMMARY
# ============================================================================
echo -e "${CYAN}=========================================${NC}"
echo -e "${CYAN}  Test Summary${NC}"
echo -e "${CYAN}=========================================${NC}"
echo ""
echo -e "  ${GREEN}Passed:${NC} $PASSED"
echo -e "  ${RED}Failed:${NC} $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
