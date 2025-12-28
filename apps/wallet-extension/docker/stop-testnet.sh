#!/bin/bash
# Stop local testnet

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Stopping ZyberLink testnet..."

if command -v podman &> /dev/null; then
    podman-compose down
elif command -v docker &> /dev/null; then
    docker compose down
fi

echo "Testnet stopped."
