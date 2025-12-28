#!/bin/bash
#
# ZyberLink Localnet - Master Start Script
# Simple, reliable, one command to rule them all
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Get the project root directory (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Change to project root
cd "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo "  ZyberLink Localnet Quick Start"
echo "==========================================="
echo ""

# ============================================================================
# STEP 1: Check if already running
# ============================================================================
if solana cluster-version --url http://localhost:8899 >/dev/null 2>&1 && \
   curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    log_ok "System already running!"
    scripts/localnet-status.sh
    exit 0
fi

# ============================================================================
# STEP 2: Clean and setup infrastructure
# ============================================================================
log_step "Setting up infrastructure..."

# Kill old processes
pkill -f solana-test-validator || true
pkill -f blink-server || true
pkill -f zyberlink-prover || true
sleep 2

# Run setup
if scripts/setup-localnet.sh > /tmp/setup.log 2>&1; then
    log_ok "Infrastructure ready"
else
    log_error "Setup failed. Check: tail -50 /tmp/setup.log"
fi

# ============================================================================
# STEP 3: Start backend
# ============================================================================
log_step "Starting backend server..."

export $(grep -v '^#' services/blink-server/.env | xargs)

RUST_LOG=info \
DATABASE_URL=$DATABASE_URL \
SOLANA_RPC_URL=$SOLANA_RPC_URL \
PROGRAM_ID=$PROGRAM_ID \
PORT=8080 \
HOST=127.0.0.1 \
./target/release/blink-server > /tmp/blink-server.log 2>&1 &

sleep 3

if curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    log_ok "Backend started (PID: $!)"
else
    log_error "Backend failed. Check: tail -20 /tmp/blink-server.log"
fi

# ============================================================================
# STEP 4: Fund prover wallets (they're at 0 SOL)
# ============================================================================
log_step "Funding prover wallets..."

for i in 1 2 3; do
    ADDR=$(solana address --keypair /tmp/prover-$i-keypair.json)
    solana airdrop 5 $ADDR --url http://localhost:8899 >/dev/null 2>&1
    log_ok "Prover $i funded (5 SOL)"
done

# ============================================================================
# STEP 5: Build provers if needed
# ============================================================================
PROVER_BIN="./target/release/zyberlink-prover"
if [ ! -f "$PROVER_BIN" ]; then
    log_step "Building prover binary (first time only)..."
    if cargo build --release --bin zyberlink-prover > /tmp/prover-build.log 2>&1; then
        log_ok "Prover built successfully"
    else
        log_error "Prover build failed. Check: tail -50 /tmp/prover-build.log"
    fi
fi

# ============================================================================
# STEP 6: Start provers
# ============================================================================
log_step "Starting prover nodes..."

if scripts/localnet-start-provers.sh > /tmp/provers.log 2>&1; then
    log_ok "3 provers started"
else
    log_error "Provers failed. Check: tail -20 /tmp/provers.log"
fi

# ============================================================================
# STEP 7: Verify everything
# ============================================================================
sleep 2
scripts/localnet-status.sh

echo ""
echo "==========================================="
echo "  System Ready!"
echo "==========================================="
echo ""
echo "Quick commands:"
echo "  - Status:  scripts/localnet-status.sh"
echo "  - Stop:    scripts/stop-localnet.sh"
echo ""
echo "Logs:"
echo "  - Validator: tail -f /tmp/solana-validator.log"
echo "  - Backend:   tail -f /tmp/blink-server.log"
echo "  - Provers:   tail -f /tmp/prover-*.log"
echo ""
