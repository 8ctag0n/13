#!/bin/bash
#
# E2E Testing - Setup Script
# Prepares infrastructure for E2E tests: compiles programs, builds binaries, creates wallets
#

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}===${NC} $1 ${BLUE}===${NC}\n"
}

# Check we're in project root
if [ ! -f "Cargo.toml" ] || [ ! -d "programs" ]; then
    log_error "Must run from project root"
    exit 1
fi

log_step "E2E Setup - Building Infrastructure"

# ============================================================================
# STEP 1: Compile Solana Programs (SBF)
# ============================================================================
log_step "STEP 1: Compiling Solana Programs"

if [ ! -d "programs" ]; then
    log_error "programs directory not found"
    exit 1
fi

log_info "Building programs with cargo build-sbf..."
if cargo build-sbf --manifest-path programs/Cargo.toml > /tmp/e2e-build-programs.log 2>&1; then
    log_info "Programs compiled successfully"

    # List compiled programs
    if [ -d "programs/target/deploy" ]; then
        log_info "Compiled programs:"
        for prog in programs/target/deploy/*.so; do
            if [ -f "$prog" ]; then
                SIZE=$(du -h "$prog" | cut -f1)
                log_info "  $(basename $prog): $SIZE"
            fi
        done
    fi
else
    log_error "Program compilation failed. Check /tmp/e2e-build-programs.log"
    tail -30 /tmp/e2e-build-programs.log
    exit 1
fi

# ============================================================================
# STEP 2: Build Backend Binaries
# ============================================================================
log_step "STEP 2: Building Backend Binaries"

BINARIES=("blink-server" "x402-server" "zyberlink-prover")

for binary in "${BINARIES[@]}"; do
    log_info "Building $binary..."

    if cargo build --release --bin "$binary" > "/tmp/e2e-build-$binary.log" 2>&1; then
        if [ -f "target/release/$binary" ]; then
            SIZE=$(du -h "target/release/$binary" | cut -f1)
            log_info "  $binary built: $SIZE"
        else
            log_warn "  $binary binary not found at target/release/$binary"
        fi
    else
        log_error "Failed to build $binary. Check /tmp/e2e-build-$binary.log"
        tail -20 "/tmp/e2e-build-$binary.log"
        exit 1
    fi
done

# ============================================================================
# STEP 3: Generate Prover Wallets
# ============================================================================
log_step "STEP 3: Generating Prover Wallets"

log_info "Creating 3 prover keypairs for testing..."

for i in 1 2 3; do
    KEYPAIR="/tmp/prover-$i-keypair.json"

    if solana-keygen new \
        --no-bip39-passphrase \
        --force \
        --outfile "$KEYPAIR" \
        >/dev/null 2>&1; then

        ADDR=$(solana address --keypair "$KEYPAIR" 2>/dev/null)
        log_info "  Prover $i: $ADDR"
        log_info "    Keypair: $KEYPAIR"
    else
        log_error "Failed to generate keypair for prover $i"
        exit 1
    fi
done

# ============================================================================
# STEP 4: Verify Dependencies
# ============================================================================
log_step "STEP 4: Verifying Dependencies"

DEPS_OK=true

# Check solana-test-validator
if command -v solana-test-validator >/dev/null 2>&1; then
    VERSION=$(solana-test-validator --version 2>&1 | head -1)
    log_info "solana-test-validator: $VERSION"
else
    log_error "solana-test-validator not found"
    DEPS_OK=false
fi

# Check solana CLI
if command -v solana >/dev/null 2>&1; then
    VERSION=$(solana --version 2>&1 | head -1)
    log_info "solana-cli: $VERSION"
else
    log_error "solana CLI not found"
    DEPS_OK=false
fi

# Check podman-compose
if command -v podman-compose >/dev/null 2>&1; then
    VERSION=$(podman-compose --version 2>&1 | head -1)
    log_info "podman-compose: $VERSION"
else
    log_warn "podman-compose not found (optional for containers)"
fi

# Check cargo
if command -v cargo >/dev/null 2>&1; then
    VERSION=$(cargo --version)
    log_info "cargo: $VERSION"
else
    log_error "cargo not found"
    DEPS_OK=false
fi

if [ "$DEPS_OK" = false ]; then
    log_error "Missing required dependencies. Install them and try again."
    exit 1
fi

# ============================================================================
# STEP 5: Setup Test Directories
# ============================================================================
log_step "STEP 5: Setting Up Test Directories"

# Create temp directories for E2E artifacts
mkdir -p /tmp/e2e-artifacts
log_info "Created /tmp/e2e-artifacts"

# Create logs directory
mkdir -p /tmp/e2e-logs
log_info "Created /tmp/e2e-logs"

# ============================================================================
# Summary
# ============================================================================
log_step "Setup Complete!"

echo ""
echo "E2E Environment Ready:"
echo ""
echo "Programs:"
ls -lh programs/target/deploy/*.so 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}'
echo ""
echo "Binaries:"
for bin in "${BINARIES[@]}"; do
    if [ -f "target/release/$bin" ]; then
        SIZE=$(du -h "target/release/$bin" | cut -f1)
        echo "  target/release/$bin ($SIZE)"
    fi
done
echo ""
echo "Prover Wallets:"
for i in 1 2 3; do
    if [ -f "/tmp/prover-$i-keypair.json" ]; then
        ADDR=$(solana address --keypair "/tmp/prover-$i-keypair.json" 2>/dev/null)
        echo "  Prover $i: $ADDR"
    fi
done
echo ""
echo "Logs:"
echo "  Build logs: /tmp/e2e-build-*.log"
echo "  Artifacts:  /tmp/e2e-artifacts/"
echo ""
echo "Next Steps:"
echo "  1. Start stack:  ./scripts/e2e/start-stack.sh"
echo "  2. Run tests:    ./scripts/e2e/run-e2e-tests.sh"
echo "  3. Stop stack:   ./scripts/e2e/stop-stack.sh"
echo ""
echo "Or use Makefile:"
echo "  make e2e-start"
echo "  make e2e-test"
echo "  make e2e-stop"
echo ""
