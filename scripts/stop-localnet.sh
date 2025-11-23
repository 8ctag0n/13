#!/bin/bash
#
# Stop all ZyberLink localnet services
#

GREEN='\033[0;32m'
NC='\033[0m'

echo ""
echo "Stopping all services..."

pkill -f solana-test-validator && echo "  [OK] Validator stopped" || true
pkill -f blink-server && echo "  [OK] Backend stopped" || true
pkill -f cypherlink-prover && echo "  [OK] Provers stopped" || true

sleep 2

echo -e "${GREEN}[OK] All services stopped${NC}"
echo ""
