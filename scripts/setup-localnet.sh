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
if [ ! -f "Cargo.toml" ] || [ ! -d "src/programs" ]; then
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
    podman-compose up -d
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
# STEP 2: Program Deployment
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

# 2.4 Deploy program
log_info "Deploying Solana program..."
PROGRAM_PATH=src/programs/target/deploy/zyberlink.so

if [ ! -f "$PROGRAM_PATH" ]; then
    log_error "Program binary not found at $PROGRAM_PATH"
    log_error "Run: cd src/programs && cargo build-sbf"
    exit 1
fi

# Deploy with program keypair
PROGRAM_KEYPAIR=src/programs/target/deploy/zyberlink-keypair.json

if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    log_error "Program keypair not found at $PROGRAM_KEYPAIR"
    exit 1
fi

log_info "Deploying program (this may take 30-60 seconds)..."
if solana program deploy $PROGRAM_PATH \
    --url http://localhost:8899 \
    --keypair ~/.config/solana/id.json \
    --program-id $PROGRAM_KEYPAIR \
    > /tmp/program-deploy.log 2>&1; then

    PROGRAM_ID=$(solana address --keypair $PROGRAM_KEYPAIR)
    log_info "✓ Program deployed successfully"
    log_info "Program ID: $PROGRAM_ID"
else
    log_error "Program deployment failed"
    tail -20 /tmp/program-deploy.log
    exit 1
fi

# Verify deployment
log_info "Verifying deployment..."
if solana program show $PROGRAM_ID --url http://localhost:8899 >/dev/null 2>&1; then
    log_info "✓ Program verified on-chain"
else
    log_error "Program verification failed"
    exit 1
fi

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

# Backend .env
cat > src/blink-server/.env <<EOF
DATABASE_URL=postgresql://zyberlink:dev_password@localhost:5432/zyberlink
SOLANA_RPC_URL=http://localhost:8899
PROGRAM_ID=$PROGRAM_ID
PORT=8080
HOST=127.0.0.1
RUST_LOG=info
EOF

log_info "✓ Created src/blink-server/.env"

# Frontend .env
cat > src/webapp/.env.local <<EOF
VITE_SOLANA_RPC_URL=http://localhost:8899
VITE_BACKEND_URL=http://127.0.0.1:8080
VITE_PROGRAM_ID=$PROGRAM_ID
EOF

log_info "✓ Created src/webapp/.env.local"

# ============================================================================
# Summary
# ============================================================================
log_step "Setup Complete!"

echo " Infrastructure Ready:"
echo "  • Validator:    http://localhost:8899"
echo "  • Database:     postgresql://localhost:5432/zyberlink"
echo "  • Program ID:   $PROGRAM_ID"
echo ""
echo " Provers:"
for i in 1 2 3; do
    echo "  • Prover $i:    ${PROVER_ADDRESSES[$i-1]}"
done
echo ""
echo " Configuration:"
echo "  • Backend env:  src/blink-server/.env"
echo "  • Frontend env: src/webapp/.env.local"
echo ""
echo " Logs:"
echo "  • Validator:    tail -f /tmp/solana-validator.log"
echo ""
echo " Next Steps:"
echo "  1. Start backend:  ./target/release/blink-server"
echo "  2. Start frontend: cd src/webapp && npm run dev"
echo "  3. Start provers:  Run scripts/localnet-start-provers.sh"
echo "  4. Run tests:      Run scripts/localnet-test-api.sh"
echo ""
echo " To save this session:"
echo "  export ZYBERLINK_PROGRAM_ID=$PROGRAM_ID"
echo ""
