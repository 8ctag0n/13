#!/bin/bash
# Script para generar pruebas con el circuito VERIFY_GROTH16

set -e

CIRCUIT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SNARKJS="${CIRCUIT_DIR}/../node_modules/snarkjs/build/cli.cjs"

if [ ! -f "$SNARKJS" ]; then
    echo "Error: snarkjs not found. Run 'bun install' in circuits/"
    exit 1
fi

if [ $# -lt 1 ]; then
    echo "Usage: $0 <input.json> [output_dir]"
    echo ""
    echo "Generates:"
    echo "  - witness.wtns"
    echo "  - proof.json"
    echo "  - public.json"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_DIR="${2:-.}"

if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file not found: $INPUT_FILE"
    exit 1
fi

echo "=== Generating Groth16 Proof ==="
echo "Input: $INPUT_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Step 1: Generate witness
echo "[1/3] Generating witness..."
node "$SNARKJS" wtns calculate \
    "${CIRCUIT_DIR}/build/circuit_js/circuit.wasm" \
    "$INPUT_FILE" \
    "${OUTPUT_DIR}/witness.wtns"

# Step 2: Generate proof
echo "[2/3] Generating proof..."
node "$SNARKJS" groth16 prove \
    "${CIRCUIT_DIR}/circuit_final.zkey" \
    "${OUTPUT_DIR}/witness.wtns" \
    "${OUTPUT_DIR}/proof.json" \
    "${OUTPUT_DIR}/public.json"

# Step 3: Verify proof
echo "[3/3] Verifying proof..."
node "$SNARKJS" groth16 verify \
    "${CIRCUIT_DIR}/verification_key.json" \
    "${OUTPUT_DIR}/public.json" \
    "${OUTPUT_DIR}/proof.json"

echo ""
echo "=== Proof Generation Complete ==="
echo "Files generated:"
echo "  - ${OUTPUT_DIR}/witness.wtns"
echo "  - ${OUTPUT_DIR}/proof.json"
echo "  - ${OUTPUT_DIR}/public.json"
