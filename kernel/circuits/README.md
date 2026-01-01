# Kernel Circuits

Chain-agnostic cryptographic primitives.

## Library Components

- `lib/bn254.circom` - BN254 elliptic curve operations
- `lib/poseidon_utils.circom` - Poseidon hash utilities
- `lib/pairing.circom` - Pairing operations

## Verification Circuits

- `verify-groth16/` - Groth16 proof verification circuits
  - `circuit.circom` - Main verification circuit
  - `test_*.circom` - Unit tests for primitives

## Usage

These circuits are imported by vertical-specific circuits:
```circom
include "../../kernel/circuits/lib/poseidon_utils.circom";
```
