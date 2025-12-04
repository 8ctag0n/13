#!/bin/bash
# Build all ZyberLink components

set -e

echo "======================================"
echo "Building ZyberLink"
echo "======================================"
echo ""

# Build shared libraries
echo "[1/4] Building shared types..."
cargo build --package zyberlink-types
echo "✅ Shared types built"
echo ""

echo "[2/4] Building shared crypto..."
cargo build --package zyberlink-crypto
echo "✅ Shared crypto built"
echo ""

# Build SDK
echo "[3/4] Building SDK..."
cargo build --package zyberlink-sdk
echo "✅ SDK built"
echo ""

# Build Solana program
echo "[4/4] Building Solana program..."
cd src/programs/zyberlink
cargo build-sbf
cd ../../..
echo "✅ Solana program built"
echo ""

# Build prover node
echo "[5/5] Building prover node..."
cargo build --package zyberlink-prover --release
echo "✅ Prover node built"
echo ""

echo "======================================"
echo "✅ All components built successfully!"
echo "======================================"
echo ""
echo "Build artifacts:"
echo "  • Program: target/deploy/zyberlink.so"
echo "  • Prover: target/release/zyberlink-prover"
echo ""
