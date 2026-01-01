#!/bin/bash
#
# Check status of all ZyberLink localnet services
#

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

check_service() {
    local name=$1
    local check_cmd=$2

    if eval "$check_cmd" >/dev/null 2>&1; then
        echo -e "  ${GREEN}[OK]${NC} $name"
        return 0
    else
        echo -e "  ${RED}[FAIL]${NC} $name"
        return 1
    fi
}

echo ""
echo "==========================================="
echo "  ZyberLink Localnet Status"
echo "==========================================="
echo ""

# Infrastructure
echo -e "${YELLOW}Infrastructure:${NC}"
check_service "Validator (port 8899)" "solana cluster-version --url http://localhost:8899"
check_service "Database (PostgreSQL)" "podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c 'SELECT 1'"
check_service "Backend API (port 8080)" "curl -s http://127.0.0.1:8080/health"
echo ""

# Provers
echo -e "${YELLOW}Prover Nodes:${NC}"
PROVER_COUNT=$(ps aux | grep -E "zyberlink-prover|prover-node" | grep -v grep | wc -l)
if [ "$PROVER_COUNT" -gt 0 ]; then
    echo -e "  ${GREEN}[OK]${NC} $PROVER_COUNT prover(s) running"
    ps aux | grep -E "zyberlink-prover|prover-node" | grep -v grep | awk '{print "    - PID " $2}'
else
    echo -e "  ${RED}[FAIL]${NC} No provers running"
fi
echo ""

# Prover wallets
echo -e "${YELLOW}Prover Wallets:${NC}"
for i in 1 2 3; do
    if [ -f "/tmp/prover-$i-keypair.json" ]; then
        ADDR=$(solana address --keypair /tmp/prover-$i-keypair.json 2>/dev/null)
        BALANCE=$(solana balance --keypair /tmp/prover-$i-keypair.json --url http://localhost:8899 2>/dev/null | awk '{print $1}')
        if [ -n "$BALANCE" ]; then
            echo -e "  ${GREEN}[OK]${NC} Prover $i: $BALANCE SOL ($ADDR)"
        else
            echo -e "  ${YELLOW}[WARN]${NC} Prover $i: No balance (validator down?)"
        fi
    else
        echo -e "  ${RED}[FAIL]${NC} Prover $i: Wallet not found"
    fi
done
echo ""

# Program
echo -e "${YELLOW}Solana Program:${NC}"
if [ -f "services/blink-server/.env" ]; then
    PROGRAM_ID=$(grep PROGRAM_ID services/blink-server/.env | cut -d= -f2)
    if solana account "$PROGRAM_ID" --url http://localhost:8899 >/dev/null 2>&1; then
        echo -e "  ${GREEN}[OK]${NC} Program deployed: $PROGRAM_ID"
    else
        echo -e "  ${RED}[FAIL]${NC} Program not found on-chain"
    fi
else
    echo -e "  ${RED}[FAIL]${NC} Program ID not configured (.env missing)"
fi
echo ""

# Logs
echo -e "${YELLOW}Log Files:${NC}"
for log in /tmp/solana-validator.log /tmp/blink-server.log /tmp/prover-{1,2,3}.log; do
    if [ -f "$log" ]; then
        SIZE=$(du -h "$log" | awk '{print $1}')
        echo -e "  ${GREEN}[OK]${NC} $(basename $log) ($SIZE)"
    fi
done
echo ""

# Summary
echo -e "${BLUE}=======================================${NC}"
VALIDATOR_OK=$(solana cluster-version --url http://localhost:8899 >/dev/null 2>&1 && echo 1 || echo 0)
BACKEND_OK=$(curl -s http://127.0.0.1:8080/health >/dev/null 2>&1 && echo 1 || echo 0)
DB_OK=$(podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c 'SELECT 1' >/dev/null 2>&1 && echo 1 || echo 0)

TOTAL=$((VALIDATOR_OK + BACKEND_OK + DB_OK))

if [ "$TOTAL" -eq 3 ]; then
    echo -e "${GREEN}Status: All systems operational [OK]${NC}"
elif [ "$TOTAL" -gt 0 ]; then
    echo -e "${YELLOW}Status: Partial ($TOTAL/3 services up)${NC}"
    echo ""
    echo "Run: make l1 (or deployment/scripts/start-localnet.sh)"
else
    echo -e "${RED}Status: System down${NC}"
    echo ""
    echo "Run: make l1 (or deployment/scripts/start-localnet.sh)"
fi
echo -e "${BLUE}=======================================${NC}"
echo ""
