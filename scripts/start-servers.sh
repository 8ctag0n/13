#!/bin/bash
#
# Start ZyberLink Server Stack
# Uses podman-compose to start full E2E stack
#
# Architecture:
#   nginx:80 -> public-api:3000 -> x402:8081 -> blink:8080
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

cd "$PROJECT_ROOT"

log_step "Starting ZyberLink Server Stack"

# ============================================================================
# Check Prerequisites
# ============================================================================
log_info "Checking prerequisites..."

# Check if solana validator is running
if ! solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
    log_error "Solana validator not running at localhost:8899"
    log_error "Run: ./scripts/setup-localnet.sh"
    exit 1
fi

log_info "✓ Solana validator is running"

# Check if .env files exist (created by setup-localnet.sh)
if [ ! -f "src/blink-server/.env" ]; then
    log_error "Environment files not found. Run ./scripts/setup-localnet.sh first"
    exit 1
fi

log_info "✓ Environment files found"

# Load environment variables from blink-server/.env
set -a
source src/blink-server/.env
set +a

# Check if BEDROCK_PROGRAM_ID is set
if [ -z "$BEDROCK_PROGRAM_ID" ]; then
    log_warn "BEDROCK_PROGRAM_ID not set, using legacy PROGRAM_ID"
fi

log_info "Program IDs:"
[ -n "$BEDROCK_PROGRAM_ID" ] && log_info "  • bedrock: $BEDROCK_PROGRAM_ID"
[ -n "$ZK_GENERATOR_PROGRAM_ID" ] && log_info "  • zk-generator: $ZK_GENERATOR_PROGRAM_ID"
[ -n "$FHE_GENERATOR_PROGRAM_ID" ] && log_info "  • fhe-generator: $FHE_GENERATOR_PROGRAM_ID"
[ -n "$PROGRAM_ID" ] && log_info "  • legacy: $PROGRAM_ID"

# ============================================================================
# Start Stack with Podman Compose
# ============================================================================
log_step "Starting Stack (podman-compose)"

COMPOSE_FILE="infra/docker/docker-compose.localnet.yml"

if [ ! -f "$COMPOSE_FILE" ]; then
    log_error "Compose file not found: $COMPOSE_FILE"
    exit 1
fi

log_info "Using compose file: $COMPOSE_FILE"

# Export environment variables for docker-compose
export BEDROCK_PROGRAM_ID
export ZK_GENERATOR_PROGRAM_ID
export FHE_GENERATOR_PROGRAM_ID
export THRESHOLD_PROGRAM_ID
export PROGRAM_ID
export RUST_LOG=${RUST_LOG:-info}

# Start services
log_info "Starting containers (this may take a few minutes on first run)..."
podman-compose -f "$COMPOSE_FILE" up -d --build 2>&1 | tee /tmp/podman-compose.log

# Wait for services to be healthy
log_step "Waiting for Services to be Healthy"

# Function to wait for service
wait_for_service() {
    local SERVICE_NAME=$1
    local URL=$2
    local MAX_WAIT=60

    log_info "Waiting for $SERVICE_NAME..."
    for i in $(seq 1 $MAX_WAIT); do
        if curl -s "$URL" > /dev/null 2>&1; then
            log_info "✓ $SERVICE_NAME ready after ${i}s"
            return 0
        fi
        sleep 1
    done

    log_warn "✗ $SERVICE_NAME not ready after ${MAX_WAIT}s"
    return 1
}

# Wait for each layer
wait_for_service "blink-server" "http://localhost:8080/health"
wait_for_service "x402-server" "http://localhost:8081/health"
wait_for_service "public-api" "http://localhost:3000/health"
wait_for_service "nginx" "http://localhost:80/health"

# ============================================================================
# Health Checks
# ============================================================================
log_step "Health Checks"

# Check each service
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    log_info "✓ blink-server (Layer 1: Backend): OK"
else
    log_error "✗ blink-server: FAILED"
fi

if curl -s http://localhost:8081/health > /dev/null 2>&1; then
    log_info "✓ x402-server (Layer 2: Gateway): OK"
else
    log_error "✗ x402-server: FAILED"
fi

if curl -s http://localhost:3000/health > /dev/null 2>&1; then
    log_info "✓ public-api (Layer 3: Auth): OK"
else
    log_error "✗ public-api: FAILED"
fi

if curl -s http://localhost:80/health > /dev/null 2>&1; then
    log_info "✓ nginx (Reverse Proxy): OK"
else
    log_error "✗ nginx: FAILED"
fi

# ============================================================================
# Summary
# ============================================================================
log_step "ZyberLink Stack Running"

echo " Architecture:"
echo "  Client -> nginx:80 -> public-api:3000 -> x402:8081 -> blink:8080"
echo ""
echo " Public Endpoint:"
echo "  http://localhost:80"
echo ""
echo " API Examples:"
echo "  curl http://localhost:80/health"
echo "  curl http://localhost:80/api/provers"
echo ""
echo " Direct Service URLs (for debugging):"
echo "  • nginx:        http://localhost:80"
echo "  • public-api:   http://localhost:3000"
echo "  • x402-server:  http://localhost:8081"
echo "  • blink-server: http://localhost:8080"
echo "  • postgres:     postgresql://localhost:5432/zyberlink"
echo ""
echo " View Logs:"
echo "  podman-compose -f $COMPOSE_FILE logs -f"
echo "  podman-compose -f $COMPOSE_FILE logs -f blink-server"
echo "  podman-compose -f $COMPOSE_FILE logs -f x402-server"
echo "  podman-compose -f $COMPOSE_FILE logs -f public-api"
echo "  podman-compose -f $COMPOSE_FILE logs -f nginx"
echo ""
echo " Container Status:"
podman-compose -f "$COMPOSE_FILE" ps
echo ""
echo " To stop all servers:"
echo "  ./scripts/stop-servers.sh"
echo ""
