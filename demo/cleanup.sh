#!/bin/bash
# Cleanup Script - Stop all demo services

# Add Solana to PATH
export PATH=$PATH:$HOME/.local/share/solana/install/active_release/bin

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGS_DIR="$PROJECT_ROOT/logs"

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

echo ""
echo -e "${YELLOW}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║                                                               ║${NC}"
echo -e "${YELLOW}║              🧹 Zyberlink Demo Cleanup 🧹                     ║${NC}"
echo -e "${YELLOW}║                                                               ║${NC}"
echo -e "${YELLOW}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

log_info "Stopping all demo services..."

# Kill by PID files
if [ -f "$LOGS_DIR/validator.pid" ]; then
    VALIDATOR_PID=$(cat "$LOGS_DIR/validator.pid")
    kill $VALIDATOR_PID 2>/dev/null && log_info "Stopped validator (PID: $VALIDATOR_PID)"
    rm "$LOGS_DIR/validator.pid"
fi

if [ -f "$LOGS_DIR/witness-storage.pid" ]; then
    WITNESS_PID=$(cat "$LOGS_DIR/witness-storage.pid")
    kill $WITNESS_PID 2>/dev/null && log_info "Stopped witness storage (PID: $WITNESS_PID)"
    rm "$LOGS_DIR/witness-storage.pid"
fi

# Kill by process name (backup)
pkill -f "solana-test-validator" 2>/dev/null && log_info "Killed solana-test-validator"
pkill -f "witness-storage" 2>/dev/null && log_info "Killed witness-storage"
pkill -f "prover-node.*--tui-mode" 2>/dev/null && log_info "Killed prover-node"

# Wait a bit
sleep 1

# Check if anything is still running
REMAINING=$(ps aux | grep -E "(solana-test-validator|witness-storage|prover-node)" | grep -v grep | wc -l)

if [ $REMAINING -eq 0 ]; then
    log_success "All services stopped successfully"
else
    echo -e "${YELLOW}[!]${NC} Some processes may still be running:"
    ps aux | grep -E "(solana-test-validator|witness-storage|prover-node)" | grep -v grep
fi

echo ""
log_info "Logs preserved in: $LOGS_DIR"
echo ""
