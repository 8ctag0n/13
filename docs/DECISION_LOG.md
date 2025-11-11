# Decision Log

## Overview

This document records significant architectural and strategic decisions made during the development of CypherLink, including context, alternatives considered, and rationale.

**Format:**
- Date of decision
- Decision made
- Context/problem
- Alternatives considered
- Rationale
- Trade-offs
- Status (active, superseded, deprecated)

---

## November 10, 2025

### Decision: Use Solana Bare Metal (No Anchor)

**Context:**
Need to choose framework for smart contract development. Anchor is the most popular framework but adds overhead.

**Alternatives:**
1. **Anchor Framework** - Macro-based, high-level abstraction
2. **Solana bare metal** - Direct use of solana_program crate
3. **Seahorse** - Python-based, compiles to Anchor

**Decision: Solana bare metal**

**Rationale:**
- Full control over program size (critical for optimization)
- Direct access to Light Protocol primitives
- Reduced attack surface (no macro magic)
- Better understanding of underlying mechanisms
- Smaller binary size (cheaper deployment)

**Trade-offs:**
- ✅ Pros: Full control, better performance, smaller size
- ❌ Cons: More verbose code, manual account validation, steeper learning curve

**Status:** Active

---

### Decision: Light Protocol for ZK Compression

**Context:**
Storing job data on-chain is expensive. Need cost-effective solution for marketplace state.

**Alternatives:**
1. **Standard Solana accounts** - Simple but expensive (~$0.77 per job)
2. **Light Protocol ZK Compression** - Off-chain state with on-chain commitments
3. **Geyser plugin** - Custom indexer, completely off-chain
4. **External database** - AWS/GCP, centralized

**Decision: Light Protocol**

**Rationale:**
- 50x cost reduction (32 bytes on-chain vs 5 KB)
- Still decentralized (ZK proofs ensure validity)
- Composable with Solana ecosystem
- Production-ready (used by several projects)
- Better than centralized database (maintains decentralization)

**Trade-offs:**
- ✅ Pros: Massive cost savings, decentralized, proven tech
- ❌ Cons: Dependency on Light indexer, slightly more complex reads

**Impact:** Reduces operational costs from ~$770K to ~$15K for 1M jobs

**Status:** Active

---

### Decision: Wallet Demo (Not Generic Marketplace)

**Context:**
Need to choose primary demo for hackathon. Could showcase marketplace infrastructure or concrete use case.

**Alternatives:**
1. **Generic marketplace UI** - Shows jobs, provers, stats
2. **Wallet demo** - Zcash mobile wallet as use case
3. **Multiple small demos** - Wallet + voting + one more
4. **SDK only** - Developer tools, let others build apps

**Decision: Wallet demo (Zcash)**

**Rationale:**
- Concrete, measurable impact (10x faster)
- Universal understanding (everyone uses wallets)
- Emotional impact (visceral side-by-side comparison)
- Market validation (300K Zcash users want this)
- Judge alignment (Zooko is a judge)
- Demonstrates complete E2E flow

**Trade-offs:**
- ✅ Pros: Clear value prop, measurable, emotional impact
- ❌ Cons: Doesn't show full platform potential initially

**Reference:** See APPROACH.md for detailed rationale

**Status:** Active

---

### Decision: Post-Quantum Encryption (ML-KEM)

**Context:**
Need to encrypt witness data for prover. Standard ECDH is not quantum-resistant.

**Alternatives:**
1. **ECDH (X25519)** - Standard, fast, but not quantum-safe
2. **ML-KEM (Kyber)** - Post-quantum KEM, NIST standardized
3. **Frodo** - Alternative PQ scheme, larger keys
4. **NTRU** - Another PQ option, less standardized

**Decision: ML-KEM-768 (Kyber)**

**Rationale:**
- NIST standardized (FIPS 203, August 2024)
- Future-proof against quantum attacks
- Reasonable key sizes (1,184 bytes public key)
- Fast performance (~0.5ms operations)
- Excellent Rust library support
- "Harvest now, decrypt later" attack mitigation

**Trade-offs:**
- ✅ Pros: Quantum-safe, standardized, future-proof
- ❌ Cons: Larger keys than ECDH, newer (less battle-tested)

**Security level:** 192 bits post-quantum (equivalent to AES-192)

**Status:** Active

---

### Decision: Solana Attestation Service for Reputation

**Context:**
Need reputation system to prevent bad provers. Could build custom or use existing infrastructure.

**Alternatives:**
1. **Custom reputation contract** - Full control, more work
2. **SAS (Solana Attestation Service)** - Decentralized attestation protocol
3. **Off-chain reputation** - Database, centralized
4. **No reputation** - Pure price competition

**Decision: SAS integration**

**Rationale:**
- Decentralized (fits our ethos)
- Composable (other protocols can use same attestations)
- Standard schema (CypherLinkProof attestation type)
- Less development work than custom
- On-chain queryable
- Prevents race to bottom on quality

**Trade-offs:**
- ✅ Pros: Decentralized, composable, less work
- ❌ Cons: Dependency on SAS, schema constraints

**Alternative considered:** Initially considered custom contract but chose SAS for composability

**Status:** Active

---

### Decision: Flutter for Mobile (Not React Native)

**Context:**
Need cross-platform mobile framework. Must support iOS and Android with native performance.

**Alternatives:**
1. **React Native** - JavaScript-based, popular
2. **Flutter** - Dart-based, Google-backed
3. **Native (Swift/Kotlin)** - Platform-specific
4. **Xamarin** - C#-based, Microsoft

**Decision: Flutter**

**Rationale:**
- Better FFI support (flutter_rust_bridge is excellent)
- Native performance (compiled, not interpreted)
- Single codebase for iOS + Android
- Rich UI components
- Fast hot-reload (developer productivity)
- Strong Rust integration story

**Trade-offs:**
- ✅ Pros: Great FFI, native perf, single codebase
- ❌ Cons: Larger app size (~15 MB), some platform-specific code

**Comparison:**
- vs React Native: Better performance, better FFI
- vs Native: 50% less development time
- vs Xamarin: Better ecosystem, more modern

**Status:** Active

---

### Decision: Halo2 (Zcash Orchard) Circuit

**Context:**
Need ZK circuit for shielded transactions. Could use existing or build custom.

**Alternatives:**
1. **Halo2 (Zcash Orchard)** - Production Zcash circuit
2. **Circom/SnarkJS** - Custom circuit in Circom
3. **Arkworks** - Lower-level Rust library
4. **Plonky2** - Different proof system

**Decision: Halo2 Orchard**

**Rationale:**
- Production-ready (powers Zcash mainnet)
- Audited and battle-tested
- Optimized performance
- No need to reinvent wheel
- Security confidence (used by $2B+ network)
- Zcash alignment (judge and partnership opportunities)

**Trade-offs:**
- ✅ Pros: Battle-tested, audited, optimized
- ❌ Cons: Large proving keys (~100 MB), memory intensive

**Alternative:** Considered custom circuit but security risk too high

**Status:** Active

---

## November 11, 2025

### Decision: 21-Day Development Timeline

**Context:**
Hackathon runs Nov 10 - Dec 1 (21 days). Need to scope appropriately.

**Alternatives:**
1. **Full platform** - Marketplace + wallet + voting + credentials
2. **MVP wallet** - Just enough to demo concept
3. **SDK only** - Infrastructure, no UI

**Decision: MVP wallet with working marketplace**

**Rationale:**
- Focused scope (one thing done well)
- Achievable in 21 days
- Demonstrates complete flow
- Production-quality code (not hackathon spaghetti)
- Strong foundation for post-hackathon work

**Scope:**
- Week 1: Core infrastructure (program, SDK, prover MVP)
- Week 2: Real proving + wallet
- Week 3: Polish + demo prep

**Trade-offs:**
- ✅ Pros: Achievable, high quality, clear demo
- ❌ Cons: Doesn't show full vision (but roadmap does)

**Reference:** See ROADMAP.md for detailed timeline

**Status:** Active

---

### Decision: No Multi-Sig or Complex Access Control (MVP)

**Context:**
Could implement multi-sig wallets, delegated access, etc. but adds complexity.

**Alternatives:**
1. **Single-sig only** - One key, one wallet
2. **Multi-sig** - M-of-N signatures required
3. **Delegated access** - Hot wallet + cold storage

**Decision: Single-sig only for MVP**

**Rationale:**
- MVP scope management
- Reduces attack surface
- Simpler UX
- Can add later without breaking changes

**Post-hackathon:**
- Month 2-3: Add multi-device sync
- Month 4-5: Add multi-sig support
- Month 6+: Advanced access control

**Trade-offs:**
- ✅ Pros: Simpler, faster to build, easier to audit
- ❌ Cons: Less flexible for advanced users

**Status:** Active (will revisit post-hackathon)

---

### Decision: Testnet/Devnet First, Mainnet Later

**Context:**
Could deploy directly to mainnet or start with testnet.

**Alternatives:**
1. **Mainnet immediately** - Real money, real stakes
2. **Devnet for hackathon** - Fake money, safer testing
3. **Testnet for beta** - Middle ground

**Decision: Devnet for hackathon, testnet for beta, mainnet post-audit**

**Rationale:**
- Devnet: Fast iteration, no real funds at risk
- Testnet: More stable, better for user testing
- Mainnet: After security audit and extensive testing

**Timeline:**
- Nov 10-30: Devnet
- Dec 1-31: Testnet beta
- Jan 1+: Mainnet (post-audit)

**Trade-offs:**
- ✅ Pros: Safe testing, iterative improvement
- ❌ Cons: Delayed real usage

**Status:** Active

---

## November 12, 2025

### Decision: Prover Selection Algorithm

**Context:**
When multiple provers available, need to choose one. Could be random, reputation-based, or price-based.

**Alternatives:**
1. **Random selection** - Fair but no quality incentive
2. **Highest reputation** - Quality but may create monopoly
3. **Lowest price** - Cheap but race to bottom
4. **Weighted (reputation + price)** - Balanced

**Decision: Weighted selection with user override**

**Rationale:**
- Default: Weighted by (reputation * 0.7) + (price * 0.3)
- Advanced: User can manually select prover
- Balances quality and cost
- Prevents monopoly (randomness factor)
- Encourages high-quality provers

**Formula:**
```rust
score = (reputation / 1000) * 0.7 + (min_price / price) * 0.3
```

**Trade-offs:**
- ✅ Pros: Balanced, prevents extremes
- ❌ Cons: More complex than pure random

**Status:** Active

---

### Decision: Fallback to Local Proving

**Context:**
What happens if no provers available? System must gracefully degrade.

**Alternatives:**
1. **Fail** - Transaction rejected
2. **Queue** - Wait indefinitely for prover
3. **Fallback to local** - Slow but works

**Decision: Fallback to local proving with warning**

**Rationale:**
- User can always complete transaction
- Proves system isn't brittle
- Better UX than failure
- Shows local vs offload comparison

**UX:**
```
⚠️ No provers available
Falling back to local proving (slower)
Expected time: ~2 minutes

[Continue] [Cancel]
```

**Trade-offs:**
- ✅ Pros: Always works, better UX
- ❌ Cons: User may not get speed benefit

**Status:** Active

---

### Decision: Fixed Pricing (No Dynamic Pricing for MVP)

**Context:**
Provers could set prices dynamically or we enforce fixed pricing.

**Alternatives:**
1. **Fixed platform pricing** - $0.02 per proof
2. **Prover sets price** - Market-driven
3. **Dynamic/surge pricing** - Like Uber

**Decision: Fixed pricing for MVP ($0.02), enable market pricing post-launch**

**Rationale:**
- Simplifies MVP implementation
- Predictable costs for users
- Easier to demo (no price fluctuations)
- Can enable dynamic pricing later

**Post-hackathon:**
- Month 2-3: Allow provers to set prices
- Month 4+: Market-based pricing with reputation tiers

**Trade-offs:**
- ✅ Pros: Simple, predictable
- ❌ Cons: Not optimized for supply/demand

**Status:** Active (temporary, will evolve)

---

## November 13, 2025

### Decision: Job Timeout Strategy

**Context:**
If prover claims job but doesn't complete, need timeout mechanism.

**Alternatives:**
1. **No timeout** - Job stuck forever
2. **Fixed timeout** - e.g., 5 minutes
3. **Dynamic timeout** - Based on circuit complexity
4. **Prover-specified** - Prover commits to time

**Decision: Fixed 5-minute timeout for MVP**

**Rationale:**
- Orchard proofs should complete in 10-15s
- 5 minutes gives 20x buffer
- Simple to implement and understand
- Can make dynamic later

**Behavior on timeout:**
- Job status → Failed
- Escrow returned to creator
- Prover reputation slashed (-5%)

**Trade-offs:**
- ✅ Pros: Simple, protects users
- ❌ Cons: May punish slow provers unfairly

**Future:** Dynamic timeouts based on circuit type and prover hardware

**Status:** Active

---

### Decision: Escrow Smart Contract (Not External Service)

**Context:**
Need to hold user funds until proof delivered. Could use program escrow or external service.

**Alternatives:**
1. **Program escrow** - PDA holds funds
2. **External service** - Custodial escrow
3. **No escrow** - Prepaid provers

**Decision: Program escrow via PDAs**

**Rationale:**
- Trustless (no third party)
- Automatic release on proof submission
- Standard Solana pattern
- Fully on-chain and transparent

**Implementation:**
```rust
#[account]
pub struct Escrow {
    pub job_id: u64,
    pub creator: Pubkey,
    pub amount: u64,
    pub released: bool,
}
```

**Trade-offs:**
- ✅ Pros: Trustless, transparent, automatic
- ❌ Cons: Requires rent-exempt balance

**Status:** Active

---

### Decision: Slashing Percentage Tiers

**Context:**
Need to define slashing amounts for different infractions.

**Alternatives:**
1. **Fixed amount** - e.g., 1 SOL
2. **Percentage of stake** - e.g., 10%
3. **Tiered by severity** - Different penalties

**Decision: Tiered slashing**

**Slashing schedule:**
- **Invalid proof:** 10% of stake
- **Timeout (first offense):** 5% of stake
- **Timeout (repeat offender):** 15% of stake
- **Malicious behavior:** 100% of stake (full slash)

**Rationale:**
- Proportional punishment
- Distinguishes between honest mistakes and malice
- Escalates for repeat offenders
- Severe enough to deter bad behavior

**Trade-offs:**
- ✅ Pros: Fair, proportional, clear incentives
- ❌ Cons: Requires tracking offense history

**Status:** Active

---

## November 14, 2025

### Decision: Monorepo Structure

**Context:**
Multiple components (program, SDK, wallet, prover). Could be separate repos or monorepo.

**Alternatives:**
1. **Monorepo** - All in one repository
2. **Multi-repo** - Separate repos for each component
3. **Hybrid** - Core in one, apps separate

**Decision: Monorepo**

**Rationale:**
- Easier to keep in sync
- Shared types and utilities
- Single version number
- Simpler CI/CD
- Better for hackathon (judges see everything)

**Structure:**
```
zyberlink/
├── programs/       # Solana programs
├── sdk/            # Rust client SDK
├── prover-node/    # Desktop prover
├── cypherlink-wallet/  # Mobile wallet
├── shared/         # Shared types
└── tests/          # Integration tests
```

**Trade-offs:**
- ✅ Pros: Easier sync, shared code, simpler versioning
- ❌ Cons: Larger repo, potential conflicts

**Status:** Active

---

### Decision: Rust FFI (Not WASM) for Mobile

**Context:**
Flutter needs to call Rust code. Could use FFI (C bindings) or WASM.

**Alternatives:**
1. **FFI (flutter_rust_bridge)** - Native bindings
2. **WASM** - Run Rust in WebAssembly
3. **Platform channels** - Manual bindings

**Decision: FFI via flutter_rust_bridge**

**Rationale:**
- Native performance (no WASM overhead)
- Type-safe bindings (auto-generated)
- Async support
- Better cryptography library access (can't use some in WASM)
- Excellent developer experience

**Trade-offs:**
- ✅ Pros: Native speed, type-safe, async support
- ❌ Cons: Platform-specific builds (longer compile time)

**Alternative:** WASM considered but ruled out due to crypto library limitations

**Status:** Active

---

## November 15, 2025

### Decision: No Token (SOL for Payments)

**Context:**
Could create custom token (CPL, CYPHER, etc.) or use SOL for payments.

**Alternatives:**
1. **Custom token** - Own tokenomics, utility
2. **SOL** - Native Solana token
3. **Stablecoin** - USDC for predictable pricing

**Decision: SOL for MVP, consider stablecoin for production**

**Rationale:**
- No token launch complexity
- Simpler for users (already have SOL)
- Avoid regulatory scrutiny
- Faster to market
- Can add token later if needed

**Post-hackathon consideration:**
- USDC for price stability (provers want predictable income)
- Custom token only if clear utility beyond payment

**Trade-offs:**
- ✅ Pros: Simple, fast, no regulation risk
- ❌ Cons: No token upside, price volatility (SOL fluctuates)

**Status:** Active (may revisit for production)

---

### Decision: Open Source (MIT OR Apache-2.0)

**Context:**
License choice affects adoption, contributions, and business model.

**Alternatives:**
1. **Proprietary** - Closed source
2. **GPL** - Copyleft
3. **MIT/Apache** - Permissive
4. **Business Source License** - Time-delayed open source

**Decision: MIT OR Apache-2.0 (dual license)**

**Rationale:**
- Cypherpunk ethos (open source)
- Easier adoption (permissive license)
- Attracts contributors
- Grant-friendly (Zcash, Solana foundations prefer open source)
- Can still build commercial services on top

**Trade-offs:**
- ✅ Pros: Community trust, grant-friendly, contributors
- ❌ Cons: Anyone can fork (but network effects protect us)

**Status:** Active

---

## November 16, 2025

### Decision: No Account Abstraction (MVP)

**Context:**
Could implement account abstraction (gasless txs, social recovery) but adds complexity.

**Alternatives:**
1. **Standard accounts** - User pays gas
2. **Account abstraction** - Sponsored transactions, social recovery
3. **Hybrid** - Optional sponsorship

**Decision: Standard accounts for MVP**

**Rationale:**
- Solana gas is already cheap (~$0.00025)
- Account abstraction is complex (new programs, relayers)
- Not critical for MVP demo
- Can add later as protocol matures

**Post-hackathon:**
- Month 6+: Consider sponsorship for user onboarding
- Year 2+: Full account abstraction if needed

**Trade-offs:**
- ✅ Pros: Simpler, faster to build
- ❌ Cons: Less user-friendly onboarding

**Status:** Active (revisit post-hackathon)

---

## Superseded Decisions

### ~~Anchor Framework~~ → Bare Metal Solana
**Date:** November 10, 2025
**Reason:** Better control, smaller binaries, Light Protocol integration
**See:** Decision above on Solana bare metal

---

## Deprecated Decisions

None yet.

---

## Pending Decisions

### To Be Decided: Multi-Chain Support

**Context:** Should we support Ethereum, Polygon, other L1s?

**Timeline:** Post-hackathon (Month 6+)

**Options:**
1. Solana-only (simplest)
2. Add Ethereum L2s (bigger market)
3. Multi-chain from start (complex)

**Leaning toward:** Solana-only for first year, evaluate later

---

### To Be Decided: Circuit Marketplace

**Context:** Should provers advertise which circuits they support?

**Timeline:** Month 4-6

**Options:**
1. Fixed circuits (Orchard, voting, credentials)
2. Marketplace where provers list capabilities
3. Custom circuit upload

**Leaning toward:** Fixed circuits for first 6 months, then marketplace

---

### To Be Decided: Hardware Acceleration

**Context:** Should we support FPGA/ASIC provers?

**Timeline:** Year 2+

**Options:**
1. CPU-only (current)
2. GPU support (easier)
3. FPGA/ASIC (fastest but complex)

**Leaning toward:** GPU support in Year 2, FPGA later if demand

---

## Decision-Making Process

**For architectural decisions:**
1. Identify problem and context
2. List alternatives (minimum 3)
3. Evaluate trade-offs
4. Consult team (if applicable)
5. Document decision here
6. Implement

**For strategic decisions:**
1. Same as above, plus:
2. Consider market impact
3. Evaluate competitive positioning
4. Assess resource requirements
5. Define success metrics

**Revisiting decisions:**
- Quarterly review of active decisions
- Update if context changes
- Mark as superseded if replaced
- Document lessons learned

---

## Lessons Learned

**Good decisions:**
- Light Protocol adoption (massive cost savings)
- Wallet-first approach (clear demo)
- Bare metal Solana (full control)
- Post-quantum crypto (future-proof)

**Decisions we'd reconsider:**
- TBD (will update as project matures)

**Decisions that surprised us:**
- TBD (will update during implementation)

---

## References

- [Approach Document](./APPROACH.md) - Wallet demo rationale
- [Roadmap](./ROADMAP.md) - Timeline and milestones
- [Tech Stack](./TECH_STACK.md) - Technology choices
- [Architecture](./ARCHITECTURE.md) - System design

---

This log will be updated as new decisions are made and context evolves.
