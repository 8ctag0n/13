# ZK Circuits Guide

## Overview

ZyberLink utilizes Zero-Knowledge (ZK) circuits to provide verifiable, private computations. These circuits allow users to prove properties of their data (e.g., membership in a list, net worth thresholds) without revealing the data itself.

## Circuit Categories

The protocol supports several specialized ZK circuits located in `zyb-circuits/`:

### 1. Core & Identity (`poi/`)
- **Proof of Innocence (POI)**: Proves a user is NOT present in a specific blacklist (e.g., sanctions list).
- **Membership**: Proves a user belongs to a specific group without revealing their identity.

### 2. Governance (`vote/`)
- **Private Vote**: Ensures vote secrecy while allowing public tally verification.
- **Vote with POI**: Combines identity verification with anonymous voting to prevent sanctioned entities from participating in governance.

### 3. Market & DeFi (`market/`, `blind/`)
- **Market Bet**: Validates that a bet is placed within allowed bounds without revealing the exact strategy.
- **Blind Betting**: Encrypted gambling logic where outcomes are verified via ZK.
- **Private Balance**: Handles encrypted token transfers and balance state updates.

---

## Technical Stack

- **Domain Specific Language**: Circom 2.1
- **Proving System**: Groth16 (via SnarkJS)
- **Curve**: BN128
- **Backend Verification**: `ark-groth16` (Rust)

---

## Development Workflow

### 1. Circuit Design
Circuits are defined in `.circom` files.

```text
template Example() {
    signal input in;
    signal output out;
    out <== in * in;
}
```

### 2. Compilation
Use the provided scripts to compile circuits:

```bash
cd zyb-circuits/poi
./scripts/compile.sh
```

### 3. Trusted Setup
The protocol uses a per-circuit trusted setup (Powers of Tau).

### 4. Proof Generation (Prover)
Provers generate a `.json` proof and a `public.json` inputs file.

```bash
snarkjs groth16 prove circuit.zkey witness.wtns proof.json public.json
```

---

## On-Chain Verification

ZK proofs are verified on Solana via the `zk-generator` program. The protocol compares the proof hash submitted by the prover against the requirements defined in the job.

| Circuit ID | Implementation Status |
|------------|-----------------------|
| 10 (POI)   | ✅ Production Ready    |
| 20 (Vote)  | ✅ Production Ready    |
| 30 (DeFi)  | ⚠️ Beta (Testing)      |
| 40 (Port)  | ⚠️ Alpha (Research)    |
