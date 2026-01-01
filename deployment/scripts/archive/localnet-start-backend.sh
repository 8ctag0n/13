#!/bin/bash
#
# Start the blink-server backend
#

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if .env exists
if [ ! -f "blink-server/.env" ]; then
    log_error "blink-server/.env not found. Run ./localnet-setup.sh first"
    exit 1
fi

# Load environment variables
export $(grep -v '^#' blink-server/.env | xargs)

log_info "Starting blink-server..."
log_info "  RPC URL: $SOLANA_RPC_URL"
log_info "  Program ID: $PROGRAM_ID"
log_info "  Port: $PORT"

# Kill any existing backend
pkill -f blink-server || true
sleep 2

# Start backend in background
RUST_LOG=info \
DATABASE_URL=$DATABASE_URL \
SOLANA_RPC_URL=$SOLANA_RPC_URL \
PROGRAM_ID=$PROGRAM_ID \
PORT=$PORT \
HOST=$HOST \
./target/release/blink-server > /tmp/blink-server.log 2>&1 &

BACKEND_PID=$!
log_info "Backend started with PID: $BACKEND_PID"

# Wait for backend to be ready
log_info "Waiting for backend to start..."
for i in {1..10}; do
    if curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
        log_info "✓ Backend ready after ${i}s"
        echo ""
        log_info "Backend is running!"
        log_info "  Health: http://127.0.0.1:8080/health"
        log_info "  Logs:   tail -f /tmp/blink-server.log"
        echo ""
        exit 0
    fi
    sleep 1
done

log_error "Backend failed to start"
log_error "Check logs: tail -20 /tmp/blink-server.log"
exit 1
