#!/bin/bash
# Script automatizado para validar los 4 circuitos prioritarios
# FASE 0 - ZK Proof Validation

set -e

CIRCUITS_DIR="/home/deploy/2025q4/13-area2-provers/circuits"
VALIDATION_DIR="$CIRCUITS_DIR/zk-validation"
INPUTS_DIR="$VALIDATION_DIR/inputs"
OUTPUTS_DIR="$VALIDATION_DIR/outputs"
LOGS_DIR="$VALIDATION_DIR/logs"

echo "=== FASE 0: ZK Proof Validation ==="
echo ""

# Circuit 50: Simple (Multiplier)
echo "[1/4] Validating Circuit 50 (Simple)..."
npx snarkjs wtns calculate \
  $CIRCUITS_DIR/simple/circuit_js/circuit.wasm \
  $INPUTS_DIR/circuit_50_input.json \
  $OUTPUTS_DIR/circuit_50.wtns \
  > $LOGS_DIR/circuit_50_witness.log 2>&1

npx snarkjs g16p \
  $CIRCUITS_DIR/simple/simple_final.zkey \
  $OUTPUTS_DIR/circuit_50.wtns \
  $OUTPUTS_DIR/circuit_50_proof.json \
  $OUTPUTS_DIR/circuit_50_public.json \
  > $LOGS_DIR/circuit_50_prove.log 2>&1

npx snarkjs zkev \
  $CIRCUITS_DIR/simple/simple_final.zkey \
  $OUTPUTS_DIR/circuit_50_vkey.json \
  > $LOGS_DIR/circuit_50_vkey.log 2>&1

npx snarkjs g16v \
  $OUTPUTS_DIR/circuit_50_vkey.json \
  $OUTPUTS_DIR/circuit_50_public.json \
  $OUTPUTS_DIR/circuit_50_proof.json \
  > $LOGS_DIR/circuit_50_verify.log 2>&1

if grep -q "OK" $LOGS_DIR/circuit_50_verify.log; then
  echo "✅ Circuit 50 (Simple): VALID"
else
  echo "❌ Circuit 50 (Simple): FAILED"
  exit 1
fi

# Circuit 10: PoI
echo "[2/4] Validating Circuit 10 (PoI)..."
npx snarkjs wtns calculate \
  $CIRCUITS_DIR/poi/proof_of_innocence_js/proof_of_innocence.wasm \
  $INPUTS_DIR/circuit_10_input.json \
  $OUTPUTS_DIR/circuit_10.wtns \
  > $LOGS_DIR/circuit_10_witness.log 2>&1

npx snarkjs g16p \
  $CIRCUITS_DIR/poi/poi_final.zkey \
  $OUTPUTS_DIR/circuit_10.wtns \
  $OUTPUTS_DIR/circuit_10_proof.json \
  $OUTPUTS_DIR/circuit_10_public.json \
  > $LOGS_DIR/circuit_10_prove.log 2>&1

npx snarkjs zkev \
  $CIRCUITS_DIR/poi/poi_final.zkey \
  $OUTPUTS_DIR/circuit_10_vkey.json \
  > $LOGS_DIR/circuit_10_vkey.log 2>&1

npx snarkjs g16v \
  $OUTPUTS_DIR/circuit_10_vkey.json \
  $OUTPUTS_DIR/circuit_10_public.json \
  $OUTPUTS_DIR/circuit_10_proof.json \
  > $LOGS_DIR/circuit_10_verify.log 2>&1

if grep -q "OK" $LOGS_DIR/circuit_10_verify.log; then
  echo "✅ Circuit 10 (PoI): VALID"
else
  echo "❌ Circuit 10 (PoI): FAILED"
  exit 1
fi

# Circuit 20: Vote
echo "[3/4] Validating Circuit 20 (Vote)..."
npx snarkjs wtns calculate \
  $CIRCUITS_DIR/vote/private_vote_js/private_vote.wasm \
  $INPUTS_DIR/circuit_20_input.json \
  $OUTPUTS_DIR/circuit_20.wtns \
  > $LOGS_DIR/circuit_20_witness.log 2>&1

npx snarkjs g16p \
  $CIRCUITS_DIR/vote/vote_final.zkey \
  $OUTPUTS_DIR/circuit_20.wtns \
  $OUTPUTS_DIR/circuit_20_proof.json \
  $OUTPUTS_DIR/circuit_20_public.json \
  > $LOGS_DIR/circuit_20_prove.log 2>&1

npx snarkjs zkev \
  $CIRCUITS_DIR/vote/vote_final.zkey \
  $OUTPUTS_DIR/circuit_20_vkey.json \
  > $LOGS_DIR/circuit_20_vkey.log 2>&1

npx snarkjs g16v \
  $OUTPUTS_DIR/circuit_20_vkey.json \
  $OUTPUTS_DIR/circuit_20_public.json \
  $OUTPUTS_DIR/circuit_20_proof.json \
  > $LOGS_DIR/circuit_20_verify.log 2>&1

if grep -q "OK" $LOGS_DIR/circuit_20_verify.log; then
  echo "✅ Circuit 20 (Vote): VALID"
else
  echo "❌ Circuit 20 (Vote): FAILED"
  exit 1
fi

# Circuit 30: Market
echo "[4/4] Validating Circuit 30 (Market)..."
npx snarkjs wtns calculate \
  $CIRCUITS_DIR/market/market_bet_js/market_bet.wasm \
  $INPUTS_DIR/circuit_30_input.json \
  $OUTPUTS_DIR/circuit_30.wtns \
  > $LOGS_DIR/circuit_30_witness.log 2>&1

npx snarkjs g16p \
  $CIRCUITS_DIR/market/market_bet_final.zkey \
  $OUTPUTS_DIR/circuit_30.wtns \
  $OUTPUTS_DIR/circuit_30_proof.json \
  $OUTPUTS_DIR/circuit_30_public.json \
  > $LOGS_DIR/circuit_30_prove.log 2>&1

npx snarkjs zkev \
  $CIRCUITS_DIR/market/market_bet_final.zkey \
  $OUTPUTS_DIR/circuit_30_vkey.json \
  > $LOGS_DIR/circuit_30_vkey.log 2>&1

npx snarkjs g16v \
  $OUTPUTS_DIR/circuit_30_vkey.json \
  $OUTPUTS_DIR/circuit_30_public.json \
  $OUTPUTS_DIR/circuit_30_proof.json \
  > $LOGS_DIR/circuit_30_verify.log 2>&1

if grep -q "OK" $LOGS_DIR/circuit_30_verify.log; then
  echo "✅ Circuit 30 (Market): VALID"
else
  echo "❌ Circuit 30 (Market): FAILED"
  exit 1
fi

echo ""
echo "=== FASE 0 COMPLETED SUCCESSFULLY ==="
echo "4/4 circuits validated ✅"
echo ""
echo "Results:"
echo "  - Circuit 50 (Simple): VALID"
echo "  - Circuit 10 (PoI): VALID"
echo "  - Circuit 20 (Vote): VALID"
echo "  - Circuit 30 (Market): VALID"
echo ""
echo "Outputs saved to: $OUTPUTS_DIR"
echo "Logs saved to: $LOGS_DIR"
