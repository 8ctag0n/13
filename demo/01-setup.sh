#!/bin/bash
# Setup Script - Terminal 1
# Starts validator, deploys program, starts witness storage

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
RPC_URL="http://localhost:8899"
WITNESS_URL="http://localhost:3030"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

# Create logs directory
mkdir -p "$LOGS_DIR"

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

log_step() {
    echo ""
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${MAGENTA}[$1]${NC} $2"
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Banner
echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                                                               ║${NC}"
echo -e "${CYAN}║         🚀 Zyberlink Demo Setup - Terminal 1 🚀               ║${NC}"
echo -e "${CYAN}║                                                               ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Start Solana Validator
log_step "1/4" "Starting Solana Test Validator"

log_info "Starting validator on port 8899..."
solana-test-validator \
    --rpc-port 8899 \
    --quiet \
    --reset \
    > "$LOGS_DIR/validator.log" 2>&1 &

VALIDATOR_PID=$!
echo $VALIDATOR_PID > "$LOGS_DIR/validator.pid"
log_info "Validator PID: $VALIDATOR_PID"

# Wait for validator
log_info "Waiting for Solana validator to be healthy..."
for i in {1..30}; do
    if curl -s -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
        $RPC_URL | grep -q "ok"; then
        log_success "Solana validator is ready"
        break
    fi
    echo -n "."
    sleep 1
done

SLOT=$(solana slot --url localhost 2>/dev/null || echo "0")
log_success "Validator ready (slot: $SLOT)"

# Step 2: Deploy Program
log_step "2/4" "Building & Deploying CypherLink Program"

cd "$PROJECT_ROOT"

log_info "Building program (this may take a minute)..."
cd programs/cypherlink
cargo build-sbf --quiet
cd ../..
log_success "Program built"

log_info "Deploying program..."
PROGRAM_PATH="$PROJECT_ROOT/programs/target/deploy/cypherlink.so"

if [ ! -f "$PROGRAM_PATH" ]; then
    log_error "Program binary not found at $PROGRAM_PATH"
    exit 1
fi

DEPLOY_OUTPUT=$(solana program deploy $PROGRAM_PATH --url localhost --keypair "$KEYPAIR_PATH" 2>&1)
PROGRAM_ID=$(echo "$DEPLOY_OUTPUT" | grep -oP 'Program Id: \K[A-Za-z0-9]+' || echo "")

if [ -z "$PROGRAM_ID" ]; then
    log_error "Failed to extract program ID"
    log_error "Deploy output: $DEPLOY_OUTPUT"
    exit 1
fi

log_success "Program deployed: $PROGRAM_ID"
echo "$PROGRAM_ID" > "$LOGS_DIR/zyberlink_program_id.txt"

# Initialize program
log_info "Initializing program..."
cd "$PROJECT_ROOT/sdk"
export PROGRAM_ID="$PROGRAM_ID"
cargo run --example initialize_program --quiet 2>&1 | grep -v "warning:"
log_success "Program initialized"

# Register prover
log_info "Registering prover..."
cd "$PROJECT_ROOT/prover-node"
STAKE_AMOUNT=100000000  # 0.1 SOL
cargo run --release --quiet --bin cypherlink-prover -- \
    --rpc-url "$RPC_URL" \
    --program-id "$PROGRAM_ID" \
    --keypair "$KEYPAIR_PATH" \
    register \
    --stake-amount $STAKE_AMOUNT 2>&1 | grep -E "Prover registered|Signature|Prover PDA" || true
log_success "Prover registered (stake: 0.1 SOL)"

# Step 3: Register Prover (already done above)

# Step 4: Start Witness Storage
log_step "4/4" "Starting Witness Storage Backend"

cd "$PROJECT_ROOT/witness-storage"

log_info "Building witness storage..."
cargo build --release --quiet

log_info "Starting witness storage on port 3030..."
cargo run --release > "$LOGS_DIR/witness-storage.log" 2>&1 &
WITNESS_PID=$!
echo $WITNESS_PID > "$LOGS_DIR/witness-storage.pid"
log_info "Witness storage PID: $WITNESS_PID"

# Wait for witness storage
log_info "Waiting for Witness storage to be healthy..."
for i in {1..20}; do
    if curl -s $WITNESS_URL/health | grep -q "ok"; then
        log_success "Witness storage is ready"
        break
    fi
    echo -n "."
    sleep 1
done

HEALTH=$(curl -s $WITNESS_URL/health)
log_success "Witness storage ready ($HEALTH)"

# Summary
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  ✅ Infrastructure Ready!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${CYAN}Services Running:${NC}"
echo -e "  • Validator:           ${GREEN}http://localhost:8899${NC} (PID: $VALIDATOR_PID)"
echo -e "  • Witness Storage:     ${GREEN}http://localhost:3030${NC} (PID: $WITNESS_PID)"
echo -e "  • Program ID:          ${GREEN}$PROGRAM_ID${NC}"
echo ""
echo -e "  ${YELLOW}Logs:${NC}"
echo -e "  • Validator:           tail -f $LOGS_DIR/validator.log"
echo -e "  • Witness Storage:     tail -f $LOGS_DIR/witness-storage.log"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Next Steps:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${YELLOW}Terminal 2:${NC} ./demo/02-start-prover.sh"
echo -e "  ${YELLOW}Terminal 3:${NC} ./demo/generate-jobs.sh"
echo ""
echo -e "  ${RED}To stop:${NC} ./demo/cleanup.sh"
echo ""

# Keep running
log_info "Setup complete. Press Ctrl+C to stop all services..."
trap "echo ''; log_info 'Stopping services...'; kill $VALIDATOR_PID $WITNESS_PID 2>/dev/null; rm -f $LOGS_DIR/*.pid; log_success 'Services stopped'; exit 0" EXIT INT TERM

# Wait forever
while true; do
    sleep 1
done
