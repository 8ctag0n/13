#!/bin/bash
# Prover Node with Logs - Terminal 2
# Shows prover logs instead of TUI for debugging

set -e

# Add Solana to PATH
export PATH=$PATH:$HOME/.local/share/solana/install/active_release/bin

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGS_DIR="$PROJECT_ROOT/logs"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║         🔐 Zyberlink Prover (Logs Mode) - Terminal 2 🔐       ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check setup
if [ ! -f "$LOGS_DIR/zyberlink_program_id.txt" ]; then
    echo "Error: Run ./demo/01-setup.sh first!"
    exit 1
fi

PROGRAM_ID=$(cat "$LOGS_DIR/zyberlink_program_id.txt")
log_info "Program ID: $PROGRAM_ID"

# Fund prover
BALANCE=$(solana balance "$KEYPAIR_PATH" --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
if (( $(echo "$BALANCE < 1" | bc -l) )); then
    log_info "Requesting airdrop..."
    solana airdrop 10 "$KEYPAIR_PATH" --url localhost > /dev/null 2>&1
fi
log_info "Balance: $(solana balance "$KEYPAIR_PATH" --url localhost)"

echo ""
log_info "Starting prover with LOGS visible..."
log_info "You'll see polling activity every 5 seconds"
echo ""

cd "$PROJECT_ROOT/prover-node"

# Run prover WITHOUT TUI to see logs
export RUST_LOG=info
export PROGRAM_ID="$PROGRAM_ID"

cargo run --release --bin cypherlink-prover -- \
    --rpc-url http://localhost:8899 \
    --program-id "$PROGRAM_ID" \
    --witness-backend-url http://localhost:3030 \
    --keypair "$KEYPAIR_PATH" \
    --poll-interval 5
