#!/bin/bash
set -e

VK_DIR="/home/deploy/2025q4/13-area1-circuits/verification_keys"

echo "=========================================="
echo "ZyberLink ZK Circuits - VKey Validation"
echo "=========================================="

PASSED=0
FAILED=0

validate_vkey() {
    local circuit_id=$1
    local vk_file="$VK_DIR/circuit_${circuit_id}_vkey.json"

    echo -e "\n[Circuit $circuit_id] Validating VKey"
    echo "-------------------------------------------"

    # Check file exists
    if [ ! -f "$vk_file" ]; then
        echo "  ERROR: VKey file not found: $vk_file"
        ((FAILED++))
        return 1
    fi
    echo "  File exists: OK"

    # Validate JSON format
    if ! jq empty "$vk_file" 2>/dev/null; then
        echo "  ERROR: Invalid JSON format"
        ((FAILED++))
        return 1
    fi
    echo "  Valid JSON: OK"

    # Check required fields
    local required_fields=("protocol" "curve" "nPublic" "vk_alpha_1" "vk_beta_2" "vk_gamma_2" "vk_delta_2" "IC")

    for field in "${required_fields[@]}"; do
        if ! jq -e ".$field" "$vk_file" >/dev/null 2>&1; then
            echo "  ERROR: Missing field: $field"
            ((FAILED++))
            return 1
        fi
    done
    echo "  Required fields: OK"

    # Check protocol is groth16
    local protocol=$(jq -r '.protocol' "$vk_file")
    if [ "$protocol" != "groth16" ]; then
        echo "  ERROR: Expected protocol 'groth16', got '$protocol'"
        ((FAILED++))
        return 1
    fi
    echo "  Protocol (groth16): OK"

    # Check curve is bn128
    local curve=$(jq -r '.curve' "$vk_file")
    if [ "$curve" != "bn128" ]; then
        echo "  ERROR: Expected curve 'bn128', got '$curve'"
        ((FAILED++))
        return 1
    fi
    echo "  Curve (bn128): OK"

    # Get number of public inputs
    local n_public=$(jq -r '.nPublic' "$vk_file")
    local ic_length=$(jq '.IC | length' "$vk_file")
    local expected_ic=$((n_public + 1))

    if [ "$ic_length" -ne "$expected_ic" ]; then
        echo "  ERROR: IC length mismatch. Expected $expected_ic, got $ic_length"
        ((FAILED++))
        return 1
    fi
    echo "  IC length ($ic_length for $n_public public inputs): OK"

    echo "  PASSED"
    ((PASSED++))
    return 0
}

# Validate all VKeys
for circuit_id in 10 20 21 30 31 32 40 41; do
    validate_vkey $circuit_id || true
done

echo -e "\n=========================================="
echo "Results: $PASSED passed, $FAILED failed"
echo "=========================================="

if [ $FAILED -gt 0 ]; then
    exit 1
fi

echo "All VKeys validated successfully!"
