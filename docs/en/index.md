# ZyberLink

**Decentralized Multi-Prover Network for Privacy-Preserving Computations**

<div style="padding:56.25% 0 0 0;position:relative; margin-top: 2rem; margin-bottom: 2rem; border-radius: 8px; overflow: hidden;"><iframe src="https://player.vimeo.com/video/1143423578?badge=0&amp;autopause=0&amp;player_id=0&amp;app_id=58479" frameborder="0" allow="autoplay; fullscreen; picture-in-picture; clipboard-write; encrypted-media; web-share" referrerpolicy="strict-origin-when-cross-origin" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="ZyberLink__Private_Compute"></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

## Vision

ZyberLink is a decentralized marketplace for FHE (Fully Homomorphic Encryption) and ZK (Zero-Knowledge) computations on Solana. We enable privacy-preserving infrastructure for any application by distributing trust across a network of independent provers who compete to execute cryptographic computations.

Our mission is to make privacy-preserving computations accessible, affordable, and censorship-resistant through decentralized multi-prover consensus.

## The Problem

Modern applications require intensive cryptographic computations (FHE, ZK proofs) that are:

- **Too slow on mobile devices** - Minutes of computation, heavy battery drain
- **Centralized when offloaded** - Trust in single servers, censorship risk
- **Expensive to verify** - No cost-effective way to ensure correctness

## Our Solution

ZyberLink solves this through **multi-prover consensus**:

1. **Client** encrypts sensitive data and posts a computation job to Solana
2. **Multiple provers** compete to execute the computation independently
3. **Consensus algorithm** ensures correctness (2-of-3 or 3-of-5 agreement)
4. **Automated payments** reward honest provers, penalize dishonest ones
5. **Decentralization** ensures no single point of failure or censorship

## Key Features

- **Multi-Prover Consensus** - Trust distributed across independent provers
- **FHE & ZK Support** - Fully homomorphic encryption (TFHE-rs) + zero-knowledge circuits
- **Privacy Preserved** - Encrypted witness data, provers never see plaintext
- **Censorship Resistant** - Permissionless network, no central authority
- **On-Chain Verification** - Trustless payment distribution via consensus
- **Post-Quantum Ready** - ML-KEM key exchange for future-proof privacy

## Use Cases

### FHE-Powered Applications
- **Private DeFi** - Encrypted balance swaps without revealing amounts
- **Confidential Voting** - Private DAO governance with verifiable results
- **Privacy Analytics** - Compute on encrypted datasets without decryption
- **Secure Multi-Party Computation** - Distributed computation without trust

### ZK Infrastructure
- **Mobile Wallet Proving** - Offload proof generation from phones to network
- **Proof Aggregation** - Batch multiple proofs efficiently
- **Cross-Chain Privacy** - Bridge ZK proofs across blockchains

## How It Works

```
┌──────────────┐
│    Client    │ (Mobile app, web app, etc.)
└──────┬───────┘
       │ 1. Request Job (via Public API Layer)
       ▼
┌─────────────────────────────────────┐
│        ZyberLink Backend            │
│  [Public API] -> [x402] -> [Blink]  │
│  - Rate Limiting & Auth             │
│  - Payment Gating                   │
│  - Witness Storage & Validation     │
└──────┬──────────────────────────────┘
       │ 2. Create On-Chain Job
       ▼
┌─────────────────────────┐
│   Solana Marketplace    │ (On-chain job coordination)
└────┬────────────────────┘
     │ 3. Job broadcast
     ▼
┌────────────────────────────────────────────┐
│           Prover Network                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Prover A │  │ Prover B │  │ Prover C │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
└───────│─────────────│─────────────│────────┘
        │ 4. Execute  │             │
        │    (async)  │             │
        ▼             ▼             ▼
┌────────────────────────────────────────────┐
│   Consensus + On-Chain Verification        │
│   A: hash_abc  B: hash_abc (OK) C: hash_def (Fail)│
│   -> Pay A & B, penalize C                 │
└────────────────────────────────────────────┘
```

## Technology Stack

- **Blockchain:** Solana (high-performance, low fees)
- **FHE Engine:** TFHE-rs (Zama)
- **ZK Circuits:** Halo2
- **Prover Node:** Rust
- **Encryption:** ML-KEM (post-quantum)
- **UI:** Terminal User Interface (TUI) with ratatui

## Project Status

**Production Alpha** - January 2026

**Current Features:**
- Multi-prover marketplace on Solana (Mainnet & Devnet)
- 3-Layer Microservices Backend (Public API → x402 → Blink)
- FHE computation engine (TFHE-rs)
- On-chain consensus algorithm
- Terminal UI for prover monitoring
- 14/14 FHE E2E tests passing

## Quick Links

- [Getting Started](getting-started/quickstart.md) - Run your first job
- [Architecture](architecture/overview.md) - Technical deep dive
- [Guides](guides/) - Developer guides and tutorials

## Get Involved

ZyberLink is currently in hackathon development. After December 1, 2025, we will open for:

- **Beta Testers** - Run a prover node and earn rewards
- **Integration Partners** - Build privacy-preserving applications
- **Contributors** - Help build the decentralized privacy infrastructure

---

**Built with privacy, powered by decentralization.**

## Next Step

[Getting Started](getting-started/README.md) - Begin your journey with ZyberLink.