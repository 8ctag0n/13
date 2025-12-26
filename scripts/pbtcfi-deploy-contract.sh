#!/bin/bash
# Deploy pBTCFi Cairo Contract to Katana Devnet
# This script helps deploy PbtcfiJobs contract using the cairo-dev container
#
# Usage:
#   ./scripts/pbtcfi-deploy-contract.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

# Config
DEVNET_RPC="http://localhost:5050"
CAIRO_DIR="/workspace/src/verticals/pbtcfi/contracts/cairo"
CONTAINER_NAME="pbtcfi-cairo-dev"

# Check container is running
if ! podman ps --format "{{.Names}}" | grep -q "$CONTAINER_NAME"; then
    log_error "Container $CONTAINER_NAME not running."
    log_info "Start it with: podman-compose -f docker-compose.pbtcfi.yml up -d"
    exit 1
fi

log_info "===== pBTCFi Contract Deployment ====="
echo ""

# Step 1: Build contracts
log_info "Step 1/4: Building Cairo contracts..."
podman exec -w "$CAIRO_DIR" "$CONTAINER_NAME" scarb build

if [ $? -ne 0 ]; then
    log_error "Scarb build failed"
    exit 1
fi
log_success "Contracts built successfully"
echo ""

# Step 2: Check devnet connectivity
log_info "Step 2/4: Checking Katana devnet connectivity..."
CHAIN_ID=$(curl -s -X POST "$DEVNET_RPC" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}' | jq -r '.result')

if [ -z "$CHAIN_ID" ] || [ "$CHAIN_ID" = "null" ]; then
    log_error "Cannot connect to devnet at $DEVNET_RPC"
    log_info "Is the devnet running? Check: podman logs pbtcfi-devnet"
    exit 1
fi
log_success "Devnet responding (chain ID: $CHAIN_ID)"
echo ""

# Step 3: Get contract artifacts
log_info "Step 3/4: Preparing contract artifacts..."

# Check if artifacts exist
ARTIFACTS_FILE="$CAIRO_DIR/target/dev/pbtcfi_core.starknet_artifacts.json"
if ! podman exec "$CONTAINER_NAME" test -f "$ARTIFACTS_FILE"; then
    log_error "Artifacts not found at $ARTIFACTS_FILE"
    exit 1
fi

# Extract class hash for PbtcfiJobs
CLASS_HASH=$(podman exec "$CONTAINER_NAME" cat "$ARTIFACTS_FILE" | \
  jq -r '.contracts[] | select(.contract_name == "pbtcfi_core::jobs::PbtcfiJobs") | .class_hash')

if [ -z "$CLASS_HASH" ] || [ "$CLASS_HASH" = "null" ]; then
    log_error "Could not find PbtcfiJobs class hash in artifacts"
    exit 1
fi

log_success "PbtcfiJobs class hash: $CLASS_HASH"
echo ""

# Step 4: Deploy using sncast (if available)
log_info "Step 4/4: Deploying contract..."

# Check if sncast is available
if podman exec "$CONTAINER_NAME" which sncast > /dev/null 2>&1; then
    log_info "Using sncast for deployment..."

    # Katana pre-funded account for testing
    PROVER_ADDRESS="0x0517ececd29116499f4a1b64b094da79ba08dfd54a3edaa316134c41f8160973"
    PRIVATE_KEY="0x00c1cf1490de1352865301bb8705143f3ef938f97fdf892f1090dcb5ac7bcd1d"

    # Create temporary account config
    podman exec "$CONTAINER_NAME" bash -c "cat > /tmp/account.json <<EOF
{
  \"version\": 1,
  \"variant\": {
    \"type\": \"open_zeppelin\",
    \"version\": 1,
    \"public_key\": \"0x0\"
  },
  \"deployment\": {
    \"status\": \"deployed\",
    \"class_hash\": \"0x0\",
    \"address\": \"$PROVER_ADDRESS\"
  }
}
EOF"

    # Deploy contract
    DEPLOY_RESULT=$(podman exec -w "$CAIRO_DIR" "$CONTAINER_NAME" bash -c "
      sncast --url http://devnet:5050 \
        --account /tmp/account.json \
        --private-key $PRIVATE_KEY \
        deploy \
        --class-hash $CLASS_HASH \
        --max-fee 999999999999999 2>&1 || echo 'DEPLOY_FAILED'
    ")

    if echo "$DEPLOY_RESULT" | grep -q "DEPLOY_FAILED"; then
        log_error "Deployment failed"
        echo "$DEPLOY_RESULT"
        log_warn "Trying alternative deployment method..."
    else
        # Extract contract address from output
        CONTRACT_ADDRESS=$(echo "$DEPLOY_RESULT" | grep -oP 'contract_address: 0x[0-9a-fA-F]+' | cut -d' ' -f2)

        if [ -n "$CONTRACT_ADDRESS" ]; then
            log_success "Contract deployed successfully!"
            echo ""
            echo "CONTRACT_ADDRESS=$CONTRACT_ADDRESS"
            echo "CLASS_HASH=$CLASS_HASH"
            echo ""

            # Save to .env file
            ENV_FILE=".env.pbtcfi"
            cat > "$ENV_FILE" <<EOF
# pBTCFi E2E Test Environment
# Generated: $(date)

STARKNET_RPC_URL=$DEVNET_RPC
PBTCFI_CONTRACT_ADDRESS=$CONTRACT_ADDRESS
PBTCFI_CLASS_HASH=$CLASS_HASH

# Katana devnet pre-funded account #0
STARKNET_PROVER_ADDRESS=$PROVER_ADDRESS
STARKNET_PRIVATE_KEY=$PRIVATE_KEY
EOF
            log_success "Configuration saved to $ENV_FILE"
            exit 0
        fi
    fi
fi

# Fallback: Manual deployment instructions
log_warn "Automated deployment not available."
echo ""
log_info "Manual deployment steps:"
echo ""
echo "1. Enter cairo-dev container:"
echo "   podman exec -it $CONTAINER_NAME bash"
echo ""
echo "2. Navigate to contracts directory:"
echo "   cd $CAIRO_DIR"
echo ""
echo "3. Deploy using sncast/starkli:"
echo "   sncast deploy --class-hash $CLASS_HASH"
echo ""
echo "4. Save the contract address and run:"
echo "   ./scripts/pbtcfi-e2e-test.sh deploy"
echo ""
