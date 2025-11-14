#!/bin/bash
# Job Generator - Creates test jobs automatically for demo
# Run this while prover TUI is running to see live updates

set -e

# Add Solana to PATH
export PATH=$PATH:$HOME/.local/share/solana/install/active_release/bin

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGS_DIR="$PROJECT_ROOT/logs"
RPC_URL="http://localhost:8899"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

# Configuration
DELAY_BETWEEN_JOBS=5  # seconds between job creation
TOTAL_JOBS=10         # how many jobs to create
PROGRAM_ID=""

log_job() {
    echo -e "${CYAN}[JOB $1/$TOTAL_JOBS]${NC} $2"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_info() {
    echo -e "${YELLOW}→${NC} $1"
}

# Banner
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                                                               ║${NC}"
echo -e "${GREEN}║         🎲 Zyberlink Job Generator - Terminal 3 🎲            ║${NC}"
echo -e "${GREEN}║                                                               ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if program ID file exists
if [ -f $LOGS_DIR/zyberlink_program_id.txt ]; then
    PROGRAM_ID=$(cat $LOGS_DIR/zyberlink_program_id.txt)
    log_success "Found program ID: $PROGRAM_ID"
else
    echo -e "${YELLOW}[!]${NC} Program ID not found. Run ./demo/run-demo.sh first."
    echo ""
    echo "Or provide program ID manually:"
    read -p "Enter program ID: " PROGRAM_ID
fi

# Check validator is running
log_info "Checking Solana validator..."
if ! curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
    $RPC_URL | grep -q "ok"; then
    echo -e "${RED}✗${NC} Validator not running on $RPC_URL"
    echo "Start with: solana-test-validator --rpc-port 8899"
    exit 1
fi
log_success "Validator is running"

# Fund user if needed
log_info "Checking user balance..."
BALANCE=$(solana balance $KEYPAIR_PATH --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
if (( $(echo "$BALANCE < 1" | bc -l) )); then
    log_info "Low balance ($BALANCE SOL), requesting airdrop..."
    solana airdrop 10 $KEYPAIR_PATH --url localhost > /dev/null 2>&1
    BALANCE=$(solana balance $KEYPAIR_PATH --url localhost 2>/dev/null | grep -oP '[0-9.]+' || echo "0")
fi
log_success "User balance: $BALANCE SOL"

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Starting job generation (${TOTAL_JOBS} jobs, ${DELAY_BETWEEN_JOBS}s delay)${NC}"
echo -e "${CYAN}  Watch the Prover TUI to see jobs being processed!${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Job generation loop
cd "$PROJECT_ROOT"

for i in $(seq 1 $TOTAL_JOBS); do
    log_job $i "Creating job..."

    # Randomize job type (only 2 types now: zk and fhe)
    JOB_TYPE=$((RANDOM % 2))

    case $JOB_TYPE in
        0)
            JOB_NAME="ZK Zcash Orchard"
            CIRCUIT_TYPE="zk"
            PRICE=15000000  # 0.015 SOL
            ;;
        1)
            JOB_NAME="FHE Add Operation"
            CIRCUIT_TYPE="fhe"
            PRICE=50000000  # 0.05 SOL
            ;;
    esac

    log_info "Type: $JOB_NAME | Price: $(echo "scale=4; $PRICE / 1000000000" | bc) SOL"

    # Create job using SDK example
    cd sdk

    # Export configuration for SDK
    export SOLANA_RPC_URL="$RPC_URL"
    export PROGRAM_ID="$PROGRAM_ID"
    export KEYPAIR_PATH="$KEYPAIR_PATH"

    # Run SDK example to create job
    if cargo run --example create_test_job --quiet -- "$CIRCUIT_TYPE" "$PRICE" 2>&1 | grep -q "Job created successfully"; then
        log_success "Job #$i created and submitted"
    else
        echo -e "${YELLOW}[!]${NC} Job creation may have failed, check prover logs"
    fi

    cd ..

    if [ $i -lt $TOTAL_JOBS ]; then
        echo -e "${CYAN}⏱${NC}  Waiting ${DELAY_BETWEEN_JOBS}s before next job..."
        echo ""
        sleep $DELAY_BETWEEN_JOBS
    fi
done

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  ✅ All ${TOTAL_JOBS} jobs created!${NC}"
echo -e "${GREEN}  Watch the Prover TUI to see them being processed.${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
