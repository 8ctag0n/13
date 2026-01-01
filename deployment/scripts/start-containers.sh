#!/bin/bash
#
# ZyberLink Container Stack - Start Script
# Production-like setup with containerized services
#
# Validator: host (not containerized)
# Postgres: container
# Backend: container (with replicas)
# Webapp: container
# Nginx: container (load balancer)
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
cd "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo "  ZyberLink Container Stack"
echo "==========================================="
echo ""

# ============================================================================
# STEP 1: Check if already running
# ============================================================================
if curl -s http://localhost:8899/health 2>/dev/null | grep -q "ok" && \
   curl -s http://localhost:9000/health 2>/dev/null; then
    log_ok "System already running!"
    echo ""
    echo "Services:"
    echo "  - Validator: http://localhost:8899"
    echo "  - Nginx LB:  http://localhost:9000"
    echo "  - API:       http://localhost:9000/api/"
    exit 0
fi

# ============================================================================
# STEP 2: Build programs and backend
# ============================================================================
log_step "Building Solana programs..."

# Programs location after refactor
PROGRAMS_DIR="chain/solana"

# List of programs in dependency order
PROGRAMS=("bedrock" "fhe_generator" "zk_generator" "threshold" "futarchy_markets")

# Check if any program needs building
NEED_BUILD=false
for prog in "${PROGRAMS[@]}"; do
    if [ ! -f "${PROGRAMS_DIR}/target/deploy/${prog}.so" ]; then
        NEED_BUILD=true
        break
    fi
done

if [ "$NEED_BUILD" = true ]; then
    cd "${PROGRAMS_DIR}" && cargo build-sbf 2>&1 | tail -10
    cd "$PROJECT_ROOT"
fi

for prog in "${PROGRAMS[@]}"; do
    if [ -f "${PROGRAMS_DIR}/target/deploy/${prog}.so" ]; then
        log_ok "$prog ready"
    else
        log_warn "$prog not found (may not be critical)"
    fi
done

log_step "Building backend (blink-server)..."
if [ ! -f "target/release/blink-server" ]; then
    cargo build --release --bin blink-server 2>&1 | tail -5
fi
log_ok "Backend binary ready"

# ============================================================================
# STEP 3: Start ALL containers
# ============================================================================
log_step "Starting all containers..."

# Stop old containers and volumes (fresh start each time)
podman-compose down -v 2>/dev/null || true

# Create .env.containers with all program IDs
BEDROCK_ID=$(solana address --keypair ${PROGRAMS_DIR}/target/deploy/bedrock-keypair.json)
FHE_GENERATOR_ID=$(solana address --keypair ${PROGRAMS_DIR}/target/deploy/fhe_generator-keypair.json)
ZK_GENERATOR_ID=$(solana address --keypair ${PROGRAMS_DIR}/target/deploy/zk_generator-keypair.json)
THRESHOLD_ID=$(solana address --keypair ${PROGRAMS_DIR}/target/deploy/threshold-keypair.json)
FUTARCHY_ID=$(solana address --keypair ${PROGRAMS_DIR}/target/deploy/futarchy_markets-keypair.json 2>/dev/null || echo "NOT_DEPLOYED")

cat > .env.containers << EOF
DB_PASSWORD=dev_password
BEDROCK_PROGRAM_ID=$BEDROCK_ID
FHE_GENERATOR_PROGRAM_ID=$FHE_GENERATOR_ID
ZK_GENERATOR_PROGRAM_ID=$ZK_GENERATOR_ID
THRESHOLD_PROGRAM_ID=$THRESHOLD_ID
FUTARCHY_PROGRAM_ID=$FUTARCHY_ID
PROGRAM_ID=$BEDROCK_ID
EOF

# Start all containers at once
podman-compose --env-file .env.containers up -d

echo "  Waiting for validator container..."
for i in {1..60}; do
    if curl -s http://localhost:8899/health 2>/dev/null | grep -q "ok"; then
        log_ok "Validator ready (${i}s)"
        break
    fi
    sleep 1
done

if ! curl -s http://localhost:8899/health 2>/dev/null | grep -q "ok"; then
    log_error "Validator failed to start. Check: podman logs demo-zyberlink-zcash_validator_1"
fi

solana config set --url http://localhost:8899 > /dev/null

# Check postgres
sleep 3
if podman exec demo-zyberlink-zcash_postgres_1 pg_isready -U zyberlink >/dev/null 2>&1; then
    log_ok "Postgres ready"
else
    log_warn "Postgres still starting..."
    sleep 5
fi

# ============================================================================
# STEP 4: Deploy all programs (in dependency order)
# ============================================================================
log_step "Deploying programs..."

solana airdrop 100 --url http://localhost:8899 >/dev/null 2>&1 || true

# Deploy programs in dependency order
declare -A PROGRAM_IDS
PROGRAM_IDS["bedrock"]=$BEDROCK_ID
PROGRAM_IDS["fhe_generator"]=$FHE_GENERATOR_ID
PROGRAM_IDS["zk_generator"]=$ZK_GENERATOR_ID
PROGRAM_IDS["threshold"]=$THRESHOLD_ID
PROGRAM_IDS["futarchy_markets"]=$FUTARCHY_ID

for prog in bedrock fhe_generator zk_generator threshold futarchy_markets; do
    KEYPAIR="${PROGRAMS_DIR}/target/deploy/${prog}-keypair.json"
    SO_FILE="${PROGRAMS_DIR}/target/deploy/${prog}.so"

    if [ ! -f "$SO_FILE" ]; then
        log_warn "$prog.so not found, skipping"
        continue
    fi

    if solana program deploy "$SO_FILE" \
        --url http://localhost:8899 \
        --program-id "$KEYPAIR" \
        > /tmp/deploy-${prog}.log 2>&1; then
        log_ok "$prog deployed: ${PROGRAM_IDS[$prog]}"
    else
        log_warn "$prog deploy failed (check /tmp/deploy-${prog}.log)"
    fi
done

# ============================================================================
# STEP 5: Create backend keypair and fund it
# ============================================================================
log_step "Creating backend finalizer keypair..."

if [ ! -f "/tmp/backend-keypair.json" ]; then
    solana-keygen new --no-bip39-passphrase --force --outfile /tmp/backend-keypair.json >/dev/null 2>&1
fi

BACKEND_ADDR=$(solana address --keypair /tmp/backend-keypair.json)
solana airdrop 10 "$BACKEND_ADDR" --url http://localhost:8899 >/dev/null 2>&1 || true
log_ok "Backend finalizer: $BACKEND_ADDR"

# Wait for nginx to be ready
log_step "Waiting for services..."
sleep 5

if curl -s http://localhost:9000/health 2>/dev/null; then
    log_ok "All containers started"
else
    log_warn "Nginx not responding yet, waiting..."
    sleep 5
fi

# ============================================================================
# STEP 8: Create prover wallets
# ============================================================================
log_step "Creating prover wallets..."

for i in 1 2 3; do
    KEYPAIR="/tmp/prover-$i-keypair.json"
    if [ ! -f "$KEYPAIR" ]; then
        solana-keygen new --no-bip39-passphrase --force --outfile "$KEYPAIR" >/dev/null 2>&1
    fi
    ADDR=$(solana address --keypair "$KEYPAIR")
    solana airdrop 10 "$ADDR" --url http://localhost:8899 >/dev/null 2>&1 || true
    log_ok "Prover $i: $ADDR"
done

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "==========================================="
echo "  Container Stack Ready!"
echo "==========================================="
echo ""
echo "Services:"
echo "  - Validator:  http://localhost:8899 (host)"
echo "  - Nginx LB:   http://localhost:9000"
echo "  - Frontend:   http://localhost:9000"
echo "  - API:        http://localhost:9000/api/"
echo ""
echo "Program IDs:"
echo "  - Bedrock:       $BEDROCK_ID"
echo "  - FHE Generator: $FHE_GENERATOR_ID"
echo "  - ZK Generator:  $ZK_GENERATOR_ID"
echo "  - Threshold:     $THRESHOLD_ID"
echo "  - Futarchy:      $FUTARCHY_ID"
echo ""
echo "Next steps:"
echo "  make c2   # Init marketplace + register provers"
echo "  make c3   # Start provers + job creator"
echo ""
echo "Logs:"
echo "  - Validator:  tail -f /tmp/solana-validator.log"
echo "  - Containers: podman-compose logs -f"
echo ""
