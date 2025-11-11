# Roadmap

## Overview

**Hackathon Duration:** 21 days (November 10 - December 1, 2025)
**Target:** Working demo + presentation ready for submission

**Post-Hackathon:** 12-month roadmap to production platform

---

## Hackathon Timeline: 21 Days

### Week 1: Core Infrastructure (November 10-16)

**Goal:** Functional marketplace contracts + SDK + basic prover node

#### Day 1-2: Solana Program Foundation
**Target:** State definitions and basic instruction handlers

**Tasks:**
- [ ] Define state structs (Job, Prover, Escrow, MarketplaceConfig)
- [ ] Light SDK integration (compressed account setup)
- [ ] Instruction definitions (CreateJob, RegisterProver, ClaimJob, SubmitProof, VerifyAndPay)
- [ ] Basic processor scaffolding
- [ ] Deploy boilerplate to local validator

**Deliverable:** Compilable program with state management

**Risk:** Light Protocol integration complexity
**Mitigation:** Use Light examples as reference, limit to basic compression

---

#### Day 3-4: Instruction Processors
**Target:** Complete smart contract logic

**Tasks:**
- [ ] Implement CreateJob (witness encryption, escrow)
- [ ] Implement RegisterProver (stake, reputation init)
- [ ] Implement ClaimJob (prover selection, job assignment)
- [ ] Implement SubmitProof (proof storage, verification trigger)
- [ ] Implement VerifyAndPay (proof check, payment release)
- [ ] Add security checks (ownership, state transitions)
- [ ] Unit tests for each instruction

**Deliverable:** Fully functional smart contract

**Risk:** Complex state transitions
**Mitigation:** State machine diagram first, code second

---

#### Day 5-6: Client SDK
**Target:** Rust SDK for interacting with marketplace

**Tasks:**
- [ ] MarketplaceClient struct (RPC wrapper)
- [ ] Instruction builders (typed, validated)
- [ ] Job fetcher (query by status, prover, etc.)
- [ ] Event parser (decode program logs)
- [ ] Helper functions (encode witness, decode proof)
- [ ] Integration tests against local validator
- [ ] Documentation and examples

**Deliverable:** SDK that programs can use

**Risk:** RPC reliability
**Mitigation:** Retry logic, multiple endpoints

---

#### Day 7: Prover Node MVP
**Target:** Basic proving daemon (mock proving for now)

**Tasks:**
- [ ] Job polling loop (query marketplace)
- [ ] Job claim logic (automatic selection)
- [ ] Mock proving (instant "proof" generation)
- [ ] Proof submission (call SDK)
- [ ] CLI interface (start, stop, status)
- [ ] Configuration file (RPC endpoint, keypair, etc.)

**Deliverable:** Runnable prover daemon

**Risk:** Timing/concurrency issues
**Mitigation:** Keep it simple, no optimization yet

---

#### Day 8: Week 1 Integration Testing
**Target:** End-to-end flow working

**Tasks:**
- [ ] Deploy program to local validator
- [ ] Start prover node daemon
- [ ] Create test job via SDK
- [ ] Verify prover claims job
- [ ] Verify prover submits mock proof
- [ ] Verify payment released
- [ ] Fix any integration bugs
- [ ] Document setup process

**Deliverable:** Working E2E flow on local validator

**Milestone:** ✅ Core infrastructure functional

---

### Week 2: Real Proving + Wallet (November 17-23)

**Goal:** Wallet app functional with real Halo2 proof generation

#### Day 9-10: Halo2 Integration
**Target:** Real Zcash Orchard proof generation

**Tasks:**
- [ ] Integrate Orchard circuit (zcash_proofs crate)
- [ ] Proving key management (load, cache)
- [ ] Witness building from transaction
- [ ] Proof generation function
- [ ] Proof verification function
- [ ] Benchmark proving times (target <15s)
- [ ] Optimize hot paths
- [ ] Replace mock proving in prover node

**Deliverable:** Real ZK proof generation working

**Risk:** Proving time exceeds 20s
**Mitigation:** Profiling, proving key caching, optimization

---

#### Day 11-12: Flutter Wallet UI
**Target:** Mobile UI with all screens

**Tasks:**
- [ ] Project setup (Flutter + dependencies)
- [ ] Home screen (balance display)
- [ ] Send screen (amount, address input)
- [ ] Receive screen (address + QR code)
- [ ] History screen (transaction list)
- [ ] Job status screen (proving progress)
- [ ] Settings screen (seed phrase, network)
- [ ] Navigation and routing
- [ ] Loading states
- [ ] Error handling UI

**Deliverable:** Complete UI mockup (no backend yet)

**Risk:** UI complexity
**Mitigation:** Use existing wallet as reference, keep it simple

---

#### Day 13-14: Rust FFI Bridge
**Target:** Connect Flutter to Rust backend

**Tasks:**
- [ ] FFI function definitions (create_wallet, send_transaction, etc.)
- [ ] Wallet operations (HD derivation, key management)
- [ ] Zcash light client integration (sync, balance)
- [ ] Marketplace client integration (create job, poll status)
- [ ] Transaction building (witness generation)
- [ ] FFI bindings generation (flutter_rust_bridge)
- [ ] Memory management (pass data across FFI)
- [ ] Error handling (propagate Rust errors to Dart)

**Deliverable:** Flutter can call Rust functions

**Risk:** FFI complexity and bugs
**Mitigation:** Extensive logging, test each function separately

---

#### Day 15: Week 2 Integration
**Target:** Working wallet end-to-end

**Tasks:**
- [ ] Connect UI to FFI functions
- [ ] Test wallet creation flow
- [ ] Test send transaction via marketplace
- [ ] Test job status updates
- [ ] Fix mobile-specific bugs
- [ ] Test on real device (not just emulator)
- [ ] Battery usage profiling
- [ ] Performance optimization

**Deliverable:** Functional wallet on mobile

**Milestone:** ✅ Wallet + marketplace working on devnet

---

### Week 3: Polish + Demo (November 24-30)

**Goal:** Production-ready demo + presentation

#### Day 16-17: UI/UX Polish
**Target:** Professional, bug-free interface

**Tasks:**
- [ ] Loading animations (smooth, informative)
- [ ] Error states (helpful messages)
- [ ] Success animations (celebrate completion)
- [ ] Color scheme and branding
- [ ] Typography and spacing
- [ ] Responsive layout (different screen sizes)
- [ ] Accessibility (contrast, font sizes)
- [ ] Dark mode (optional but nice)
- [ ] Onboarding flow (first-time users)

**Deliverable:** Polished, professional UI

**Risk:** Design bikeshedding
**Mitigation:** Time-box, good enough > perfect

---

#### Day 18: Performance Optimization
**Target:** <15s proof time, <1% battery usage

**Tasks:**
- [ ] Profile proof generation (identify bottlenecks)
- [ ] Optimize proving key loading
- [ ] Implement proving key caching
- [ ] Reduce RPC calls (batch, cache)
- [ ] Optimize witness building
- [ ] Battery usage profiling
- [ ] Memory optimization
- [ ] Benchmark and validate targets

**Deliverable:** Measurably fast performance

**Risk:** Can't hit performance targets
**Mitigation:** Test on high-end device if needed, document actual performance

---

#### Day 19: Demo Preparation
**Target:** Perfect 3-minute demo

**Tasks:**
- [ ] Write demo script (see DEMO_SCRIPT.md)
- [ ] Record side-by-side comparison video
- [ ] Create slides (10 slides max)
- [ ] Rehearse presentation (10+ times)
- [ ] Prepare backup phone (in case of issues)
- [ ] Pre-record backup video (if live demo fails)
- [ ] Test demo on fresh device
- [ ] Prepare Q&A responses

**Deliverable:** Rehearsed, confident presentation

**Risk:** Demo fails during presentation
**Mitigation:** Multiple rehearsals, backup video, redundancy

---

#### Day 20: Documentation
**Target:** Complete, clear documentation

**Tasks:**
- [ ] README with setup instructions
- [ ] Architecture diagrams (Mermaid)
- [ ] API documentation (SDK functions)
- [ ] Deployment guide (devnet/mainnet)
- [ ] Video walkthrough (how to use wallet)
- [ ] Code comments (clean, helpful)
- [ ] License files
- [ ] Contribution guidelines (post-hackathon)

**Deliverable:** Professional documentation

**Risk:** Running out of time
**Mitigation:** Templates prepared in advance

---

#### Day 21: Final Testing & Submission
**Target:** Bug-free submission

**Tasks:**
- [ ] Full E2E test on fresh device
- [ ] Record final demo video
- [ ] Test all demo scenarios
- [ ] Fix any last-minute bugs
- [ ] Submit to hackathon portal
- [ ] Deploy to Solana devnet (if not already)
- [ ] Tweet announcement
- [ ] Backup all code and materials

**Deliverable:** Submission complete!

**Milestone:** ✅ Hackathon submission delivered

---

## Post-Hackathon Roadmap

### Month 1-2: MVP Refinement (December-January)

**Goal:** Production-ready wallet, mainnet deployment

**Phase:** Beta testing with early adopters

#### Technical Tasks
- [ ] Security audit (smart contracts)
- [ ] Fix audit findings
- [ ] Mainnet deployment (Solana + Zcash)
- [ ] TUI for prover node (ratatui)
- [ ] Reputation system (SAS integration)
- [ ] Slashing mechanism (penalize bad provers)
- [ ] Rate limiting (prevent spam)
- [ ] Monitoring and alerting

#### Product Tasks
- [ ] Beta tester recruitment (50-100 users)
- [ ] User feedback collection
- [ ] Bug fixes from beta testing
- [ ] App store submission (iOS + Android)
- [ ] Prover onboarding documentation
- [ ] User documentation
- [ ] Support infrastructure (Discord, email)

#### Business Tasks
- [ ] Grant applications (Zcash, Solana, Light)
- [ ] Partnership conversations
- [ ] Blog posts and case studies
- [ ] Media outreach

**Success Metrics:**
- 100+ wallet installs
- 50+ active provers
- 1,000+ proofs generated
- <$0.10 average cost per proof
- Break even on infrastructure costs

**Funding:** Hackathon prizes + grants ($100K-300K target)

---

### Month 3-4: Second Use Case (February-March)

**Goal:** Expand beyond wallet, prove platform versatility

**Phase:** Anonymous voting for DAOs

#### Technical Tasks
- [ ] Voting circuit implementation
- [ ] Voting UI (mobile + web)
- [ ] Ballot encryption
- [ ] Result tallying (privacy-preserving)
- [ ] Integration with existing DAO tools
- [ ] Voting SDK documentation

#### Product Tasks
- [ ] Partner with 2-3 DAOs for pilots
- [ ] Run test votes
- [ ] Collect feedback
- [ ] Case studies
- [ ] Speaking at DAO conferences

#### Business Tasks
- [ ] Voting-specific pricing
- [ ] Marketing to DAO space
- [ ] Integration partnerships (Snapshot, Tally)

**Success Metrics:**
- 3+ DAOs using platform
- 10,000+ votes cast
- 200+ active provers (growth)
- 1,000+ wallet users
- $1K-2K MRR

**Funding:** Grants + early revenue

---

### Month 5-6: Third Use Case (April-May)

**Goal:** Establish multi-use case platform narrative

**Phase:** Private credentials

#### Technical Tasks
- [ ] Credential issuance circuit
- [ ] Verification circuit
- [ ] Credential SDK
- [ ] Issuer portal (web)
- [ ] Verifier SDK
- [ ] Selective disclosure support

#### Product Tasks
- [ ] Partner with credential issuers
- [ ] University diploma use case
- [ ] KYC provider use case
- [ ] Integration guides
- [ ] Developer documentation

#### Business Tasks
- [ ] Enterprise sales conversations
- [ ] Compliance documentation
- [ ] Case studies and benchmarks

**Success Metrics:**
- 2+ credential issuers integrated
- 5,000+ credentials issued
- 500+ active provers
- 5,000+ wallet users
- $5K-10K MRR

**Funding:** Seed round prep ($500K-1M target)

---

### Month 7-9: Platform Maturity (June-August)

**Goal:** General-purpose ZK marketplace

**Phase:** Developer ecosystem

#### Technical Tasks
- [ ] SDK for custom circuits
- [ ] Circuit marketplace (publish/discover)
- [ ] Multi-chain support (Ethereum L2s)
- [ ] Hardware acceleration support
- [ ] Enterprise API tier
- [ ] Advanced monitoring and analytics

#### Product Tasks
- [ ] Developer documentation
- [ ] Tutorial series
- [ ] Hackathon sponsorships
- [ ] Developer grants program
- [ ] Community building

#### Business Tasks
- [ ] Seed fundraising
- [ ] Team expansion (5-7 people)
- [ ] Partnership with ZK projects (Mina, Aztec, etc.)

**Success Metrics:**
- 10+ different use cases
- 1,000+ active provers
- 20,000+ users
- $20K-50K MRR
- Series A pipeline

**Funding:** Seed round ($500K-1M)

---

### Month 10-12: Scale (September-November)

**Goal:** Industry standard for mobile ZK

**Phase:** Growth and expansion

#### Technical Tasks
- [ ] Performance optimization at scale
- [ ] Geographic prover distribution
- [ ] Advanced slashing and reputation
- [ ] White-label solutions
- [ ] API v2 (based on learnings)

#### Product Tasks
- [ ] International expansion
- [ ] Multi-language support
- [ ] Advanced analytics dashboard
- [ ] Customer success team

#### Business Tasks
- [ ] Series A preparation
- [ ] Revenue optimization
- [ ] Strategic partnerships
- [ ] Industry positioning

**Success Metrics:**
- 50,000+ users
- 2,000+ active provers
- 20+ use cases
- $100K+ MRR
- Series A ready

**Funding:** Series A prep ($3M-7M target)

---

## Year 2+: Long-Term Vision

### Year 2 Goals
- 100,000+ users across all applications
- 5,000+ active provers globally
- 50+ use cases and integrations
- $500K+ MRR
- Multi-chain support (Ethereum, Polygon, etc.)
- Enterprise customers

### Year 3 Goals
- 1M+ users
- 10,000+ provers
- Industry-standard infrastructure
- Acquisition discussions or path to profitability

---

## Milestones Summary

| Milestone | Date | Key Deliverable | Success Metric |
|-----------|------|-----------------|----------------|
| **Week 1** | Nov 16 | Core infrastructure | E2E flow on local validator |
| **Week 2** | Nov 23 | Wallet + proving | Working demo on devnet |
| **Week 3** | Dec 1 | Submission | Hackathon delivered |
| **Month 2** | Jan 31 | Production launch | 100+ users, 50+ provers |
| **Month 4** | Mar 31 | Voting use case | 3+ DAO partners |
| **Month 6** | May 31 | Credentials use case | 2+ issuers |
| **Month 9** | Aug 31 | Platform maturity | 10+ use cases |
| **Month 12** | Nov 30 | Series A ready | $100K+ MRR |

---

## Risk Management

### Hackathon Risks

**Technical Risk: Proof time exceeds target**
- Impact: High (breaks demo narrative)
- Probability: Medium
- Mitigation: Optimization sprints on Day 18, use high-end device if needed
- Fallback: Adjust messaging to "5x faster" instead of "10x"

**Technical Risk: Integration bugs**
- Impact: High (broken demo)
- Probability: Medium
- Mitigation: Daily integration testing, buffer day before submission
- Fallback: Pre-recorded video backup

**Resource Risk: Feature creep**
- Impact: Medium (miss deadline)
- Probability: High
- Mitigation: Strict scope control, MVP-only features
- Fallback: Cut non-essential features ruthlessly

**Demo Risk: Live demo fails**
- Impact: High (poor presentation)
- Probability: Low
- Mitigation: 10+ rehearsals, backup phone, backup video
- Fallback: Show pre-recorded video, explain issue honestly

### Post-Hackathon Risks

**Market Risk: No user adoption**
- Impact: High (product-market fit failure)
- Probability: Low (validated demand)
- Mitigation: Beta testing, user feedback, iteration
- Fallback: Pivot to enterprise/B2B model

**Technical Risk: Security vulnerability**
- Impact: Critical (loss of funds)
- Probability: Medium (complex crypto)
- Mitigation: Security audit, bug bounty, gradual rollout
- Fallback: Circuit breakers, insurance

**Business Risk: Funding gap**
- Impact: High (can't sustain development)
- Probability: Medium
- Mitigation: Multiple funding sources (grants, prizes, VCs)
- Fallback: Consulting revenue, slower growth

**Competitive Risk: Copycat projects**
- Impact: Medium (market share loss)
- Probability: Medium
- Mitigation: Fast execution, network effects, partnerships
- Fallback: Focus on quality and trust

---

## Dependencies & Blockers

### External Dependencies

**Solana Foundation:**
- Devnet/mainnet stability
- RPC endpoint reliability
- Grant approval timeline

**Light Protocol:**
- SDK stability
- Documentation quality
- Technical support

**Zcash Foundation:**
- Circuit availability
- Grant approval
- Partnership willingness

### Internal Dependencies

**Day 1-8:** Smart contract must work before SDK
**Day 9-14:** SDK must work before wallet integration
**Day 15-21:** Everything must work before demo

**Critical path:** Program → SDK → Prover → Wallet → Demo

Any delay in early stages cascades to later stages.

---

## Adjustment Strategy

**Daily standups:** Review progress, identify blockers
**Weekly retrospectives:** Adjust scope if needed
**Buffer days:** Day 8, Day 15, Day 21 are buffer/integration days
**Scope flexibility:** Cut features to hit timeline, quality > quantity

**Red lines (cannot compromise):**
- Working E2E demo
- <30s proof time (adjust messaging if needed)
- Clean presentation
- On-time submission

**Flexible (can adjust):**
- Feature completeness
- UI polish
- Documentation depth
- Performance optimization

---

## Success Definition

**Hackathon success:**
- ✅ Working demo (no bugs)
- ✅ Top 3 placement
- ✅ Positive judge feedback
- ✅ Media attention

**Long-term success:**
- Production launch with real users
- Multiple use cases live
- Sustainable revenue
- Industry recognition

**Ultimate success:**
- Industry-standard infrastructure for mobile ZK
- Thousands of provers, millions of users
- Privacy technology that actually gets used
