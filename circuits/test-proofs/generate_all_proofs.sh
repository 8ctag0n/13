#!/bin/bash

CIRCUITS_DIR="/home/deploy/2025q4/13-area1-circuits/circuits"
INPUTS_DIR="$CIRCUITS_DIR/test-proofs/inputs"
OUTPUTS_DIR="$CIRCUITS_DIR/test-proofs/outputs"

mkdir -p "$OUTPUTS_DIR"

echo "=========================================="
echo "ZyberLink ZK Circuits - Proof Generation"
echo "=========================================="

# Track results
PASSED=0
FAILED=0

generate_and_verify() {
    local circuit_id=$1
    local circuit_dir=$2
    local circuit_name=$3

    echo -e "\n[Circuit $circuit_id] $circuit_name"
    echo "-------------------------------------------"

    cd "$CIRCUITS_DIR/$circuit_dir"

    # Check if files exist
    if [ ! -f "${circuit_name}_js/${circuit_name}.wasm" ]; then
        echo "  ERROR: WASM not found"
        ((FAILED++))
        return 1
    fi

    if [ ! -f "${circuit_name}_final.zkey" ]; then
        echo "  ERROR: Final zkey not found"
        ((FAILED++))
        return 1
    fi

    # Generate witness
    echo "  Generating witness..."
    node ${circuit_name}_js/generate_witness.js \
        ${circuit_name}_js/${circuit_name}.wasm \
        "$INPUTS_DIR/circuit_${circuit_id}_test.json" \
        witness_${circuit_id}.wtns 2>/dev/null

    if [ $? -ne 0 ]; then
        echo "  ERROR: Witness generation failed"
        ((FAILED++))
        return 1
    fi

    # Generate proof
    echo "  Generating proof..."
    npx snarkjs groth16 prove \
        ${circuit_name}_final.zkey \
        witness_${circuit_id}.wtns \
        "$OUTPUTS_DIR/circuit_${circuit_id}_proof.json" \
        "$OUTPUTS_DIR/circuit_${circuit_id}_public.json" 2>/dev/null

    if [ $? -ne 0 ]; then
        echo "  ERROR: Proof generation failed"
        ((FAILED++))
        return 1
    fi

    # Verify proof
    echo "  Verifying proof..."
    VERIFY_OUTPUT=$(npx snarkjs groth16 verify \
        ${circuit_name}_vkey.json \
        "$OUTPUTS_DIR/circuit_${circuit_id}_public.json" \
        "$OUTPUTS_DIR/circuit_${circuit_id}_proof.json" 2>&1)

    if echo "$VERIFY_OUTPUT" | grep -q "OK"; then
        echo "  OK - Proof verified successfully"
        ((PASSED++))
        return 0
    else
        echo "  ERROR: Verification failed"
        ((FAILED++))
        return 1
    fi
}

# Circuit definitions: id, directory, wasm_name, zkey_prefix, vkey_name
# Format: generate_and_verify circuit_id dir wasm_name zkey_prefix vkey_name

generate_and_verify_v2() {
    local circuit_id=$1
    local circuit_dir=$2
    local wasm_name=$3
    local zkey_prefix=$4
    local vkey_name=$5

    echo -e "\n[Circuit $circuit_id] $wasm_name"
    echo "-------------------------------------------"

    cd "$CIRCUITS_DIR/$circuit_dir"

    # Check if files exist
    if [ ! -f "${wasm_name}_js/${wasm_name}.wasm" ]; then
        echo "  ERROR: WASM not found: ${wasm_name}_js/${wasm_name}.wasm"
        ((FAILED++))
        return 1
    fi

    if [ ! -f "${zkey_prefix}_final.zkey" ]; then
        echo "  ERROR: Final zkey not found: ${zkey_prefix}_final.zkey"
        ((FAILED++))
        return 1
    fi

    # Generate witness
    echo "  Generating witness..."
    node ${wasm_name}_js/generate_witness.js \
        ${wasm_name}_js/${wasm_name}.wasm \
        "$INPUTS_DIR/circuit_${circuit_id}_test.json" \
        witness_${circuit_id}.wtns 2>/dev/null

    if [ $? -ne 0 ]; then
        echo "  ERROR: Witness generation failed"
        ((FAILED++))
        return 1
    fi

    # Generate proof
    echo "  Generating proof..."
    npx snarkjs groth16 prove \
        ${zkey_prefix}_final.zkey \
        witness_${circuit_id}.wtns \
        "$OUTPUTS_DIR/circuit_${circuit_id}_proof.json" \
        "$OUTPUTS_DIR/circuit_${circuit_id}_public.json" 2>/dev/null

    if [ $? -ne 0 ]; then
        echo "  ERROR: Proof generation failed"
        ((FAILED++))
        return 1
    fi

    # Verify proof
    echo "  Verifying proof..."
    VERIFY_OUTPUT=$(npx snarkjs groth16 verify \
        ${vkey_name}_vkey.json \
        "$OUTPUTS_DIR/circuit_${circuit_id}_public.json" \
        "$OUTPUTS_DIR/circuit_${circuit_id}_proof.json" 2>&1)

    if echo "$VERIFY_OUTPUT" | grep -q "OK"; then
        echo "  OK - Proof verified successfully"
        ((PASSED++))
        return 0
    else
        echo "  ERROR: Verification failed"
        ((FAILED++))
        return 1
    fi
}

# Generate proofs for all circuits
# Args: circuit_id, directory, wasm_name, zkey_prefix, vkey_name
generate_and_verify_v2 10 "poi" "proof_of_innocence" "poi" "poi"
generate_and_verify_v2 20 "vote" "private_vote" "vote" "vote"
generate_and_verify_v2 21 "vote" "private_vote_poi" "vote_poi" "vote_poi"
generate_and_verify_v2 30 "market" "market_bet" "market_bet" "market_bet"
generate_and_verify_v2 31 "market" "market_bet_poi" "market_bet_poi" "market_bet_poi"
generate_and_verify_v2 32 "market" "market_claim" "market_claim" "market_claim"
generate_and_verify_v2 40 "portfolio" "compliance" "compliance" "compliance"
generate_and_verify_v2 41 "portfolio" "net_worth" "net_worth" "net_worth"

echo -e "\n=========================================="
echo "Results: $PASSED passed, $FAILED failed"
echo "=========================================="

if [ $FAILED -gt 0 ]; then
    exit 1
fi

echo "All proofs generated and verified!"
