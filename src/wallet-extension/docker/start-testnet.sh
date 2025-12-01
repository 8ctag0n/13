#!/bin/bash
# Start local testnet for ZyberLink Wallet testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=================================="
echo " ZyberLink Local Testnet"
echo "=================================="
echo ""

# Check if docker/podman is available
if command -v podman &> /dev/null; then
    COMPOSE="podman-compose"
elif command -v docker &> /dev/null; then
    COMPOSE="docker compose"
else
    echo "ERROR: Neither docker nor podman found"
    exit 1
fi

echo "[1/3] Starting containers..."
$COMPOSE up -d

echo ""
echo "[2/3] Waiting for services to be ready..."

# Wait for Solana
echo -n "  Solana: "
for i in {1..30}; do
    if curl -s http://localhost:8899/health > /dev/null 2>&1; then
        echo "Ready"
        break
    fi
    echo -n "."
    sleep 2
done

# Wait for Starknet
echo -n "  Starknet: "
for i in {1..30}; do
    if curl -s http://localhost:5050 > /dev/null 2>&1; then
        echo "Ready"
        break
    fi
    echo -n "."
    sleep 2
done

# Wait for Zcash
echo -n "  Zcash: "
for i in {1..30}; do
    if curl -s --user zyberlink:testpass123 --data-binary '{"jsonrpc":"1.0","method":"getblockchaininfo","params":[]}' http://localhost:18232 > /dev/null 2>&1; then
        echo "Ready"
        break
    fi
    echo -n "."
    sleep 2
done

echo ""
echo "[3/3] Generating initial funds..."

# Airdrop SOL to a test address (if needed)
echo "  Solana: Validator running with faucet at :9900"

# Generate some Zcash blocks
echo "  Zcash: Mining initial blocks..."
curl -s --user zyberlink:testpass123 \
    --data-binary '{"jsonrpc":"1.0","method":"generate","params":[10]}' \
    http://localhost:18232 > /dev/null 2>&1 || true

echo ""
echo "=================================="
echo " Testnet Ready!"
echo "=================================="
echo ""
echo " RPC Endpoints:"
echo "   Solana:   http://localhost:8899"
echo "   Starknet: http://localhost:5050"
echo "   Zcash:    http://localhost:18232"
echo ""
echo " Zcash RPC Auth: zyberlink:testpass123"
echo ""
echo " Stop with: ./stop-testnet.sh"
echo "=================================="
