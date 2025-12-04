# ZyberLink Roadmap

## Vision

ZyberLink is building the decentralized infrastructure for privacy-preserving computation. Our goal is to enable anyone to perform computations on encrypted data without trusting a single party.

---

## Season Structure

Development is organized in seasons, each with a specific focus:

```
S0   → Solana SZN         (Foundation)
S0.5 → Ops SZN            (Decentralization)
S1   → Interop SZN        (Expansion)
S1.5 → 3rd Party SZN      (Adoption)
S2   → Consumer SZN       (Mass Market)
```

---

## S0: Solana SZN

**Status:** Active
**Focus:** Establish foundation on Solana, optimize core infrastructure

### Completed (v0.0.1-zypherpunk)

**Solana Program**
- Job lifecycle (Create, Claim, Submit, Verify)
- Multi-prover consensus (2-of-3, 3-of-5)
- Automated payment distribution
- Dynamic pricing system
- Prover registration & staking

**FHE Engine**
- TFHE-rs integration
- Operations: Sum, Average, CountIf, Threshold, Histogram, Add, Multiply
- Server key caching
- Result hashing for consensus

**Prover Network**
- Job polling & auto-claiming
- Halo2 ZK proofs (~15s)
- FHE computation execution
- Multi-prover validation

**Backend & API**
- REST API (Rust/Axum)
- PostgreSQL persistence
- Blockchain sync (chain_sync)
- Smart witness cleanup

**SDK & Tools**
- Rust client library
- fhe-cli for local encryption
- MarketplaceClient

**Testing**
- 55/56 tests passing (98%)
- E2E: PoI CountIf, Census Sum

### In Progress

- Performance optimizations (reduce FHE time)
- Additional operations (comparison, conditional)
- Prover reputation system (basic)
- UI/UX improvements

### Targets

- [ ] Win/place in Solana hackathons
- [ ] 5+ independent provers running
- [ ] 100+ jobs executed on testnet
- [ ] FHE compute time < 3 min

---

## S0.5: Ops SZN

**Status:** Planned
**Focus:** Operational improvements, decentralization of infrastructure

### Goals

**Data Decentralization**
- Witness storage on decentralized network (IPFS/Arweave)
- Remove single backend dependency
- Prover-to-prover data sharing

**Network Resilience**
- Multiple backend instances
- Load balancing
- Failover mechanisms
- Geographic distribution

**Monitoring & Observability**
- Prover health dashboards
- Job success rate metrics
- Network latency tracking
- Alert systems

**Security Hardening**
- Full security audit
- Formal verification of critical paths
- Bug bounty program
- Rate limiting & DDoS protection

### Targets

- [ ] Zero single points of failure
- [ ] 99.9% uptime SLA capability
- [ ] Decentralized witness storage
- [ ] Security audit completed

---

## S1: Interop SZN

**Status:** Planned
**Focus:** Multi-chain expansion, developer experience, zero-barrier UX

### Goals

**Cross-Chain Support**
- Ethereum bridge (Wormhole/Axelar)
- Starknet integration
- Zcash shielded pool bridge
- Chain-agnostic job submission

**SDK v2**
- TypeScript bindings
- Python bindings
- Improved documentation
- Interactive tutorials
- Example applications

**Zero-Barrier UX (x402)**
- Email-to-wallet onboarding
- Fiat onramp integration
- Gasless transactions
- Mobile-first experience

**Wallet Extension**
- Multi-chain support (SOL/ETH/STRK/ZEC)
- Browser extension
- Mobile SDK

### Targets

- [ ] 3+ chains integrated
- [ ] SDK downloads: 100+/week
- [ ] 10+ external developers building
- [ ] x402 integration live

---

## S1.5: 3rd Party SZN

**Status:** Future
**Focus:** Enable third-party adoption and integrations

### Goals

**Developer Ecosystem**
- Plugin architecture
- Marketplace for custom operations
- Revenue sharing for operation creators
- Developer grants program

**Enterprise Features**
- SLA-backed compute guarantees
- White-label solutions
- Compliance integrations (KYC/AML ready)
- Enterprise API tier

**Partnerships**
- DeFi protocol integrations
- Data provider partnerships
- Wallet partnerships
- Chain partnerships

### Targets

- [ ] 5+ third-party integrations live
- [ ] Enterprise pilot customers
- [ ] Partner-built operations in marketplace

---

## S2: Consumer SZN

**Status:** Vision
**Focus:** Consumer-facing applications

The details of S2 will be defined based on learnings from previous seasons. Potential directions include:

- Privacy-preserving consumer apps
- B2C products built on ZyberLink infrastructure
- Mobile-first experiences
- Social/community features

More details will be shared as we approach this phase.

---

## Quick Start

```bash
# Full setup
make c1    # Start containers
make c2    # Init marketplace + register provers
make c3    # Start provers

# Run tests
make e2e-poi   # Proof of Innocence test
make e2e-sum   # Census Sum test

# Stop
make c0    # Stop all
```

---

## Links

- [GitHub](https://github.com/8ctag0n/13/)
- [Tag: v0.0.1-zypherpunk](https://github.com/8ctag0n/13/releases/tag/v0.0.1-zypherpunk)

---

## Changelog

- **2025-12-03:** Restructured roadmap with season-based strategy
- **2025-12-03:** Tagged v0.0.1-zypherpunk for Zypherpunk Hackathon
- **2025-11-25:** Simplified roadmap, updated test counts
- **2025-11-10:** Project started for Zypherpunk Hackathon
