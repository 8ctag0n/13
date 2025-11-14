#!/bin/bash
# Prover Node Script - Terminal 2
# Starts the prover with TUI for live monitoring

set -e

# Add Solana to PATH
export PATH=$PATH:$HOME/.local/share/solana/install/active_release/bin

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGS_DIR="$PROJECT_ROOT/logs"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

# Logging functions
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Banner
echo ""
echo -e "${MAGENTA}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║                                                               ║${NC}"
echo -e "${MAGENTA}║         🔐 Zyberlink Prover Node - Terminal 2 🔐              ║${NC}"
echo -e "${MAGENTA}║                                                               ║${NC}"
echo -e "${MAGENTA}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if setup was run
if [ ! -f "$LOGS_DIR/zyberlink_program_id.txt" ]; then
    log_error "Program ID not found. Run ./demo/01-setup.sh first!"
    exit 1
fi

PROGRAM_ID=$(cat "$LOGS_DIR/zyberlink_program_id.txt")
log_success "Found program ID: $PROGRAM_ID"

# Check if validator is running
log_info "Checking Solana validator..."
if ! curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
    http://localhost:8899 | grep -q "ok"; then
    log_error "Validator not running. Run ./demo/01-setup.sh first!"
    exit 1
fi
log_success "Validator is running"

# Fund prover if needed
log_info "Checking prover balance..."
if [ -f "$KEYPAIR_PATH" ]; then
    BALANCE=$(solana balance "$KEYPAIR_PATH" --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
    if (( $(echo "$BALANCE < 1" | bc -l) )); then
        log_info "Low balance ($BALANCE SOL), requesting airdrop..."
        solana airdrop 10 "$KEYPAIR_PATH" --url localhost > /dev/null 2>&1
        BALANCE=$(solana balance "$KEYPAIR_PATH" --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
    fi
    log_success "Prover balance: $BALANCE SOL"
fi

# Build prover (if not already built)
cd "$PROJECT_ROOT/prover-node"

if [ ! -f "../target/release/prover-node" ]; then
    log_info "Building prover node (this will take a few minutes)..."
    log_info "Go grab a coffee ☕ while TFHE and Halo2 compile..."
    cargo build --release
    log_success "Prover built!"
else
    log_info "Using existing prover binary"
fi

# Start prover with TUI
echo ""
log_info "Starting prover node with TUI..."
log_info "Program ID: $PROGRAM_ID"
echo ""
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}  TUI will appear below. Press 'q' to quit.${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
sleep 2

# Run prover with TUI
export PROGRAM_ID="$PROGRAM_ID"
cargo run --release --bin cypherlink-prover -- \
    --rpc-url http://localhost:8899 \
    --program-id "$PROGRAM_ID" \
    --witness-backend-url http://localhost:3030 \
    --keypair "$KEYPAIR_PATH" \
    --tui-mode

echo ""
log_info "Prover stopped"
