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

# Check prerequisites
if [ ! -f "src/blink-server/.env" ]; then
    echo "ERROR: Run scripts/setup-localnet.sh first"
    exit 1
fi

# Load PROGRAM_ID from backend .env
export $(grep PROGRAM_ID src/blink-server/.env | xargs)
export $(grep SOLANA_RPC_URL src/blink-server/.env | xargs)

log_info "Starting 3 prover nodes..."
log_info "Program ID: $PROGRAM_ID"
log_info "RPC URL: $SOLANA_RPC_URL"

# Find the prover binary (use relative path from project root)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROVER_BIN="$PROJECT_ROOT/target/release/zyberlink-prover"
if [ ! -f "$PROVER_BIN" ]; then
    echo "ERROR: Prover binary not found at $PROVER_BIN"
    echo "Build it first: cargo build --release --bin zyberlink-prover"
    exit 1
fi

for i in 1 2 3; do
    PROVER_KEYPAIR="$PROJECT_ROOT/keypairs/prover-$i.json"

    if [ ! -f "$PROVER_KEYPAIR" ]; then
        echo "ERROR: Prover $i keypair not found at $PROVER_KEYPAIR"
        echo "Generate keypairs first: for i in 1 2 3; do solana-keygen new --no-bip39-passphrase -o keypairs/prover-\$i.json; done"
        exit 1
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
echo "  pkill -f prover-node"
