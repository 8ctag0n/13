#!/bin/bash
#
# Master script to start entire ZyberLink localnet
# Usage: ./localnet-start-all.sh
#

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_step() {
    echo -e "\n${GREEN}===${NC} $1 ${GREEN}===${NC}\n"
}

# ============================================================================
# STEP 1: Infrastructure Setup
# ============================================================================
log_step "STEP 1: Infrastructure Setup"

if [ ! -f "blink-server/.env" ]; then
    log_info "Running initial setup..."
    ./localnet-setup.sh
else
    log_info "Environment files exist, checking infrastructure..."

    # Check if validator is running
    if ! solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
        log_info "Validator not running, restarting infrastructure..."
        ./localnet-setup.sh
    else
        log_info "✓ Validator already running"
        log_info "✓ Skipping setup"
    fi
fi

# ============================================================================
# STEP 2: Start Backend
# ============================================================================
log_step "STEP 2: Start Backend Server"

./localnet-start-backend.sh

# ============================================================================
# STEP 3: Start Provers
# ============================================================================
log_step "STEP 3: Start Prover Nodes"

./localnet-start-provers.sh

# ============================================================================
# STEP 4: Run API Tests
# ============================================================================
log_step "STEP 4: Validate Backend API"

log_info "Running API validation tests..."
./localnet-test-api.sh

# ============================================================================
# Summary
# ============================================================================
log_step "🎉 All Services Started!"

echo "📋 Services:"
echo "  • Validator:    http://localhost:8899"
echo "  • Backend API:  http://127.0.0.1:8080"
echo "  • Database:     postgresql://localhost:5432/zyberlink"
echo ""
echo "👥 Provers:"
for i in 1 2 3; do
    ADDR=$(solana address --keypair /tmp/prover-$i-keypair.json 2>/dev/null || echo "N/A")
    echo "  • Prover $i:    $ADDR"
done
echo ""
echo "📝 Logs:"
echo "  • Validator:    tail -f /tmp/solana-validator.log"
echo "  • Backend:      tail -f /tmp/blink-server.log"
echo "  • Prover 1:     tail -f /tmp/prover-1.log"
echo "  • Prover 2:     tail -f /tmp/prover-2.log"
echo "  • Prover 3:     tail -f /tmp/prover-3.log"
echo ""
echo "🛑 To stop all services:"
echo "  ./localnet-stop-all.sh"
echo ""
