# Documentation Index

Welcome to the CypherLink documentation. This index provides a quick overview of all available documentation and recommended reading paths.

---

## Quick Start

**New to the project?** Start here:

1. [README.md](../README.md) - Project overview
2. [APPROACH.md](APPROACH.md) - Why we built a wallet as demo
3. [ROADMAP.md](ROADMAP.md) - 21-day development timeline

**Total reading time:** ~20 minutes

---

## Strategic Documentation

### [STRATEGY.md](STRATEGY.md)
**Purpose:** Complete hackathon and go-to-market strategy

**Read this if you want to understand:**
- How we plan to win the hackathon
- Target judges and key messages
- Post-hackathon go-to-market (12 months)
- Revenue model and funding strategy
- Competitive positioning

**Reading time:** 30 minutes

**Key sections:**
- Hackathon Strategy → Zooko, Toly, Balaji, Cobie messaging
- Target Markets → Zcash users, DAOs, enterprise
- Revenue Model → Unit economics and scaling
- Funding Strategy → Grants to Series A

---

### [APPROACH.md](APPROACH.md)
**Purpose:** Rationale for wallet-first demo approach

**Read this if you want to understand:**
- Why wallet instead of generic marketplace
- Use case deep dive (Zcash shielded transactions)
- Privacy guarantees and security model
- Alternatives considered and rejected
- Scaling path (wallet → platform)

**Reading time:** 20 minutes

**Key sections:**
- Decision → Wallet vs marketplace UI
- Rationale → Emotional impact, measurability
- Use Case → Zcash flow comparison
- Trade-offs → Focus vs comprehensiveness

---

### [ROADMAP.md](ROADMAP.md)
**Purpose:** Detailed development timeline

**Read this if you want to understand:**
- 21-day hackathon breakdown (day-by-day)
- Post-hackathon roadmap (12 months)
- Milestones and success metrics
- Risk management strategy
- Dependencies and critical path

**Reading time:** 25 minutes

**Key sections:**
- Week 1: Core Infrastructure
- Week 2: Real Proving + Wallet
- Week 3: Polish + Demo
- Post-Hackathon: Month-by-month roadmap

---

## Presentation Materials

### [PITCH.md](PITCH.md)
**Purpose:** Complete pitch deck for hackathon judges

**Read this if you want to:**
- Deliver the hackathon presentation
- Understand messaging by judge
- Prepare for Q&A
- Learn demo highlights

**Reading time:** 35 minutes

**Key sections:**
- Elevator Pitch (30 seconds)
- Full Presentation (3 minutes, 10 slides)
- Judge-Specific Messaging
- Prepared Q&A (25+ questions)
- Demo Highlights

**Use case:** Rehearse this before presentation

---

### [DEMO_SCRIPT.md](DEMO_SCRIPT.md)
**Purpose:** Step-by-step demo execution guide

**Read this if you want to:**
- Run the live demo
- Understand timing and narration
- Prepare backup plans
- Ensure successful demo

**Reading time:** 25 minutes

**Key sections:**
- Pre-Demo Checklist
- Script (3 minutes, word-by-word)
- Backup Plans (4 failure scenarios)
- Rehearsal Checklist
- Day-Of Checklist

**Use case:** Rehearse 10+ times before presentation

---

## Technical Documentation

### [ARCHITECTURE.md](ARCHITECTURE.md)
**Purpose:** Complete system architecture and design

**Read this if you want to understand:**
- How the system works end-to-end
- Component architecture (mobile, Solana, prover)
- Data flow and state management
- Performance characteristics
- Security model

**Reading time:** 40 minutes

**Key sections:**
- System Overview (Mermaid diagrams)
- Component Architecture (6 layers)
- Data Flow (sequence diagrams)
- Performance Characteristics
- Security Model

**Audience:** Developers, architects, technical judges

---

### [TECH_STACK.md](TECH_STACK.md)
**Purpose:** Technology choices and justifications

**Read this if you want to understand:**
- Why we chose each technology
- Versions and stability assessment
- Alternatives considered
- Trade-offs made
- Future evolution

**Reading time:** 30 minutes

**Key sections:**
- Smart Contract Layer (Solana, Light, SAS)
- Client SDK (Rust)
- Prover Node (Halo2, Ratatui)
- Mobile Layer (Flutter + FFI)
- Cryptography (ML-KEM, AES-GCM)

**Audience:** Developers, technical decision-makers

---

### [DECISION_LOG.md](DECISION_LOG.md)
**Purpose:** Historical record of architectural decisions

**Read this if you want to understand:**
- Why we made specific choices
- What alternatives we considered
- Trade-offs we accepted
- Context and rationale

**Reading time:** 35 minutes

**Key decisions:**
- Solana bare metal (no Anchor)
- Light Protocol for ZK Compression
- Wallet demo approach
- ML-KEM (post-quantum crypto)
- SAS for reputation
- 20+ more decisions

**Audience:** Team members, auditors, future maintainers

---

## Supporting Files

### [README.md](../README.md)
**Purpose:** Project overview and quick start

**Sections:**
- Overview and features
- Architecture diagram
- Quick links to all docs
- Repository structure
- Getting started
- License

**Reading time:** 10 minutes

**Audience:** Everyone (entry point)

---

### [.gitignore](../.gitignore)
**Purpose:** Prevent committing sensitive/unnecessary files

**Coverage:**
- Rust/Cargo artifacts
- Solana keypairs (CRITICAL)
- Flutter/Dart builds
- Mobile platform files
- Proving keys (large files)
- Environment secrets
- IDE configurations

**Audience:** Developers

---

## Reading Paths

### For Hackathon Judges

**Time budget: 15 minutes**

1. README.md (5 min) - What is this?
2. APPROACH.md (5 min) - Why wallet?
3. PITCH.md - Elevator pitch section (5 min)

**Extended:** +30 min → Read full PITCH.md and ARCHITECTURE.md

---

### For Developers

**Time budget: 2 hours**

1. README.md (10 min) - Overview
2. ARCHITECTURE.md (40 min) - How it works
3. TECH_STACK.md (30 min) - What we use
4. DECISION_LOG.md (40 min) - Why we chose it

**Extended:** +1 hour → Read ROADMAP.md for implementation plan

---

### For Business/Strategy

**Time budget: 1.5 hours**

1. README.md (10 min) - Overview
2. STRATEGY.md (30 min) - Go-to-market
3. APPROACH.md (20 min) - Product rationale
4. ROADMAP.md (30 min) - Execution plan

**Extended:** +30 min → Read PITCH.md for investor presentation

---

### For Presenters (Demo Team)

**Time budget: 2 hours + rehearsal**

1. PITCH.md (35 min) - Memorize pitch
2. DEMO_SCRIPT.md (25 min) - Learn script
3. APPROACH.md (20 min) - Understand rationale
4. PITCH.md Q&A section (40 min) - Prepare answers

**Then:** Rehearse 10+ times using DEMO_SCRIPT.md

---

### For New Team Members

**Time budget: 3 hours**

**Day 1:**
1. README.md (10 min)
2. APPROACH.md (20 min)
3. ROADMAP.md (30 min)

**Day 2:**
4. ARCHITECTURE.md (40 min)
5. TECH_STACK.md (30 min)

**Day 3:**
6. DECISION_LOG.md (40 min)
7. STRATEGY.md (30 min)

**Result:** Complete understanding of project

---

## Documentation Stats

| Document | Size | Reading Time | Audience |
|----------|------|--------------|----------|
| README.md | 6 KB | 10 min | Everyone |
| STRATEGY.md | 16 KB | 30 min | Business |
| APPROACH.md | 13 KB | 20 min | All |
| ROADMAP.md | 17 KB | 25 min | Team |
| PITCH.md | 20 KB | 35 min | Presenters |
| DEMO_SCRIPT.md | 15 KB | 25 min | Presenters |
| ARCHITECTURE.md | 23 KB | 40 min | Developers |
| TECH_STACK.md | 16 KB | 30 min | Developers |
| DECISION_LOG.md | 21 KB | 35 min | Team |
| **TOTAL** | **~141 KB** | **~4.5 hours** | - |

---

## Maintenance

### Updating Documentation

**When code changes:**
- Update ARCHITECTURE.md if system design changes
- Update TECH_STACK.md if dependencies change
- Log decision in DECISION_LOG.md

**When strategy evolves:**
- Update STRATEGY.md with new insights
- Update ROADMAP.md with actual progress
- Update PITCH.md if messaging changes

**When demos/presentations happen:**
- Update DEMO_SCRIPT.md with lessons learned
- Update PITCH.md Q&A with new questions
- Document feedback in DECISION_LOG.md

### Review Schedule

- **Daily:** Update ROADMAP.md progress
- **Weekly:** Review and update relevant sections
- **Monthly:** Comprehensive documentation review
- **Post-hackathon:** Major update based on learnings

---

## Getting Help

**Questions about:**
- **Strategy/Business:** See STRATEGY.md, APPROACH.md
- **Technical implementation:** See ARCHITECTURE.md, TECH_STACK.md
- **Timeline/Planning:** See ROADMAP.md
- **Presentation:** See PITCH.md, DEMO_SCRIPT.md
- **Past decisions:** See DECISION_LOG.md

**Still have questions?**
- Check cross-references in each document
- Search across all markdown files
- Reach out to team members

---

## Contributing to Docs

**When adding new documentation:**
1. Follow existing format and style
2. Add entry to this INDEX.md
3. Cross-reference from relevant docs
4. Update README.md if major addition
5. Keep language clear and concise

**Style guidelines:**
- Use markdown headers properly (# → ## → ###)
- Include code blocks with syntax highlighting
- Add diagrams where helpful (Mermaid preferred)
- Cross-link related sections
- Keep paragraphs short (3-5 sentences)
- Use bullet points for lists
- Bold key terms on first use

---

## Version History

- **November 10, 2025:** Initial documentation created
  - All 9 strategic/technical documents
  - README.md and .gitignore
  - This INDEX.md

**Current version:** v1.0 (Hackathon MVP)

---

## Quick Reference

**Most important documents:**
1. README.md - Start here
2. APPROACH.md - Why this approach
3. ROADMAP.md - What we're building when
4. ARCHITECTURE.md - How it works
5. PITCH.md - How we present it

**Before presenting:**
- Read PITCH.md
- Rehearse DEMO_SCRIPT.md
- Review APPROACH.md (know the "why")

**Before coding:**
- Read ARCHITECTURE.md
- Check TECH_STACK.md
- Review DECISION_LOG.md for context

**Before planning:**
- Read ROADMAP.md
- Check STRATEGY.md
- Review milestones and KPIs

---

**Happy reading!**

For the latest version of this documentation, see: `zyberlink/docs/`
