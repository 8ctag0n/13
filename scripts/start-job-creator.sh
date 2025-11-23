#!/bin/bash
#
# Start the job creator heartbeat
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }

# Check localnet is running
if ! curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    echo "ERROR: Localnet not running. Start with: ./start-localnet.sh"
    exit 1
fi

# Load config
export $(grep -v '^#' blink-server/.env | xargs)

echo ""
echo "==========================================="
echo "  Job Creator - Heartbeat Mode"
echo "==========================================="
echo ""

log_step "Building job-creator..."
cd job-creator
cargo build --release 2>&1 | tail -5
cd ..
log_ok "Built successfully"

log_step "Starting continuous job creation..."
echo ""
echo "This will create a new FHE job every 10 seconds"
echo "Watch the provers claim and process them!"
echo ""
echo "Logs to monitor:"
echo "  - Job creator: This terminal"
echo "  - Provers: tail -f /tmp/prover-*.log"
echo "  - Backend: tail -f /tmp/blink-server.log"
echo ""
echo "Press Ctrl+C to stop"
echo ""

RUST_LOG=info \
SOLANA_RPC_URL=$SOLANA_RPC_URL \
PROGRAM_ID=$PROGRAM_ID \
USER_KEYPAIR=/tmp/job-creator-keypair.json \
cargo run --manifest-path job-creator/Cargo.toml --release
