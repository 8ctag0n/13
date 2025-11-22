#!/bin/bash
#
# View localnet logs
#

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

show_menu() {
    echo ""
    echo -e "${YELLOW}ZyberLink Logs${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "1. Backend API"
    echo "2. Validator"
    echo "3. Prover 1"
    echo "4. Prover 2"
    echo "5. Prover 3"
    echo "6. All Provers"
    echo "7. All Services"
    echo "q. Quit"
    echo ""
    echo -n "Select: "
}

while true; do
    show_menu
    read -r choice

    case $choice in
        1) tail -f /tmp/blink-server.log ;;
        2) tail -f /tmp/solana-validator.log ;;
        3) tail -f /tmp/prover-1.log ;;
        4) tail -f /tmp/prover-2.log ;;
        5) tail -f /tmp/prover-3.log ;;
        6) tail -f /tmp/prover-*.log ;;
        7) tail -f /tmp/blink-server.log /tmp/prover-*.log ;;
        q|Q) exit 0 ;;
        *) echo "Invalid option" ;;
    esac
done
