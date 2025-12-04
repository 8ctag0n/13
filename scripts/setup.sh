#!/bin/bash
# Setup script for ZyberLink development environment

set -e

echo "======================================"
echo "ZyberLink Development Setup"
echo "======================================"
echo ""

# Check Rust installation
echo "[1/4] Checking Rust installation..."
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed"
    echo "Install Rust from: https://rustup.rs/"
    exit 1
fi
RUST_VERSION=$(rustc --version)
echo "✅ Rust found: $RUST_VERSION"
echo ""

# Check Solana CLI
echo "[2/4] Checking Solana CLI installation..."
if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI is not installed"
    echo "Install Solana from: https://docs.solana.com/cli/install-solana-cli-tools"
    exit 1
fi
SOLANA_VERSION=$(solana --version)
echo "✅ Solana CLI found: $SOLANA_VERSION"
echo ""

# Check cargo-build-sbf
echo "[3/4] Checking cargo-build-sbf..."
if ! command -v cargo-build-sbf &> /dev/null; then
    echo "⚠️  cargo-build-sbf not found, installing..."
    cargo install cargo-build-sbf
fi
echo "✅ cargo-build-sbf ready"
echo ""

# Build all components
echo "[4/4] Building all components..."
cargo build --all
echo "✅ Build successful"
echo ""

echo "======================================"
echo "✅ Setup complete!"
echo "======================================"
echo ""
echo "Next steps:"
echo "  • Run './scripts/build-all.sh' to build everything"
echo "  • Run 'solana-test-validator' to start local validator"
echo "  • Run './scripts/deploy-local.sh' to deploy program"
echo ""
