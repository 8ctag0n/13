#!/bin/bash
#
# Stop all ZyberLink localnet services
#

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_info "Stopping all ZyberLink localnet services..."

# Stop validator
log_info "Stopping Solana test validator..."
pkill -f solana-test-validator || true

# Stop backend
log_info "Stopping blink-server..."
pkill -f blink-server || true

# Stop provers
log_info "Stopping prover nodes..."
pkill -f cypherlink-prover || true
pkill -f prover-node || true

# Give processes time to cleanup
sleep 2

log_info "✓ All services stopped"
echo ""
log_info "To restart: ./localnet-start-all.sh"
echo ""
