#!/bin/bash
# Start full ZyberLink demo stack

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "======================================"
echo "  ZyberLink Demo Stack Launcher"
echo "======================================"
echo ""

# Check if postgres is running
if ! podman ps | grep -q zyberlink-postgres; then
    echo "[1/6] Starting PostgreSQL..."
    podman-compose up -d
    sleep 3
else
    echo "[1/6] PostgreSQL already running"
fi

# Check validator
echo "[2/6] Checking Solana validator..."
if solana cluster-version 2>/dev/null; then
    echo "       Validator OK"
else
    echo "       ERROR: Validator not running!"
    echo "       Run: solana-test-validator --reset &"
    exit 1
fi

# Start backend
echo "[3/6] Starting backend server..."
pkill -f blink-server || true
sleep 1

# Load env from backend if exists, otherwise use defaults
if [ -f "src/blink-server/.env" ]; then
    export $(grep -v '^#' src/blink-server/.env | xargs)
else
    export DATABASE_URL=postgresql://zyberlink:dev_password@localhost:5432/zyberlink
    export SOLANA_RPC_URL=http://localhost:8899
    export PROGRAM_ID=ZyberLinkProgram11111111111111111111111111
fi

RUST_LOG=info \
DATABASE_URL=$DATABASE_URL \
SOLANA_RPC_URL=$SOLANA_RPC_URL \
PROGRAM_ID=$PROGRAM_ID \
PORT=8080 \
HOST=127.0.0.1 \
./target/release/blink-server > ~/zyberlink-logs/backend.log 2>&1 &
BACKEND_PID=$!
echo "       Backend PID: $BACKEND_PID"
sleep 2

# Verify backend is up
if curl -s http://localhost:8080/health > /dev/null; then
    echo "       Backend OK"
else
    echo "       ERROR: Backend failed to start!"
    tail -20 ~/zyberlink-logs/backend.log
    exit 1
fi

# Start 3 provers
echo "[4/6] Starting 3 prover nodes..."
pkill -f zyberlink-prover || true
sleep 1

for i in 1 2 3; do
    PROVER_KEYPAIR="/tmp/prover-$i-keypair.json"

    # Create keypair if doesn't exist
    if [ ! -f "$PROVER_KEYPAIR" ]; then
        solana-keygen new --no-bip39-passphrase --force --outfile $PROVER_KEYPAIR >/dev/null 2>&1
        solana airdrop 5 $(solana address --keypair $PROVER_KEYPAIR) --url $SOLANA_RPC_URL >/dev/null 2>&1
    fi

    RUST_LOG=info \
    ./target/release/zyberlink-prover \
        --program-id $PROGRAM_ID \
        --rpc-url $SOLANA_RPC_URL \
        --keypair $PROVER_KEYPAIR \
        > ~/zyberlink-logs/prover-$i.log 2>&1 &
    echo "       Prover $i PID: $!"
done
sleep 2

# Start frontend
echo "[5/6] Starting frontend..."
pkill -f "vite" || true
cd src/webapp
npm run dev > ~/zyberlink-logs/frontend.log 2>&1 &
FRONTEND_PID=$!
echo "       Frontend PID: $FRONTEND_PID"
sleep 3
cd ../..

echo "[6/6] All services started!"
echo ""
echo "======================================"
echo "  Stack Running"
echo "======================================"
echo ""
echo "Services:"
echo "  - Validator:  http://localhost:8899"
echo "  - Backend:    http://localhost:8080"
echo "  - Frontend:   http://localhost:5173"
echo "  - Database:   postgresql://localhost:5432/zyberlink"
echo "  - Prover 1-3: Running"
echo ""
echo "Logs:"
echo "  - Backend:  tail -f ~/zyberlink-logs/backend.log"
echo "  - Prover 1: tail -f ~/zyberlink-logs/prover-1.log"
echo "  - Prover 2: tail -f ~/zyberlink-logs/prover-2.log"
echo "  - Prover 3: tail -f ~/zyberlink-logs/prover-3.log"
echo "  - Frontend: tail -f ~/zyberlink-logs/frontend.log"
echo ""
echo "Press Ctrl+C to stop (or run: ./stop-demo.sh)"
echo ""

# Wait for user interrupt
trap "echo ''; echo 'Stopping services...'; pkill -f blink-server; pkill -f zyberlink-prover; pkill -f vite; echo 'Done!'; exit 0" INT

wait
