#!/bin/bash
#
# E2E Testing - Start Stack Script
# Starts complete E2E testing environment:
#   - Solana test validator
#   - Deploy programs (bedrock, zk-generator, fhe-generator)
#   - Fund prover wallets
#   - Start backend with podman-compose
#

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}===${NC} $1 ${BLUE}===${NC}\n"
}

# Check we're in project root
if [ ! -f "Cargo.toml" ] || [ ! -d "src/programs" ]; then
    log_error "Must run from project root"
    exit 1
fi

log_step "E2E Stack - Starting Services"

# ============================================================================
# STEP 1: Start Solana Test Validator
# ============================================================================
log_step "STEP 1: Starting Solana Test Validator"

LEDGER_DIR="$HOME/.zyberlink-e2e-ledger"
VALIDATOR_LOG="/tmp/e2e-validator.log"
VALIDATOR_PID="/tmp/e2e-validator.pid"

# Kill existing validator
if [ -f "$VALIDATOR_PID" ]; then
    OLD_PID=$(cat "$VALIDATOR_PID")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        log_warn "Stopping existing validator (PID: $OLD_PID)"
        kill "$OLD_PID" 2>/dev/null || true
        sleep 2
    fi
fi

# Clean old ledger
log_info "Cleaning old ledger data..."
rm -rf "$LEDGER_DIR" 2>/dev/null || true

# Start validator
log_info "Starting validator on port 8899..."
solana-test-validator \
    --reset \
    --rpc-port 8899 \
    --ledger "$LEDGER_DIR" \
    --limit-ledger-size 10000000 \
    > "$VALIDATOR_LOG" 2>&1 &

VALIDATOR_PID_VALUE=$!
echo "$VALIDATOR_PID_VALUE" > "$VALIDATOR_PID"
log_info "Validator PID: $VALIDATOR_PID_VALUE (saved to $VALIDATOR_PID)"

# Wait for validator to be ready
log_info "Waiting for validator to start..."
RETRIES=0
MAX_RETRIES=30

while [ $RETRIES -lt $MAX_RETRIES ]; do
    if solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
        log_info "Validator ready after ${RETRIES}s"
        break
    fi
    sleep 1
    RETRIES=$((RETRIES + 1))
done

if [ $RETRIES -eq $MAX_RETRIES ]; then
    log_error "Validator failed to start after ${MAX_RETRIES}s"
    log_error "Check logs: tail -f $VALIDATOR_LOG"
    tail -50 "$VALIDATOR_LOG"
    exit 1
fi

# ============================================================================
# STEP 2: Configure Solana CLI and Airdrop
# ============================================================================
log_step "STEP 2: Configuring Solana CLI"

log_info "Setting Solana CLI to localnet (http://localhost:8899)..."
solana config set --url http://localhost:8899 >/dev/null

# Get or create default keypair
if [ ! -f "$HOME/.config/solana/id.json" ]; then
    log_info "Creating default deployer keypair..."
    solana-keygen new --no-bip39-passphrase --force >/dev/null 2>&1
fi

DEPLOYER=$(solana address 2>/dev/null)
log_info "Deployer address: $DEPLOYER"

# Airdrop for deployment
log_info "Airdropping 10 SOL to deployer..."
solana airdrop 10 --url http://localhost:8899 >/dev/null 2>&1
sleep 2

BALANCE=$(solana balance --url http://localhost:8899 2>/dev/null)
log_info "Deployer balance: $BALANCE"

# ============================================================================
# STEP 3: Deploy Programs
# ============================================================================
log_step "STEP 3: Deploying Programs"

PROGRAMS=("bedrock" "zk-generator" "fhe-generator")
declare -A PROGRAM_IDS

for prog in "${PROGRAMS[@]}"; do
    PROGRAM_PATH="src/programs/target/deploy/${prog}.so"
    PROGRAM_KEYPAIR="src/programs/target/deploy/${prog}-keypair.json"
    DEPLOY_LOG="/tmp/e2e-deploy-${prog}.log"

    if [ ! -f "$PROGRAM_PATH" ]; then
        log_warn "Program binary not found: $PROGRAM_PATH (skipping)"
        continue
    fi

    # Generate program keypair if doesn't exist
    if [ ! -f "$PROGRAM_KEYPAIR" ]; then
        log_info "Generating keypair for $prog..."
        solana-keygen new --no-bip39-passphrase --force --outfile "$PROGRAM_KEYPAIR" >/dev/null 2>&1
    fi

    log_info "Deploying $prog (this may take 30-60 seconds)..."

    if solana program deploy "$PROGRAM_PATH" \
        --url http://localhost:8899 \
        --keypair "$HOME/.config/solana/id.json" \
        --program-id "$PROGRAM_KEYPAIR" \
        > "$DEPLOY_LOG" 2>&1; then

        PROG_ID=$(solana address --keypair "$PROGRAM_KEYPAIR" 2>/dev/null)
        PROGRAM_IDS[$prog]=$PROG_ID
        log_info "  $prog deployed: $PROG_ID"

        # Verify deployment
        if solana program show "$PROG_ID" --url http://localhost:8899 >/dev/null 2>&1; then
            log_info "  Verified on-chain"
        else
            log_warn "  Program verification failed"
        fi
    else
        log_error "Deployment failed for $prog"
        log_error "Check logs: tail -f $DEPLOY_LOG"
        tail -20 "$DEPLOY_LOG"
        exit 1
    fi

    sleep 1
done

# Save program IDs to env file
ENV_FILE="/tmp/e2e-programs.env"
{
    echo "# E2E Program IDs - Generated $(date)"
    echo "BEDROCK_PROGRAM_ID=${PROGRAM_IDS[bedrock]:-}"
    echo "ZK_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[zk-generator]:-}"
    echo "FHE_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[fhe-generator]:-}"
    echo "SOLANA_RPC_URL=http://localhost:8899"
} > "$ENV_FILE"

log_info "Program IDs saved to $ENV_FILE"

# ============================================================================
# STEP 4: Fund Prover Wallets
# ============================================================================
log_step "STEP 4: Funding Prover Wallets"

log_info "Airdropping 5 SOL to each prover..."

for i in 1 2 3; do
    KEYPAIR="/tmp/prover-$i-keypair.json"

    if [ ! -f "$KEYPAIR" ]; then
        log_warn "Prover $i keypair not found at $KEYPAIR (run setup first)"
        continue
    fi

    ADDR=$(solana address --keypair "$KEYPAIR" 2>/dev/null)
    log_info "  Prover $i: $ADDR"

    if solana airdrop 5 "$ADDR" --url http://localhost:8899 >/dev/null 2>&1; then
        sleep 1
        BALANCE=$(solana balance --keypair "$KEYPAIR" --url http://localhost:8899 2>/dev/null)
        log_info "    Balance: $BALANCE"
    else
        log_warn "    Airdrop failed (may need to retry)"
    fi
done

# ============================================================================
# STEP 5: Start Backend with Podman Compose
# ============================================================================
log_step "STEP 5: Starting Backend Services"

COMPOSE_FILE="infra/docker/docker-compose.localnet.yml"

if [ ! -f "$COMPOSE_FILE" ]; then
    log_error "Compose file not found: $COMPOSE_FILE"
    exit 1
fi

# Check if podman-compose is available
if ! command -v podman-compose >/dev/null 2>&1; then
    log_warn "podman-compose not found, skipping backend startup"
    log_warn "Backend services must be started manually"
else
    log_info "Starting backend with podman-compose..."

    # Set environment variables for compose
    export BEDROCK_PROGRAM_ID="${PROGRAM_IDS[bedrock]:-}"
    export ZK_GENERATOR_PROGRAM_ID="${PROGRAM_IDS[zk-generator]:-}"
    export FHE_GENERATOR_PROGRAM_ID="${PROGRAM_IDS[fhe-generator]:-}"
    export SOLANA_RPC_URL="http://localhost:8899"

    if podman-compose -f "$COMPOSE_FILE" up -d > /tmp/e2e-compose.log 2>&1; then
        log_info "Backend started successfully"

        # Wait for services to be ready
        log_info "Waiting for backend services to initialize..."
        sleep 5

        # Check health
        if curl -f http://localhost:8080/health >/dev/null 2>&1; then
            log_info "  blink-server healthy"
        else
            log_warn "  blink-server not responding (may need more time)"
        fi

        if curl -f http://localhost:8081/health >/dev/null 2>&1; then
            log_info "  x402-server healthy"
        else
            log_warn "  x402-server not responding (may need more time)"
        fi
    else
        log_error "Failed to start backend services"
        log_error "Check logs: tail -f /tmp/e2e-compose.log"
        tail -20 /tmp/e2e-compose.log
        exit 1
    fi
fi

# ============================================================================
# Summary
# ============================================================================
log_step "E2E Stack Running!"

echo ""
echo "Services:"
echo "  Validator:     http://localhost:8899"
echo "  Backend:       http://localhost:8080"
echo "  Gateway:       http://localhost:8081"
echo ""
echo "Deployed Programs:"
for prog in "${!PROGRAM_IDS[@]}"; do
    echo "  $prog: ${PROGRAM_IDS[$prog]}"
done
echo ""
echo "Environment:"
echo "  Program IDs:   $ENV_FILE"
echo ""
echo "Logs:"
echo "  Validator:     tail -f $VALIDATOR_LOG"
echo "  Deploy:        tail -f /tmp/e2e-deploy-*.log"
echo "  Compose:       tail -f /tmp/e2e-compose.log"
echo ""
echo "PIDs:"
echo "  Validator:     $VALIDATOR_PID (PID: $VALIDATOR_PID_VALUE)"
echo ""
echo "Next Steps:"
echo "  1. Run tests:   ./scripts/e2e/run-e2e-tests.sh"
echo "  2. Stop stack:  ./scripts/e2e/stop-stack.sh"
echo ""
echo "Or use Makefile:"
echo "  make e2e-test"
echo "  make e2e-stop"
echo ""
