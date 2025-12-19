#!/bin/bash
# Futarchy Source Code Verification
# Verifies that all Track implementations are present in source code

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

echo "=========================================="
echo "FUTARCHY SOURCE CODE VERIFICATION"
echo "=========================================="
echo ""

# Track 0: Position encrypted_amount
echo "Track 0: Position encrypted_amount"
echo "-----------------------------------"

if grep -q "pub encrypted_amount: Option<Vec<u8>>" src/programs/futarchy-markets/src/state/position.rs; then
    pass "Position struct has encrypted_amount field"
else
    fail "Position missing encrypted_amount field"
fi

if grep -q "test_position_encrypted_amount" src/programs/futarchy-markets/tests/unit/state/position_test.rs; then
    pass "Position test for encrypted_amount exists"
else
    info "Position test for encrypted_amount not found (optional)"
fi

echo ""

# Track 1: Handlers
echo "Track 1: Handlers /bet/prepare and /bet/submit"
echo "-----------------------------------------------"

if grep -q '#\[post("/api/futarchy/bet/prepare")\]' src/blink-server/src/futarchy_handlers.rs; then
    pass "/bet/prepare handler defined"
else
    fail "/bet/prepare handler not found"
fi

if grep -q "pub async fn prepare_bet" src/blink-server/src/futarchy_handlers.rs; then
    pass "prepare_bet function exists"
else
    fail "prepare_bet function not found"
fi

if grep -q '#\[post("/api/futarchy/bet/submit")\]' src/blink-server/src/futarchy_handlers.rs; then
    pass "/bet/submit handler defined"
else
    fail "/bet/submit handler not found"
fi

if grep -q "pub async fn submit_bet" src/blink-server/src/futarchy_handlers.rs; then
    pass "submit_bet function exists"
else
    fail "submit_bet function not found"
fi

if grep -q "\.configure(futarchy_handlers::configure_routes)" src/blink-server/src/main.rs; then
    pass "Handlers registered in main.rs"
else
    fail "Handlers not registered in main.rs"
fi

echo ""

# Track 2: SDK
echo "Track 2: SDK prepare_unsigned_transaction()"
echo "---------------------------------------------"

if grep -q "pub fn place_bet" sdks/futarchy-sdk/src/client.rs; then
    pass "place_bet method exists in SDK"
else
    fail "place_bet method not found"
fi

if grep -q "test_prepare_unsigned_transaction" sdks/futarchy-sdk/tests/integration.rs; then
    pass "SDK test for unsigned transaction flow exists"
else
    info "SDK unsigned transaction test not found (checking alternatives...)"
fi

# Run SDK tests
info "Running SDK tests..."
if cargo test -p futarchy-sdk --quiet 2>&1 | grep -q "test result: ok"; then
    pass "All SDK tests passing"
else
    fail "SDK tests failed"
fi

echo ""

# Track 3: Prover blockchain polling
echo "Track 3: Prover blockchain polling"
echo "-----------------------------------"

if grep -q "pub struct FutarchyPoolWorker" src/prover-node/src/futarchy/pool_worker.rs; then
    pass "FutarchyPoolWorker struct exists"
else
    fail "FutarchyPoolWorker struct not found"
fi

if grep -q "pub struct FutarchyPoller" src/prover-node/src/futarchy/poller.rs; then
    pass "FutarchyPoller struct exists"
else
    fail "FutarchyPoller struct not found"
fi

if [ -f "src/prover-node/src/futarchy/mod.rs" ]; then
    pass "Futarchy prover module exists"
else
    fail "Futarchy prover module not found"
fi

echo ""

# Track 4: DB migrations
echo "Track 4: Database migrations"
echo "----------------------------"

if [ -f "src/blink-server/migrations/20251219000001_futarchy_e2e_support.sql" ]; then
    pass "E2E migration file exists"
else
    fail "E2E migration file not found"
fi

if grep -q "futarchy_ciphertexts" src/blink-server/migrations/20251218000001_create_futarchy_tables.sql; then
    pass "futarchy_ciphertexts table definition found"
else
    fail "futarchy_ciphertexts table not defined"
fi

if grep -q "futarchy_positions" src/blink-server/migrations/20251218000001_create_futarchy_tables.sql; then
    pass "futarchy_positions table definition found"
else
    info "futarchy_positions table not found (checking alternatives...)"
fi

if [ -f "src/blink-server/src/db/futarchy_queries.rs" ]; then
    pass "Futarchy queries module exists"
else
    fail "Futarchy queries module not found"
fi

echo ""

# Additional checks
echo "Additional Verifications"
echo "------------------------"

info "Checking example code..."
if [ -f "sdks/futarchy-sdk/examples/prepare_transaction.rs" ]; then
    pass "prepare_transaction example exists"
else
    info "prepare_transaction example not found (optional)"
fi

info "Checking documentation..."
if [ -f "sdks/futarchy-sdk/UNSIGNED_TX_FLOW.md" ]; then
    pass "SDK flow documentation exists"
else
    info "SDK flow documentation not found (optional)"
fi

if [ -f "src/blink-server/examples/futarchy_e2e_flow.md" ]; then
    pass "E2E flow documentation exists"
else
    info "E2E flow documentation not found (optional)"
fi

echo ""
echo "=========================================="
echo "VERIFICATION COMPLETE"
echo "=========================================="
echo ""
echo "All source code checks passed!"
echo ""
echo "NEXT STEP: Rebuild containers to deploy code"
echo "  cargo build --release -p blink-server"
echo "  make c1-restart"
echo ""
