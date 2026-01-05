# Zyb CLI Guide

The ZyberLink CLI (`zyb`) is the primary tool for interacting with the ZYB protocol from the command line. It supports both FHE (Fully Homomorphic Encryption) and ZK (Zero-Knowledge) operations.

## Installation

### Prerequisites
- Rust 1.75+
- Solana CLI 2.3+

### Build from Source
```bash
cd zyb-cli
cargo build --release
sudo cp target/release/zyb /usr/local/bin/
```

---

## FHE Commands

FHE commands allow you to encrypt data locally and decrypt computation results.

### 1. Encrypt Data
Encrypt values (0-255) for use in private analytics or compliance jobs.

```bash
zyb fhe encrypt --values 10,20,30,40,50 --output ./fhe-output
```

**Output Files:**
- `witness.bin`: Encrypted data + Server key (upload to ZyberLink).
- `client_key.bin`: Your private decryption key (KEEP SECRET).

### 2. Decrypt Result
Decrypt the results returned by provers.

```bash
zyb fhe decrypt --result-path result.bin --client-key-path ./fhe-output/client_key.bin
```

---

## ZK On-Chain Commands

Interact directly with the `zk-generator` program on Solana.

### 1. Create ZK Job
Create a new ZK job directly on the blockchain.

```bash
zyb zk onchain create \
  --circuit-type 10 \
  --witness witness.json \
  --price 0.1 \
  --keypair ~/.config/solana/id.json
```

### 2. Claim ZK Job (Provers)
Claim a pending job for processing.

```bash
zyb zk onchain claim --job-id <JOB_PDA> --keypair prover.json
```

### 3. Submit ZK Proof
Submit the generated proof hash to finalize the job and receive payment.

```bash
zyb zk onchain submit --job-id <JOB_PDA> --proof proof.json --keypair prover.json
```

### 4. Check Status
Query the status of any job on-chain.

```bash
zyb zk onchain status --job-id <JOB_PDA>
```

---

## Circuit Types Reference

| ID | Name | Category | Description |
|----|------|----------|-------------|
| 10 | ProofOfInnocence | Core | Non-membership proof (blacklist check) |
| 20 | PrivateVote | Voting | Anonymous DAO voting |
| 30 | MarketBet | Market | Prediction market betting |
| 40 | PortfolioCompliance | Portfolio | Regulatory compliance checks |

---

## Environment Variables

Configure your environment for seamless CLI usage:

```bash
export SOLANA_RPC_URL="https://api.devnet.solana.com"
export SOLANA_KEYPAIR="~/.config/solana/id.json"
```

```