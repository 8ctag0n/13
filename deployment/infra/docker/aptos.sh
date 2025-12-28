#!/bin/bash
# Aptos Local Testnet Helper Script

set -e

COMPOSE_FILE="docker-compose.aptos.yml"
CONTAINER_NAME="zyberlink-aptos-node"

# Detect container runtime
if command -v podman-compose &> /dev/null; then
    COMPOSE_CMD="podman-compose"
elif command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
elif command -v docker &> /dev/null && docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
else
    echo "Error: No container runtime found (docker-compose, podman-compose, or docker compose)"
    exit 1
fi

case "$1" in
  start)
    echo "Starting Aptos local testnet (using $COMPOSE_CMD)..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" up -d
    echo "Waiting for node to be healthy..."
    sleep 5
    $COMPOSE_CMD -f "$COMPOSE_FILE" ps
    echo ""
    echo "Aptos local testnet is running:"
    echo "  REST API: http://localhost:8080"
    echo "  Faucet:   http://localhost:8081"
    ;;

  stop)
    echo "Stopping Aptos local testnet..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" down
    ;;

  restart)
    echo "Restarting Aptos local testnet..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" down
    $COMPOSE_CMD -f "$COMPOSE_FILE" up -d
    ;;

  reset)
    echo "Resetting Aptos local testnet (clearing all data)..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" down -v
    $COMPOSE_CMD -f "$COMPOSE_FILE" up -d
    echo "Fresh testnet started!"
    ;;

  logs)
    $COMPOSE_CMD -f "$COMPOSE_FILE" logs -f aptos-node
    ;;

  health)
    echo "Checking Aptos node health..."
    curl -s http://localhost:8080/v1 | jq -r '.chain_id, .ledger_version, .ledger_timestamp' || echo "Node not responding"
    ;;

  fund)
    if [ -z "$2" ]; then
      echo "Usage: $0 fund <address>"
      exit 1
    fi
    ADDRESS="$2"
    AMOUNT="${3:-100000000}"  # Default 1 APT
    echo "Funding address $ADDRESS with $AMOUNT octas..."
    curl -X POST "http://localhost:8081/mint?amount=$AMOUNT&address=$ADDRESS"
    echo ""
    ;;

  *)
    echo "Aptos Local Testnet Helper"
    echo ""
    echo "Usage: $0 {start|stop|restart|reset|logs|health|fund}"
    echo ""
    echo "Commands:"
    echo "  start   - Start the local testnet"
    echo "  stop    - Stop the local testnet"
    echo "  restart - Restart the local testnet"
    echo "  reset   - Stop and clear all data (fresh start)"
    echo "  logs    - Show logs"
    echo "  health  - Check node health"
    echo "  fund    - Fund an address (usage: fund <address> [amount])"
    exit 1
    ;;
esac
