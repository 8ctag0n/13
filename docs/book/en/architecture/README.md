# Architecture

Learn about ZyberLink's technical architecture and design decisions.

## In This Section

* [System Overview](overview.md) - High-level architecture and components
* [FHE Design](fhe-design.md) - Fully Homomorphic Encryption implementation
* [Tech Stack](tech-stack.md) - Technologies, libraries, and tools

## Key Concepts

### Multi-Prover Consensus

ZyberLink uses a novel consensus mechanism where multiple independent provers execute the same computation and submit results. The system only accepts results when a threshold of provers (e.g., 2-of-3) agree.

### Encrypted Witness

All sensitive data is encrypted before being posted on-chain. Provers execute computations on encrypted data without ever seeing the plaintext.

### On-Chain Verification

The Solana program verifies consensus and automatically distributes payments to honest provers while penalizing dishonest ones.

---

Explore each section to understand how ZyberLink achieves decentralized privacy-preserving computation.
