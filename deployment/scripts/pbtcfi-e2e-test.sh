#!/bin/bash
# pBTCFi E2E Test Automation Script
# Automates infrastructure setup and validation for pBTCFi testing
#
# Usage:
#   ./scripts/pbtcfi-e2e-test.sh [phase]
#
# Phases:
#   setup    - Start infrastructure (devnet + postgres + backend)
#   verify   - Verify all services are healthy
#   deploy   - Deploy Cairo contracts (interactive)
#   cleanup  - Stop all services and clean up
#   full     - Run complete E2E flow (setup → verify → wait for manual deploy)

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Config
COMPOSE_FILE="docker-compose.pbtcfi.yml"
ENV_FILE=".env.pbtcfi"
DEVNET_RPC="http://localhost:5050"
PUBLIC_API="http://localhost:3001"
POSTGRES_PORT="5433"

# Logging
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Phase 1: Setup Infrastructure
phase_setup() {
    log_info "===== PHASE 1: SETUP INFRASTRUCTURE ====="

    # Check podman-compose available
    if ! command -v podman-compose &> /dev/null; then
        log_error "podman-compose not found. Install it first."
        exit 1
    fi

    # Check docker-compose.pbtcfi.yml exists
    if [ ! -f "$COMPOSE_FILE" ]; then
        log_error "$COMPOSE_FILE not found in project root"
        exit 1
    fi

    log_info "Starting pBTCFi stack (devnet + postgres + backend)..."
    podman-compose -f "$COMPOSE_FILE" up -d

    log_info "Waiting 15 seconds for services to initialize..."
    sleep 15

    log_success "Infrastructure started!"
    echo ""
    log_info "Services running:"
    podman ps --filter "name=pbtcfi-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
}

# Phase 2: Verify Services
phase_verify() {
    log_info "===== PHASE 2: VERIFY SERVICES ====="

    local all_ok=true

    # Test 1: Devnet RPC
    log_info "Testing Starknet devnet RPC ($DEVNET_RPC)..."
    if curl -s -X POST "$DEVNET_RPC" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}' | jq -e '.result' > /dev/null 2>&1; then
        log_success "Devnet RPC responding"
    else
        log_error "Devnet RPC not responding"
        all_ok=false
    fi

    # Test 2: Postgres
    log_info "Testing PostgreSQL (port $POSTGRES_PORT)..."
    if podman exec pbtcfi-postgres psql -U pbtcfi -d pbtcfi -c "SELECT 1;" > /dev/null 2>&1; then
        log_success "PostgreSQL responding"
    else
        log_error "PostgreSQL not responding"
        all_ok=false
    fi

    # Test 3: Public API
    log_info "Testing Public API ($PUBLIC_API/health)..."
    if curl -s "$PUBLIC_API/health" | jq -e '.status' > /dev/null 2>&1; then
        log_success "Public API responding"
    else
        log_warn "Public API not responding (may still be starting)"
    fi

    echo ""
    if [ "$all_ok" = true ]; then
        log_success "All critical services verified!"
    else
        log_error "Some services failed verification. Check logs:"
        echo "  podman-compose -f $COMPOSE_FILE logs"
        exit 1
    fi
}

# Phase 3: Deploy Contracts (interactive)
phase_deploy() {
    log_info "===== PHASE 3: DEPLOY CAIRO CONTRACTS ====="

    log_warn "Contract deployment requires manual steps."
    log_info "Options:"
    echo ""
    echo "  A) Use cairo-dev container:"
    echo "     podman exec -it pbtcfi-cairo-dev bash"
    echo "     cd /workspace/verticals/pbtcfi/contracts/cairo"
    echo "     scarb build"
    echo "     # Then use sncast to deploy"
    echo ""
    echo "  B) Use host (if Scarb/Starkli installed):"
    echo "     cd verticals/pbtcfi/contracts/cairo"
    echo "     scarb build"
    echo "     # Deploy with sncast/starkli"
    echo ""
    echo "  C) Use pre-deployed contract address (for testing)"
    echo ""

    read -p "Have you deployed the contract? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Exiting. Deploy contract and re-run."
        exit 0
    fi

    read -p "Enter contract address (0x...): " CONTRACT_ADDRESS

    # Validate address format
    if [[ ! $CONTRACT_ADDRESS =~ ^0x[0-9a-fA-F]{64}$ ]]; then
        log_warn "Address format looks suspicious. Continue anyway? (y/n)"
        read -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    # Save to .env file
    log_info "Saving configuration to $ENV_FILE..."

    # Use Katana pre-funded account for testing
    cat > "$ENV_FILE" <<EOF
# pBTCFi E2E Test Environment
# Generated: $(date)

STARKNET_RPC_URL=$DEVNET_RPC
PBTCFI_CONTRACT_ADDRESS=$CONTRACT_ADDRESS

# Katana devnet pre-funded account #0
STARKNET_PROVER_ADDRESS=0x0517ececd29116499f4a1b64b094da79ba08dfd54a3edaa316134c41f8160973
STARKNET_PRIVATE_KEY=0x00c1cf1490de1352865301bb8705143f3ef938f97fdf892f1090dcb5ac7bcd1d
EOF

    log_success "Configuration saved!"
    log_info "Environment variables:"
    cat "$ENV_FILE"
}

# Phase 4: Create Prover Config
phase_config_prover() {
    log_info "===== PHASE 4: CONFIGURE PROVER ====="

    if [ ! -f "$ENV_FILE" ]; then
        log_error "$ENV_FILE not found. Run 'deploy' phase first."
        exit 1
    fi

    # Load env vars
    source "$ENV_FILE"

    local PROVER_CONFIG="prover.pbtcfi.toml"

    log_info "Creating prover config: $PROVER_CONFIG..."

    cat > "$PROVER_CONFIG" <<EOF
# pBTCFi Prover Configuration (E2E Testing)
# Generated: $(date)

[prover]
max_concurrent_jobs = 1
poll_interval_secs = 5
min_roi_threshold = 0.0
cost_multiplier = 1.0
mock_proving_time_secs = 3

[witness]
blink_backend_url = "$PUBLIC_API"

[chains.starknet]
enabled = true
rpc_url = "$STARKNET_RPC_URL"
contract_address = "$PBTCFI_CONTRACT_ADDRESS"
prover_address = "$STARKNET_PROVER_ADDRESS"
# Private key loaded from STARKNET_PRIVATE_KEY env var

[chains.solana]
enabled = false

[chains.aptos]
enabled = false
EOF

    log_success "Prover config created: $PROVER_CONFIG"

    # Check if FHE keys exist
    if [ -f "$HOME/.zyberlink/fhe_server_key.bin" ]; then
        log_success "FHE keys found at ~/.zyberlink/"
    else
        log_warn "FHE keys not found. Generate them:"
        echo "  cargo run --release --bin generate-fhe-keys -- --output-dir ~/.zyberlink"
        echo ""
        log_info "This takes ~30 seconds. Generate now? (y/n)"
        read -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            log_info "Generating FHE keys..."
            cargo run --release --bin generate-fhe-keys -- --output-dir "$HOME/.zyberlink"
            log_success "FHE keys generated!"
        fi
    fi
}

# Phase 5: Run Prover (foreground)
phase_run_prover() {
    log_info "===== PHASE 5: RUN PROVER ====="

    if [ ! -f "$ENV_FILE" ]; then
        log_error "$ENV_FILE not found. Run previous phases first."
        exit 1
    fi

    source "$ENV_FILE"

    local PROVER_CONFIG="prover.pbtcfi.toml"

    if [ ! -f "$PROVER_CONFIG" ]; then
        log_error "$PROVER_CONFIG not found. Run 'config-prover' phase first."
        exit 1
    fi

    log_info "Starting pbtcfi-prover (press Ctrl+C to stop)..."
    log_info "Config: $PROVER_CONFIG"
    log_info "Contract: $PBTCFI_CONTRACT_ADDRESS"
    echo ""

    RUST_LOG=debug \
    STARKNET_PRIVATE_KEY="$STARKNET_PRIVATE_KEY" \
    ./target/release/pbtcfi-prover \
        --config "$PROVER_CONFIG" \
        run
}

# Phase 6: Cleanup
phase_cleanup() {
    log_info "===== CLEANUP ====="

    log_info "Stopping pBTCFi stack..."
    podman-compose -f "$COMPOSE_FILE" down

    log_info "Removing temporary files..."
    rm -f "$ENV_FILE" prover.pbtcfi.toml

    log_success "Cleanup complete!"

    echo ""
    log_info "To clean volumes (database data) as well, run:"
    echo "  podman-compose -f $COMPOSE_FILE down -v"
}

# Full E2E flow
phase_full() {
    log_info "===== FULL E2E TEST FLOW ====="

    phase_setup
    sleep 5
    phase_verify
    echo ""

    log_info "Next steps (manual):"
    echo "  1. Deploy contracts: ./scripts/pbtcfi-e2e-test.sh deploy"
    echo "  2. Configure prover: ./scripts/pbtcfi-e2e-test.sh config-prover"
    echo "  3. Run prover:       ./scripts/pbtcfi-e2e-test.sh run-prover"
    echo ""
    log_info "Or continue interactively? (y/n)"
    read -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        phase_deploy
        phase_config_prover
        echo ""
        log_info "Ready to run prover!"
        log_info "Run: ./scripts/pbtcfi-e2e-test.sh run-prover"
    fi
}

# Main
main() {
    local PHASE="${1:-help}"

    case "$PHASE" in
        setup)
            phase_setup
            ;;
        verify)
            phase_verify
            ;;
        deploy)
            phase_deploy
            ;;
        config-prover)
            phase_config_prover
            ;;
        run-prover)
            phase_run_prover
            ;;
        cleanup)
            phase_cleanup
            ;;
        full)
            phase_full
            ;;
        help|*)
            echo "pBTCFi E2E Test Automation"
            echo ""
            echo "Usage: $0 [phase]"
            echo ""
            echo "Phases:"
            echo "  setup         - Start infrastructure (devnet + postgres + backend)"
            echo "  verify        - Verify all services are healthy"
            echo "  deploy        - Deploy Cairo contracts (interactive)"
            echo "  config-prover - Generate prover configuration"
            echo "  run-prover    - Start prover in foreground"
            echo "  cleanup       - Stop all services and clean up"
            echo "  full          - Run complete setup flow"
            echo ""
            echo "Examples:"
            echo "  $0 full          # Interactive full setup"
            echo "  $0 setup         # Just start infrastructure"
            echo "  $0 verify        # Check services health"
            echo "  $0 cleanup       # Stop everything"
            echo ""
            ;;
    esac
}

main "$@"
