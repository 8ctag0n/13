#!/bin/bash
#
# Start the dev-job heartbeat
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }

# Check localnet is running
if ! curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    echo "ERROR: Localnet not running. Start with: scripts/start-localnet.sh"
    exit 1
fi

# Load config
export $(grep -v '^#' src/blink-server/.env | xargs)

echo ""
echo "==========================================="
echo "  Dev Job - Heartbeat Mode"
echo "==========================================="
echo ""

log_step "Building zyb-cli..."
cargo build --release -p zyb-cli 2>&1 | tail -5
log_ok "Built successfully"

log_step "Funding dev-job wallet..."

# Create keypair if doesn't exist
if [ ! -f "/tmp/job-creator-keypair.json" ]; then
    echo "Creating dev-job keypair..."
    solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1
fi

JOB_CREATOR_ADDR=$(solana address --keypair /tmp/job-creator-keypair.json)
BALANCE=$(solana balance --keypair /tmp/job-creator-keypair.json --url $SOLANA_RPC_URL 2>/dev/null | awk '{print $1}')

echo "Dev job address: $JOB_CREATOR_ADDR"
echo "Current balance: $BALANCE SOL"

# Airdrop if balance is low (less than 50 SOL)
if (( $(echo "$BALANCE < 50" | bc -l) )); then
    echo "Balance low, requesting airdrop of 100 SOL..."
    solana airdrop 100 $JOB_CREATOR_ADDR --url $SOLANA_RPC_URL >/dev/null 2>&1
    sleep 2
    NEW_BALANCE=$(solana balance --keypair /tmp/job-creator-keypair.json --url $SOLANA_RPC_URL 2>/dev/null | awk '{print $1}')
    log_ok "Funded! New balance: $NEW_BALANCE SOL"
else
    log_ok "Balance sufficient: $BALANCE SOL"
fi

log_step "Starting continuous job creation..."
echo ""
echo "This will create a new FHE job every 10 seconds"
echo "Watch the provers claim and process them!"
echo ""
echo "Logs to monitor:"
echo "  - Dev job: This terminal"
echo "  - Provers: tail -f /tmp/prover-*.log"
echo "  - Backend: tail -f /tmp/blink-server.log"
echo ""
echo "Press Ctrl+C to stop"
echo ""

RUST_LOG=info \
SOLANA_RPC_URL=$SOLANA_RPC_URL \
PROGRAM_ID=$PROGRAM_ID \
USER_KEYPAIR=/tmp/job-creator-keypair.json \
cargo run --release -p zyb-cli -- dev-job run
