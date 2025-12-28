#!/bin/bash
#
# Launch 3 prover nodes for localnet testing
#

set -e

GREEN='\033[0;32m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

# Get project root (two levels up from deployment/scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
cd "$PROJECT_ROOT"

# Check prerequisites - try multiple locations for env
ENV_FILE=""
if [ -f "services/blink-server/.env" ]; then
    ENV_FILE="services/blink-server/.env"
elif [ -f ".env.containers" ]; then
    ENV_FILE=".env.containers"
else
    echo "ERROR: Run deployment/scripts/setup-localnet.sh first"
    exit 1
fi

# Load PROGRAM_ID from env
export $(grep PROGRAM_ID $ENV_FILE | head -1 | xargs)
SOLANA_RPC_URL=${SOLANA_RPC_URL:-http://localhost:8899}

log_info "Starting 3 prover nodes..."
log_info "Program ID: $PROGRAM_ID"
log_info "RPC URL: $SOLANA_RPC_URL"

# Find the prover binary
PROVER_BIN="$PROJECT_ROOT/target/release/zyberlink-prover"
if [ ! -f "$PROVER_BIN" ]; then
    echo "ERROR: Prover binary not found at $PROVER_BIN"
    echo "Build it first: cargo build --release --bin zyberlink-prover"
    exit 1
fi

for i in 1 2 3; do
    # Keypairs are in /tmp from setup-localnet.sh
    PROVER_KEYPAIR="/tmp/prover-$i-keypair.json"

    if [ ! -f "$PROVER_KEYPAIR" ]; then
        log_info "Creating keypair for prover $i..."
        solana-keygen new --no-bip39-passphrase --force --outfile "$PROVER_KEYPAIR" >/dev/null 2>&1
        solana airdrop 5 $(solana address --keypair "$PROVER_KEYPAIR") --url $SOLANA_RPC_URL >/dev/null 2>&1 || true
    fi

    log_info "Starting Prover $i..."

    RUST_LOG=info $PROVER_BIN \
        --program-id $PROGRAM_ID \
        --rpc-url $SOLANA_RPC_URL \
        --keypair $PROVER_KEYPAIR \
        > /tmp/prover-$i.log 2>&1 &

    PROVER_PID=$!
    echo "  PID: $PROVER_PID"
    echo "  Logs: tail -f /tmp/prover-$i.log"
done

log_info "\n✓ All provers started"
log_info "\nMonitor all provers:"
echo "  tail -f /tmp/prover-*.log"
log_info "\nStop all provers:"
echo "  pkill -f zyberlink-prover"
