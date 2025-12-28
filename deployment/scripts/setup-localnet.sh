#!/bin/bash
#
# ZyberLink Localnet Setup Script
# Semi-automated E2E testing infrastructure
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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
    echo -e "\n${GREEN}===${NC} $1 ${GREEN}===${NC}\n"
}

# Cleanup function
cleanup() {
    log_warn "Cleaning up previous processes..."
    pkill -f solana-test-validator || true
    pkill -f blink-server || true
    pkill -f prover-node || true
    sleep 2
}

# Check if running from project root
if [ ! -f "Cargo.toml" ] || [ ! -d "programs" ]; then
    log_error "Must run from project root (/home/deploy/experimental/zyberlink-demo)"
    exit 1
fi

log_step "ZyberLink Localnet E2E Setup"

# Parse command line args
SKIP_CLEANUP=${1:-false}

if [ "$SKIP_CLEANUP" != "skip-cleanup" ]; then
    cleanup
fi

# ============================================================================
# STEP 1: Infrastructure Bootstrap
# ============================================================================
log_step "STEP 1: Infrastructure Bootstrap"

# 1.1 Check database
log_info "Checking PostgreSQL database..."
if ! podman ps | grep -q postgres; then
    log_warn "PostgreSQL not running, starting..."
    podman-compose -f infra/docker/docker-compose.yml up -d
    sleep 5
fi

# Test database connection
if podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c "SELECT 1" >/dev/null 2>&1; then
    log_info "✓ Database connection OK"
else
    log_error "Database not accessible"
    exit 1
fi

# 1.2 Clean old validator data
LEDGER_DIR="$HOME/.zyberlink-localnet-ledger"
log_info "Cleaning old validator ledger..."
rm -rf "$LEDGER_DIR" 2>/dev/null || true

# 1.3 Start Solana validator
log_info "Starting Solana test validator..."
solana-test-validator \
    --reset \
    --rpc-port 8899 \
    --ledger "$LEDGER_DIR" \
    --limit-ledger-size 10000000 \
    > /tmp/solana-validator.log 2>&1 &

VALIDATOR_PID=$!
log_info "Validator PID: $VALIDATOR_PID"

# Wait for validator to be ready
log_info "Waiting for validator to start..."
for i in {1..30}; do
    if solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
        log_info "✓ Validator ready after ${i}s"
        break
    fi
    sleep 1
done

if ! solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
    log_error "Validator failed to start"
    tail -20 /tmp/solana-validator.log
    exit 1
fi

# ============================================================================
# STEP 2: Program Deployment (Multi-Program Architecture)
# ============================================================================
log_step "STEP 2: Program Deployment"

# 2.1 Configure CLI for localnet
log_info "Configuring Solana CLI for localnet..."
solana config set --url http://localhost:8899 >/dev/null

# 2.2 Create deployer keypair if needed
if [ ! -f ~/.config/solana/id.json ]; then
    log_info "Creating deployer keypair..."
    solana-keygen new --no-bip39-passphrase --force
fi

DEPLOYER=$(solana address)
log_info "Deployer address: $DEPLOYER"

# 2.3 Airdrop for deployment
log_info "Airdropping SOL for deployment..."
solana airdrop 10 --url http://localhost:8899 >/dev/null
sleep 2

BALANCE=$(solana balance --url http://localhost:8899)
log_info "Deployer balance: $BALANCE"

# 2.4 Generate keypairs for all programs
log_info "Generating program keypairs..."
PROGRAMS=("bedrock" "zk-generator" "fhe-generator" "threshold" "zyberlink")

for prog in "${PROGRAMS[@]}"; do
    KEYPAIR="programs/target/deploy/${prog}-keypair.json"
    if [ ! -f "$KEYPAIR" ]; then
        log_info "  Creating keypair for $prog..."
        solana-keygen new --no-bip39-passphrase --force --outfile "$KEYPAIR" >/dev/null 2>&1
    else
        log_info "  Keypair exists for $prog"
    fi
done

# 2.5 Deploy programs in order
declare -A PROGRAM_IDS

# Core programs (deployed first)
CORE_PROGRAMS=("bedrock" "zk-generator" "fhe-generator")

for prog in "${CORE_PROGRAMS[@]}"; do
    PROGRAM_PATH="programs/target/deploy/${prog}.so"
    PROGRAM_KEYPAIR="programs/target/deploy/${prog}-keypair.json"

    if [ ! -f "$PROGRAM_PATH" ]; then
        log_warn "Program binary not found: $PROGRAM_PATH (skipping)"
        continue
    fi

    log_info "Deploying $prog (this may take 30-60 seconds)..."
    if solana program deploy $PROGRAM_PATH \
        --url http://localhost:8899 \
        --keypair ~/.config/solana/id.json \
        --program-id $PROGRAM_KEYPAIR \
        > /tmp/program-deploy-${prog}.log 2>&1; then

        PROG_ID=$(solana address --keypair $PROGRAM_KEYPAIR)
        PROGRAM_IDS[$prog]=$PROG_ID
        log_info "✓ $prog deployed: $PROG_ID"

        # Verify deployment
        if solana program show $PROG_ID --url http://localhost:8899 >/dev/null 2>&1; then
            log_info "  ✓ Verified on-chain"
        else
            log_error "  Program verification failed"
            exit 1
        fi
    else
        log_error "Deployment failed for $prog"
        tail -20 /tmp/program-deploy-${prog}.log
        exit 1
    fi

    sleep 1
done

# Legacy program (for compatibility)
if [ -f "programs/target/deploy/zyberlink.so" ]; then
    log_info "Deploying legacy zyberlink program..."
    PROGRAM_KEYPAIR="programs/target/deploy/zyberlink-keypair.json"

    if solana program deploy programs/target/deploy/zyberlink.so \
        --url http://localhost:8899 \
        --keypair ~/.config/solana/id.json \
        --program-id $PROGRAM_KEYPAIR \
        > /tmp/program-deploy-zyberlink.log 2>&1; then

        LEGACY_ID=$(solana address --keypair $PROGRAM_KEYPAIR)
        PROGRAM_IDS[zyberlink]=$LEGACY_ID
        log_info "✓ Legacy zyberlink deployed: $LEGACY_ID"
    fi
fi

# Use bedrock as main PROGRAM_ID
PROGRAM_ID=${PROGRAM_IDS[bedrock]:-${PROGRAM_IDS[zyberlink]}}

if [ -z "$PROGRAM_ID" ]; then
    log_error "No programs were deployed successfully"
    exit 1
fi

log_info "\nDeployed Program IDs:"
for prog in "${!PROGRAM_IDS[@]}"; do
    log_info "  $prog: ${PROGRAM_IDS[$prog]}"
done

# ============================================================================
# STEP 3: Create Prover Wallets
# ============================================================================
log_step "STEP 3: Multi-Prover Setup"

log_info "Creating 3 prover wallets..."
PROVER_ADDRESSES=()

for i in 1 2 3; do
    PROVER_KEYPAIR="/tmp/prover-$i-keypair.json"

    # Generate keypair
    solana-keygen new \
        --no-bip39-passphrase \
        --force \
        --outfile $PROVER_KEYPAIR \
        >/dev/null 2>&1

    PROVER_ADDR=$(solana address --keypair $PROVER_KEYPAIR)
    PROVER_ADDRESSES+=("$PROVER_ADDR")

    log_info "Prover $i: $PROVER_ADDR"

    # Airdrop SOL
    log_info "  Airdropping 5 SOL..."
    solana airdrop 5 $PROVER_ADDR --url http://localhost:8899 >/dev/null
    sleep 1
done

# Verify balances
log_info "\nVerifying prover balances:"
for i in 1 2 3; do
    BALANCE=$(solana balance --keypair /tmp/prover-$i-keypair.json)
    log_info "  Prover $i: $BALANCE"
done

# ============================================================================
# STEP 4: Environment Configuration
# ============================================================================
log_step "STEP 4: Generate Environment Files"

# blink-server .env (main backend)
cat > services/blink-server/.env <<EOF
DATABASE_URL=postgresql://zyberlink:dev_password@localhost:5432/zyberlink
SOLANA_RPC_URL=http://localhost:8899

# Bedrock Architecture Program IDs
BEDROCK_PROGRAM_ID=${PROGRAM_IDS[bedrock]:-}
ZK_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[zk-generator]:-}
FHE_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[fhe-generator]:-}
THRESHOLD_PROGRAM_ID=${PROGRAM_IDS[threshold]:-}

# Legacy compatibility
PROGRAM_ID=$PROGRAM_ID

# Server config
PORT=8080
HOST=127.0.0.1
RUST_LOG=info
EOF

log_info "✓ Created services/blink-server/.env"

# x402-server .env (gateway)
cat > services/x402-server/.env <<EOF
DATABASE_URL=postgresql://zyberlink:dev_password@localhost:5432/zyberlink
SOLANA_RPC_URL=http://localhost:8899

# Bedrock Architecture Program IDs
BEDROCK_PROGRAM_ID=${PROGRAM_IDS[bedrock]:-}
ZK_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[zk-generator]:-}
FHE_GENERATOR_PROGRAM_ID=${PROGRAM_IDS[fhe-generator]:-}

# Gateway config
BLINK_SERVER_URL=http://localhost:8080
PORT=8081
HOST=127.0.0.1
RUST_LOG=info
EOF

log_info "✓ Created services/x402-server/.env"

# Frontend .env
cat > apps/webapp/.env.local <<EOF
VITE_SOLANA_RPC_URL=http://localhost:8899
VITE_BACKEND_URL=http://127.0.0.1:8080
VITE_GATEWAY_URL=http://127.0.0.1:8081
VITE_BEDROCK_PROGRAM_ID=${PROGRAM_IDS[bedrock]:-$PROGRAM_ID}
VITE_PROGRAM_ID=$PROGRAM_ID
EOF

log_info "✓ Created apps/webapp/.env.local"

# ============================================================================
# Summary
# ============================================================================
log_step "Setup Complete!"

echo " Infrastructure Ready:"
echo "  • Validator:    http://localhost:8899"
echo "  • Database:     postgresql://localhost:5432/zyberlink"
echo ""
echo " Bedrock Architecture Programs:"
for prog in "${!PROGRAM_IDS[@]}"; do
    echo "  • $prog: ${PROGRAM_IDS[$prog]}"
done
echo ""
echo " Provers:"
for i in 1 2 3; do
    echo "  • Prover $i:    ${PROVER_ADDRESSES[$i-1]}"
done
echo ""
echo " Configuration:"
echo "  • blink-server:   services/blink-server/.env"
echo "  • x402-server:    services/x402-server/.env"
echo "  • Frontend:       apps/webapp/.env.local"
echo ""
echo " Logs:"
echo "  • Validator:      tail -f /tmp/solana-validator.log"
echo "  • Program Deploy: tail -f /tmp/program-deploy-*.log"
echo ""
echo " Next Steps:"
echo "  1. Start servers:  ./scripts/start-servers.sh"
echo "  2. Start frontend: cd src/webapp && npm run dev"
echo "  3. Start provers:  ./scripts/localnet-start-provers.sh"
echo "  4. Run tests:      ./scripts/test-e2e.sh"
echo ""
echo " Quick Commands:"
echo "  • Start servers:   ./scripts/start-servers.sh"
echo "  • Stop servers:    ./scripts/stop-servers.sh"
echo "  • Check status:    ./scripts/localnet-status.sh"
echo ""
echo " Environment Variables:"
echo "  export BEDROCK_PROGRAM_ID=${PROGRAM_IDS[bedrock]:-$PROGRAM_ID}"
echo "  export ZYBERLINK_PROGRAM_ID=$PROGRAM_ID"
echo ""
