#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Deploying Futarchy Markets ===${NC}"

# 1. Compile
echo "Compiling futarchy-markets..."
cd chain/solana/programs/futarchy-markets
cargo build-sbf
cd ../../..

# 2. Keypair
KEYPAIR="chain/solana/programs/target/deploy/futarchy_markets-keypair.json"
BINARY="chain/solana/programs/target/deploy/futarchy_markets.so"

if [ ! -f "$KEYPAIR" ]; then
    echo "Creating keypair..."
    solana-keygen new --no-bip39-passphrase --force --outfile "$KEYPAIR" >/dev/null 2>&1
fi

# 3. Deploy
echo "Deploying to localnet..."
DEPLOY_OUTPUT=$(solana program deploy "$BINARY" \
    --url http://localhost:8899 \
    --program-id "$KEYPAIR" \
    --keypair ~/.config/solana/id.json)

PROGRAM_ID=$(solana address --keypair "$KEYPAIR")
echo -e "${GREEN}Futarchy Program ID: $PROGRAM_ID${NC}"

# 4. Update .env
ENV_FILE="services/blink-server/.env"
if [ -f "$ENV_FILE" ]; then
    # Remove old entry if exists
    grep -v "FUTARCHY_PROGRAM_ID" "$ENV_FILE" > "$ENV_FILE.tmp" && mv "$ENV_FILE.tmp" "$ENV_FILE"
    # Add new entry
    echo "FUTARCHY_PROGRAM_ID=$PROGRAM_ID" >> "$ENV_FILE"
    echo "Updated $ENV_FILE"
else
    echo -e "${RED}Error: $ENV_FILE not found. Run setup-localnet.sh first.${NC}"
    exit 1
fi

# 5. Restart backend if running
if pgrep -f "blink-server" > /dev/null; then
    echo "Restarting blink-server to pick up new config..."
    pkill -f blink-server
    sleep 2
    # Restart in background (same args as Makefile start-backend)
    export $(grep -v '^#' services/blink-server/.env | xargs)
    RUST_LOG=info HOST=127.0.0.1 PORT=8080 ./target/release/blink-server > /tmp/blink-server.log 2>&1 &
    echo "Backend restarted."
fi

echo -e "${GREEN}Futarchy deployment complete!${NC}"
