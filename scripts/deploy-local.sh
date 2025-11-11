#!/bin/bash
# Deploy CypherLink program to local validator

set -e

echo "======================================"
echo "Deploying to Local Validator"
echo "======================================"
echo ""

# Check if validator is running
if ! pgrep -x "solana-test-validator" > /dev/null; then
    echo "⚠️  Local validator is not running"
    echo "Start it with: solana-test-validator"
    exit 1
fi

echo "[1/2] Building program..."
cd programs/cypherlink
cargo build-sbf
cd ../..
echo "✅ Program built"
echo ""

echo "[2/2] Deploying program..."
PROGRAM_PATH="target/deploy/cypherlink.so"

if [ ! -f "$PROGRAM_PATH" ]; then
    echo "❌ Program binary not found at $PROGRAM_PATH"
    exit 1
fi

PROGRAM_ID=$(solana program deploy $PROGRAM_PATH --url localhost --keypair ~/.config/solana/id.json)
echo "✅ Program deployed"
echo ""

echo "======================================"
echo "✅ Deployment complete!"
echo "======================================"
echo ""
echo "Program ID: $PROGRAM_ID"
echo ""
