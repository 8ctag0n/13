#!/bin/bash
# Zyberlink Full Demo Orchestration Script
# Starts all services, runs demo, handles cleanup

set -e  # Exit on error
trap cleanup EXIT  # Always cleanup on exit

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Process IDs for cleanup
VALIDATOR_PID=""
WITNESS_PID=""
PROVER_PID=""
DASHBOARD_PID=""

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGS_DIR="$PROJECT_ROOT/logs"
RPC_URL="http://localhost:8899"
WITNESS_URL="http://localhost:3030"
DASHBOARD_URL="http://localhost:3000"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

# Create logs directory if it doesn't exist
mkdir -p "$LOGS_DIR"

# Logging
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_step() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}[$1]${NC} $2"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Cleanup function
cleanup() {
    log_step "CLEANUP" "Shutting down services..."

    if [ ! -z "$DASHBOARD_PID" ]; then
        log_info "Stopping dashboard (PID: $DASHBOARD_PID)..."
        kill $DASHBOARD_PID 2>/dev/null || true
    fi

    if [ ! -z "$PROVER_PID" ]; then
        log_info "Stopping prover node (PID: $PROVER_PID)..."
        kill $PROVER_PID 2>/dev/null || true
    fi

    if [ ! -z "$WITNESS_PID" ]; then
        log_info "Stopping witness storage (PID: $WITNESS_PID)..."
        kill $WITNESS_PID 2>/dev/null || true
    fi

    if [ ! -z "$VALIDATOR_PID" ]; then
        log_info "Stopping Solana validator (PID: $VALIDATOR_PID)..."
        kill $VALIDATOR_PID 2>/dev/null || true
    fi

    # Kill any remaining processes
    pkill -f "solana-test-validator" 2>/dev/null || true
    pkill -f "witness-storage" 2>/dev/null || true
    pkill -f "prover-node" 2>/dev/null || true

    log_success "All services stopped"
}

# Wait for service to be healthy
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    local attempt=1

    log_info "Waiting for $name to be healthy..."

    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url" > /dev/null 2>&1; then
            log_success "$name is ready"
            return 0
        fi
        echo -n "."
        sleep 1
        attempt=$((attempt + 1))
    done

    log_error "$name failed to start (timeout after ${max_attempts}s)"
    return 1
}

# Check if binary exists
check_binary() {
    local binary=$1
    if ! command -v $binary &> /dev/null; then
        log_error "$binary not found. Please install it first."
        exit 1
    fi
}

# Pre-flight checks
preflight_checks() {
    log_step "1/7" "Pre-flight Checks"

    log_info "Checking required binaries..."
    check_binary "solana"
    check_binary "solana-test-validator"
    check_binary "cargo"
    check_binary "curl"

    log_info "Checking project structure..."
    if [ ! -d "$PROJECT_ROOT/programs/cypherlink" ]; then
        log_error "CypherLink program directory not found"
        exit 1
    fi

    if [ ! -d "$PROJECT_ROOT/witness-storage" ]; then
        log_error "Witness storage directory not found"
        exit 1
    fi

    if [ ! -d "$PROJECT_ROOT/prover-node" ]; then
        log_error "Prover node directory not found"
        exit 1
    fi

    log_info "Checking keypair..."
    if [ ! -f "$KEYPAIR_PATH" ]; then
        log_warn "Keypair not found at $KEYPAIR_PATH, generating new one..."
        mkdir -p "$(dirname "$KEYPAIR_PATH")"
        solana-keygen new --no-passphrase -o "$KEYPAIR_PATH"
    fi

    log_success "Pre-flight checks complete"
}

# Start Solana test validator
start_validator() {
    log_step "2/7" "Starting Solana Test Validator"

    # Kill any existing validator
    pkill -f "solana-test-validator" 2>/dev/null || true
    sleep 2

    log_info "Starting validator on port 8899..."
    cd "$PROJECT_ROOT"

    # Start validator in background with logs redirected
    solana-test-validator \
        --rpc-port 8899 \
        --quiet \
        --reset \
        > "$LOGS_DIR/validator.log" 2>&1 &

    VALIDATOR_PID=$!
    log_info "Validator PID: $VALIDATOR_PID"

    # Wait for validator to be ready
    wait_for_service "$RPC_URL/health" "Solana validator"

    # Set Solana config
    solana config set --url localhost > /dev/null 2>&1

    # Show validator info
    local slot=$(solana slot 2>/dev/null || echo "N/A")
    log_success "Validator ready (slot: $slot)"
}

# Build and deploy program
deploy_program() {
    log_step "3/7" "Building & Deploying CypherLink Program"

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

    # Deploy and capture program ID
    DEPLOY_OUTPUT=$(solana program deploy $PROGRAM_PATH --url localhost --keypair "$KEYPAIR_PATH" 2>&1)
    PROGRAM_ID=$(echo "$DEPLOY_OUTPUT" | grep -oP 'Program Id: \K[A-Za-z0-9]+' || echo "")

    if [ -z "$PROGRAM_ID" ]; then
        log_error "Failed to extract program ID"
        log_error "Deploy output: $DEPLOY_OUTPUT"
        exit 1
    fi

    log_success "Program deployed: $PROGRAM_ID"

    # Save program ID for later use
    echo "$PROGRAM_ID" > "$LOGS_DIR/zyberlink_program_id.txt"
}

# Start witness storage backend
start_witness_storage() {
    log_step "4/7" "Starting Witness Storage Backend"

    cd "$PROJECT_ROOT/witness-storage"

    log_info "Building witness storage..."
    cargo build --release --quiet

    log_info "Starting witness storage on port 3030..."
    cargo run --release > "$LOGS_DIR/witness-storage.log" 2>&1 &
    WITNESS_PID=$!
    log_info "Witness storage PID: $WITNESS_PID"

    # Wait for witness storage to be ready
    wait_for_service "$WITNESS_URL/health" "Witness storage"

    # Check health endpoint
    local health=$(curl -s "$WITNESS_URL/health" | grep -o '"status":"[^"]*"' || echo "")
    log_success "Witness storage ready ($health)"
}

# Start prover node
start_prover() {
    log_step "5/7" "Starting Prover Node"

    cd "$PROJECT_ROOT/prover-node"

    log_info "Building prover node..."
    cargo build --release --quiet

    # Fund prover keypair
    log_info "Funding prover keypair..."
    solana airdrop 10 "$KEYPAIR_PATH" --url localhost > /dev/null 2>&1 || true

    local balance=$(solana balance "$KEYPAIR_PATH" --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
    log_info "Prover balance: $balance SOL"

    log_info "Starting prover node with TUI..."
    PROGRAM_ID=$(cat "$LOGS_DIR/zyberlink_program_id.txt")

    # Check if we should run with TUI (if terminal supports it)
    if [ -t 1 ] && [ -z "$NO_TUI" ]; then
        log_info "TUI mode enabled (use NO_TUI=1 to disable)"
        cargo run --release -- \
            --rpc-url "$RPC_URL" \
            --program-id "$PROGRAM_ID" \
            --keypair "$KEYPAIR_PATH" \
            --witness-backend-url "$WITNESS_URL" \
            --poll-interval 2 \
            --mock-proving-time 5 \
            --tui-mode
    else
        log_info "Headless mode (logging to $LOGS_DIR/prover-node.log)"
        cargo run --release -- \
            --rpc-url "$RPC_URL" \
            --program-id "$PROGRAM_ID" \
            --keypair "$KEYPAIR_PATH" \
            --witness-backend-url "$WITNESS_URL" \
            --poll-interval 2 \
            --mock-proving-time 5 \
            > $LOGS_DIR/prover-node.log 2>&1 &
    fi

    PROVER_PID=$!
    log_info "Prover node PID: $PROVER_PID"

    sleep 3

    # Check if prover is still running
    if ! kill -0 $PROVER_PID 2>/dev/null; then
        log_error "Prover node failed to start"
        log_error "Check logs: tail -f $LOGS_DIR/prover-node.log"
        exit 1
    fi

    log_success "Prover node started"
}

# Register prover on-chain
register_prover() {
    log_step "6/7" "Registering Prover On-Chain"

    log_info "Initializing marketplace..."
    # TODO: Call initialize instruction via SDK or CLI

    log_info "Registering prover..."
    # TODO: Call register_prover instruction

    log_warn "Registration step not yet automated (requires SDK integration)"
    log_info "Prover will automatically register on first job claim"
}

# Run test workflow
run_test_workflow() {
    log_step "7/7" "Running Test Workflow"

    log_info "Creating test job..."
    # TODO: Use SDK to create a job

    log_info "Waiting for prover to claim and complete job..."

    local max_wait=60
    local waited=0

    while [ $waited -lt $max_wait ]; do
        # Check prover logs for completion
        if grep -q "Job completed" $LOGS_DIR/prover-node.log 2>/dev/null; then
            log_success "Job completed!"
            break
        fi

        echo -n "."
        sleep 2
        waited=$((waited + 2))
    done

    if [ $waited -ge $max_wait ]; then
        log_warn "Job did not complete within ${max_wait}s"
        log_info "Check prover logs: tail -f $LOGS_DIR/prover-node.log"
    fi
}

# Show summary
show_summary() {
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}                  ✅ DEMO ENVIRONMENT READY                     ${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${CYAN}Services Running:${NC}"
    echo -e "  • Solana Validator:    ${GREEN}$RPC_URL${NC}"
    echo -e "  • Witness Storage:     ${GREEN}$WITNESS_URL${NC}"
    echo -e "  • Prover Node:         ${GREEN}Running (PID: $PROVER_PID)${NC}"
    echo ""
    echo -e "${CYAN}Program Information:${NC}"
    echo -e "  • Program ID:          ${GREEN}$(cat $LOGS_DIR/zyberlink_program_id.txt)${NC}"
    echo -e "  • Prover Keypair:      ${GREEN}$KEYPAIR_PATH${NC}"
    echo ""
    echo -e "${CYAN}Logs:${NC}"
    echo -e "  • Validator:           tail -f $LOGS_DIR/validator.log"
    echo -e "  • Witness Storage:     tail -f $LOGS_DIR/witness-storage.log"
    echo -e "  • Prover Node:         tail -f $LOGS_DIR/prover-node.log"
    echo ""
    echo -e "${YELLOW}Press Ctrl+C to stop all services and cleanup${NC}"
    echo ""
}

# Main execution
main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                                                               ║${NC}"
    echo -e "${CYAN}║           🔐 Zyberlink Demo Orchestration Script 🔐           ║${NC}"
    echo -e "${CYAN}║                                                               ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    preflight_checks
    start_validator
    deploy_program
    start_witness_storage
    start_prover
    register_prover
    run_test_workflow

    show_summary

    # Keep script running
    log_info "Demo environment is running. Logs are in: $LOGS_DIR"
    log_info "Press Ctrl+C to stop..."

    # Wait indefinitely (cleanup will run on EXIT)
    while true; do
        sleep 1
    done
}

# Run main
main "$@"
