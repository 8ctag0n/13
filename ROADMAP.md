# ZyberLink Roadmap

## Overview

ZyberLink is a decentralized ZK and FHE compute marketplace on Solana. This roadmap reflects the current state of development for the Zypherpunk Hackathon (Nov 10 - Dec 1, 2025).

---

## Done

### Solana Program (Marketplace)
- Job lifecycle management (Create, Claim, Submit, Verify)
- Multi-prover job claiming for FHE computations
- On-chain consensus algorithm (hash-based result verification)
- Automated payment distribution to matching provers
- Prover registration and stake management
- Dynamic pricing system

### SDK (Rust Client Library)
- 3-layer architecture (state, instructions, helpers)
- MarketplaceClient for program interactions
- Job creation and querying
- Event parsing and monitoring

### FHE Computation Engine
- TFHE-rs integration for encrypted computations
- Support for Add, Multiply, Subtract operations
- Server key caching and management
- Result hashing for consensus

### Prover Node Infrastructure
- Job polling and automatic claiming
- Halo2 ZK proof generation (~15s average)
- FHE computation execution
- Multi-prover consensus validation

### Backend API (blink-server)
- REST API for job management
- PostgreSQL persistence
- Blockchain synchronization
- Health monitoring endpoints

### Frontend (webapp)
- Svelte-based SPA
- Wallet connection (Phantom/Solflare)
- Job creation interface
- Real-time job status

### Testing & Infrastructure
- E2E test suite: 55/56 tests passing (98%)
- Backend API: 100% coverage
- Security audit: 6/6 checks passing
- Docker infrastructure (dev + prod)
- Make automation (localnet-setup, run, stop)

### Project Organization
- Reorganized to `src/`, `tests/`, `docs/`, `infra/`, `scripts/`
- GitBook documentation structure
- Comprehensive Makefile with aliases (l1-l4)

---

## In Progress

### Documentation
- Deployment guides
- Architecture documentation
- Demo reproduction guides

### UI Polish
- Fix remaining UI test (1 failing)
- Improve error handling in frontend
- Better loading states

---

## Post-Hackathon Ideas

These are ideas for future development, not committed features:

### Near-term
- Light Protocol integration (ZK compression for cheaper state)
- Reputation system for provers
- More FHE operations (comparison, division)

### Medium-term
- GPU acceleration for faster proofs
- Mobile wallet SDK
- Dynamic pricing based on demand

### Long-term Vision
- Multi-chain support
- DAO governance
- Enterprise API with SLAs

---

## Quick Start

```bash
# Full setup from scratch
make localnet-setup    # or: make l1
make localnet-init     # or: make l2
make localnet-run      # or: make l3

# Optional: auto-generate test jobs
make localnet-jobs     # or: make l4

# Stop everything
make localnet-stop     # or: make l0
```

---

## Changelog

- **2025-11-25:** Simplified roadmap, updated test counts (55/56), documented new structure
- **2025-11-16:** Initial roadmap created
- **2025-11-10:** Project started for Zypherpunk Hackathon
