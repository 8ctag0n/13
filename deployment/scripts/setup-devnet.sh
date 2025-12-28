#!/bin/bash
#
# ZyberLink Devnet Setup Script
# Configura todo para correr en Solana Devnet
#
# Uso: ./scripts/setup-devnet.sh --funder /path/to/keypair-con-sol.json
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Defaults
SOLANA_RPC_URL="https://api.devnet.solana.com"
FUNDER_KEYPAIR=""
SOL_PER_ACCOUNT="0.5"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --funder)
            FUNDER_KEYPAIR="$2"
            shift 2
            ;;
        --rpc)
            SOLANA_RPC_URL="$2"
            shift 2
            ;;
        --sol-per-account)
            SOL_PER_ACCOUNT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 --funder <keypair.json> [--rpc <url>] [--sol-per-account <amount>]"
            echo ""
            echo "Options:"
            echo "  --funder           Keypair con SOL para fondear las demás cuentas"
            echo "  --rpc              Solana RPC URL (default: https://api.devnet.solana.com)"
            echo "  --sol-per-account  SOL a transferir a cada cuenta (default: 0.5)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            ;;
    esac
done

# Validate funder
if [ -z "$FUNDER_KEYPAIR" ]; then
    log_error "Falta --funder. Uso: $0 --funder /path/to/keypair.json"
fi

if [ ! -f "$FUNDER_KEYPAIR" ]; then
    log_error "Funder keypair no existe: $FUNDER_KEYPAIR"
fi

# Validate dependencies
log_info "Validando dependencias..."
for cmd in solana solana-keygen cargo openssl podman-compose bc; do
    if ! command -v $cmd &> /dev/null; then
        log_error "$cmd no está instalado"
    fi
done
log_ok "Dependencias OK"

cd "$PROJECT_DIR"

echo ""
echo "==========================================="
echo "  ZyberLink Devnet Setup"
echo "==========================================="
echo ""
log_info "RPC URL: $SOLANA_RPC_URL"
log_info "Funder: $FUNDER_KEYPAIR"
echo ""

# Configure solana CLI
log_info "Configurando Solana CLI..."
solana config set --url "$SOLANA_RPC_URL" > /dev/null
solana config set --keypair "$FUNDER_KEYPAIR" > /dev/null

# Check funder balance
FUNDER_BALANCE=$(solana balance --keypair "$FUNDER_KEYPAIR" | cut -d' ' -f1)
log_info "Balance del funder: $FUNDER_BALANCE SOL"

if (( $(echo "$FUNDER_BALANCE < 5" | bc -l) )); then
    log_warn "Balance bajo. Se necesitan ~5 SOL para deploy completo."
    read -p "¿Continuar de todos modos? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Create keypairs directory
log_info "Creando directorio keypairs..."
mkdir -p keypairs

# Generate keypairs
log_info "Generando keypairs..."

generate_keypair() {
    local name=$1
    local path="keypairs/${name}.json"
    if [ ! -f "$path" ]; then
        solana-keygen new --no-bip39-passphrase --outfile "$path" --force > /dev/null 2>&1
        echo -e "${GREEN}[OK]${NC} Generada: $path" >&2
    else
        echo -e "${YELLOW}[WARN]${NC} Ya existe: $path (reusando)" >&2
    fi
    echo "$path"
}

BACKEND_KEYPAIR=$(generate_keypair "backend")
PROVER1_KEYPAIR=$(generate_keypair "prover-1")
PROVER2_KEYPAIR=$(generate_keypair "prover-2")
PROVER3_KEYPAIR=$(generate_keypair "prover-3")

# Fund accounts
log_info "Fondeando cuentas desde funder..."

fund_account() {
    local keypair=$1
    local name=$2
    local pubkey=$(solana-keygen pubkey "$keypair")
    local balance=$(solana balance "$pubkey" 2>/dev/null | cut -d' ' -f1 || echo "0")

    if (( $(echo "$balance < 0.1" | bc -l) )); then
        log_info "Transfiriendo $SOL_PER_ACCOUNT SOL a $name ($pubkey)..."
        solana transfer --keypair "$FUNDER_KEYPAIR" "$pubkey" "$SOL_PER_ACCOUNT" --allow-unfunded-recipient > /dev/null
        log_ok "$name fondeado"
    else
        log_warn "$name ya tiene $balance SOL"
    fi
}

fund_account "$BACKEND_KEYPAIR" "backend"
fund_account "$PROVER1_KEYPAIR" "prover-1"
fund_account "$PROVER2_KEYPAIR" "prover-2"
fund_account "$PROVER3_KEYPAIR" "prover-3"

# Build program
log_info "Compilando programa Solana..."
cd src/programs
if [ ! -f "target/deploy/zyberlink.so" ]; then
    cargo build-sbf
    log_ok "Programa compilado"
else
    log_warn "Programa ya compilado (reusando)"
fi
cd "$PROJECT_DIR"

# Deploy program
log_info "Desplegando programa a devnet..."
PROGRAM_KEYPAIR="src/programs/target/deploy/zyberlink-keypair.json"

if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    log_error "No se encuentra keypair del programa"
fi

PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
log_info "Program ID: $PROGRAM_ID"

# Check if already deployed
DEPLOYED=$(solana program show "$PROGRAM_ID" 2>&1 || echo "not found")
if [[ "$DEPLOYED" == *"not found"* ]] || [[ "$DEPLOYED" == *"does not exist"* ]]; then
    log_info "Desplegando programa por primera vez..."
    solana program deploy \
        src/programs/target/deploy/zyberlink.so \
        --program-id "$PROGRAM_KEYPAIR" \
        --keypair "$FUNDER_KEYPAIR"
    log_ok "Programa desplegado"
else
    log_warn "Programa ya desplegado en devnet"
fi

# Generate DB password (consistent across all configs)
DB_PASSWORD="zyberlink_devnet_$(openssl rand -hex 4)"

# Generate .env.devnet
log_info "Generando .env.devnet..."
cat > .env.devnet << EOF
# ZyberLink Devnet Configuration
# Generated by setup-devnet.sh on $(date)

SOLANA_RPC_URL=$SOLANA_RPC_URL
PROGRAM_ID=$PROGRAM_ID
DB_PASSWORD=$DB_PASSWORD

# Keypairs
BACKEND_KEYPAIR=./keypairs/backend.json
PROVER1_KEYPAIR=./keypairs/prover-1.json
PROVER2_KEYPAIR=./keypairs/prover-2.json
PROVER3_KEYPAIR=./keypairs/prover-3.json

# Funder (for reference)
FUNDER_KEYPAIR=$FUNDER_KEYPAIR
EOF

log_ok ".env.devnet generado"

# Build backend binary (required for docker-compose)
log_info "Compilando backend server..."
if [ ! -f "target/release/blink-server" ]; then
    cargo build --release --bin blink-server 2>&1 | tail -5
    log_ok "Backend compilado"
else
    log_warn "Backend ya compilado (reusando)"
fi

# Update services/blink-server/.env for sqlx
log_info "Actualizando config del backend..."
cat > services/blink-server/.env << EOF
DATABASE_URL=postgresql://zyberlink:${DB_PASSWORD}@localhost:5432/zyberlink
SOLANA_RPC_URL=$SOLANA_RPC_URL
PROGRAM_ID=$PROGRAM_ID
EOF

# Final summary
echo ""
echo "==========================================="
echo "  Setup Completo"
echo "==========================================="
echo ""
log_ok "Program ID: $PROGRAM_ID"
log_ok "RPC URL: $SOLANA_RPC_URL"
echo ""
echo "Keypairs generadas:"
echo "  - Backend: $(solana-keygen pubkey $BACKEND_KEYPAIR)"
echo "  - Prover 1: $(solana-keygen pubkey $PROVER1_KEYPAIR)"
echo "  - Prover 2: $(solana-keygen pubkey $PROVER2_KEYPAIR)"
echo "  - Prover 3: $(solana-keygen pubkey $PROVER3_KEYPAIR)"
echo ""
echo "Siguientes pasos:"
echo "  1. make d1   # Levantar containers"
echo "  2. make d2   # Init marketplace + registrar provers"
echo "  3. make d3   # Start provers"
echo ""
echo "URLs (después de make d1):"
echo "  - Frontend: http://localhost:9000"
echo "  - API: http://localhost:9000/api"
echo ""
