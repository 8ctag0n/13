# Dynamic Pricing System - Documentation Hub

Welcome to the comprehensive documentation for ZyberLink's Dynamic Pricing System. This document serves as your entry point to understanding how FHE operations are priced in the decentralized marketplace.

---

## Quick Navigation

### For First-Time Readers
Start here for a clear overview of what dynamic pricing is and why it matters:
- **[DYNAMIC_PRICING.md](./DYNAMIC_PRICING.md)** - Main documentation with examples and architecture overview

### For Developers Integrating the System
If you're building on top of ZyberLink, these guides have code examples:
- **[DYNAMIC_PRICING_USAGE.md](./DYNAMIC_PRICING_USAGE.md)** - Practical integration guide with SDK and API examples
- **[DYNAMIC_PRICING_TECHNICAL.md](./DYNAMIC_PRICING_TECHNICAL.md)** - Deep technical implementation details

---

## Documentation Structure

```
DYNAMIC PRICING SYSTEM
├── DYNAMIC_PRICING.md (333 lines)
│   ├── Overview & Why Dynamic Pricing
│   ├── 5 Complexity Tiers (O(1) to O(n×m))
│   ├── Architecture at a glance
│   ├── Real-world examples
│   ├── Testing information
│   └── Getting started guide
│
├── DYNAMIC_PRICING_TECHNICAL.md (814 lines)
│   ├── Complete system architecture
│   ├── Core data structures
│   ├── Detailed pricing algorithms (per tier)
│   ├── Validation flows (on-chain & off-chain)
│   ├── Integration points
│   ├── Edge cases & limitations
│   ├── Performance analysis
│   └── Future improvements
│
└── DYNAMIC_PRICING_USAGE.md (996 lines)
    ├── Job creator guide with examples
    ├── Prover profitability evaluation
    ├── SDK integration examples (Rust)
    ├── API integration examples (JS/TS, Python)
    ├── Frontend integration (Svelte)
    ├── Best practices
    ├── Troubleshooting guide
    └── FAQ
```

**Total:** 2,143 lines of comprehensive documentation

---

## System at a Glance

### What is Dynamic Pricing?

ZyberLink's Dynamic Pricing System automatically determines the minimum payment required for FHE operations based on their computational complexity.

**The Problem:** Traditional flat fees mean simple operations subsidize complex ones, causing provers to lose money on expensive operations.

**The Solution:** Tier-based pricing where:
- Operations are categorized 1-5 by complexity
- Costs automatically scale with operation parameters
- Validation happens on-chain at the protocol level
- Provers can evaluate profitability before accepting jobs

### 5 Complexity Tiers

| Tier | Complexity | Base Cost | Operations | Example |
|------|-----------|-----------|-----------|---------|
| **1** | O(1) | 0.001 SOL | Add, Multiply | `5 + encrypt(x)` |
| **2** | O(n) | 0.001 + 0.0001×n SOL | Sum | Sum of 1,000 values |
| **3** | O(1) + bootstrap | 0.005 SOL | Threshold, RangeCheck | Age >= 18? |
| **4** | O(n) + predicates | 0.005 + 0.0003×n SOL | CountIf, Average | Count age > 21 |
| **5** | O(n×m) exponential | 0.1 + 0.02×m^1.5 SOL | Histogram | 5-way vote tally |

**All prices are per prover.** Total cost = tier_price × number_of_provers

### Key Statistics

- **54+ passing tests** validating all pricing scenarios
- **2,143 lines** of comprehensive documentation
- **5 integration points**: Shared types, Solana program, Backend API, Frontend UI, ROI calculator
- **100% on-chain validation** - Underpriced jobs rejected at protocol level

---

## Component Overview

### 1. Core Pricing Logic
**File:** `/shared/types/src/fhe.rs`

Single source of truth for all pricing calculations. Every FHE operation has:
```rust
pub fn get_cost_config(&self) -> OperationCostConfig {
    // Returns: min_payment_lamports, timeout_seconds, complexity_tier
}
```

### 2. On-Chain Validation
**File:** `/programs/zyberlink/src/processor/create_job.rs` (lines 97-144)

Solana program validates prices during job creation:
```
User submits job → Program calculates minimum → Rejects if underpriced
```

### 3. Backend Cost Estimation API
**File:** `/blink-server/src/api_handlers.rs` (lines 342-430)

Endpoint: `POST /api/estimate-cost`

Provides real-time cost quotes before job submission.

### 4. Prover ROI Calculator
**File:** `/prover-node/src/roi_calculator.rs`

Provers use this to evaluate job profitability:
```
revenue_per_prover = price / required_provers
profit = revenue - (cost × operational_overhead)
roi_percentage = (profit / cost) × 100
```

Only profitable jobs are accepted.

### 5. Frontend Integration
**File:** `/frontend/src/lib/components/CreateJob.svelte`

Dynamic UI showing real-time costs as users adjust parameters.

---

## Common Use Cases

### Use Case 1: Age Verification (ZK-Passport)

Verify user is 18+ without revealing age.

**Operation:** `Threshold { threshold: 18, greater_or_equal: true }`
**Tier:** 3
**Cost:** 0.005 SOL per prover
**3 Provers:** 0.015 SOL total
**Timeout:** 300 seconds (5 minutes)

### Use Case 2: Census Population Count

Count 50,000 encrypted votes without revealing individual data.

**Operation:** `Sum { expected_count: 50000 }`
**Tier:** 2
**Cost:** 0.001 + (50,000 × 0.0001) = 5.001 SOL per prover
**3 Provers:** 15.003 SOL total
**Timeout:** 100,060 seconds (~27.8 hours)

### Use Case 3: Private Election Tallying

Tally votes for 10 candidates.

**Operation:** `Histogram { bins: 10 }`
**Tier:** 5
**Cost:** 0.1 + (0.02 × 10^1.5) ≈ 0.732 SOL per prover
**3 Provers:** ~2.196 SOL total
**Timeout:** 1,600 seconds (~26.7 minutes)

---

## Getting Started

### For Job Creators (1 minute)

1. **Estimate cost before creating:**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

2. **Use the returned cost** as your minimum payment
3. **Create the job** with sufficient funds in escrow

See [DYNAMIC_PRICING_USAGE.md](./DYNAMIC_PRICING_USAGE.md#for-job-creators) for complete examples (dev-job usage).

### For Provers (2 minutes)

1. **Configure your ROI requirements:**
```rust
let calculator = ROICalculator::new(
    20.0,   // 20% minimum ROI
    1.5     // 50% operational overhead
);
```

2. **Evaluate jobs before claiming:**
```rust
let roi = calculator.evaluate_job(&circuit_type, price, provers);
if roi.is_profitable {
    claim_job(&job)?;
}
```

See [DYNAMIC_PRICING_USAGE.md](./DYNAMIC_PRICING_USAGE.md#for-provers) for full prover guide.

### For Developers (5 minutes)

1. **Calculate minimum price:**
```rust
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;
```

2. **Use dynamic timeout:**
```rust
let ix = builder.create_fhe_job(
    creator,
    job_id,
    &encrypted_data,
    fhe_config,
    min_price,                      // or higher
    cost_config.timeout_seconds     // use this!
)?;
```

See [DYNAMIC_PRICING_USAGE.md](./DYNAMIC_PRICING_USAGE.md#sdk-integration-examples) for SDK examples.

---

## Testing Overview

The system includes comprehensive test coverage:

```
✅ Shared Types: 57 unit tests
   - Tier 1-5 pricing validation
   - Serialization tests
   - Cost scaling verification
   - Realistic scenario tests (census, voting, age verification)

✅ Integration Tests: 4 tests
   - On-chain price validation
   - Underpriced job rejection
   - Prover count scaling

✅ E2E Tests: 7 tests (in test-dynamic-pricing-e2e.sh)
   - API cost estimation
   - Database integration
   - End-to-end job flow

✅ ROI Calculator: 4 tests
   - Profitability calculations
   - Minimum price computation
   - Operational overhead handling

TOTAL: 72 tests passing
```

Run all tests:
```bash
cd /home/deploy/experimental/zyberlink-demo
bash test-dynamic-pricing-e2e.sh
```

---

## Document Quick Reference

### Main Documentation (DYNAMIC_PRICING.md)

**Read this for:** High-level overview, architecture, why dynamic pricing matters

**Key sections:**
- Overview and problem statement
- 5 Complexity tiers with pricing examples
- Real-world use cases (4 examples)
- Implementation status checklist
- Testing overview
- Getting started guide
- Security guarantees

### Technical Implementation (DYNAMIC_PRICING_TECHNICAL.md)

**Read this for:** Algorithm details, data structures, validation flows

**Key sections:**
- Complete component hierarchy (4 layers)
- Core data structures (OperationCostConfig, FheOperation)
- Detailed pricing algorithm per tier
- On-chain and off-chain validation flows
- Integration points with code
- Edge cases and limitations
- Performance analysis
- Testing strategy

### Usage Guide (DYNAMIC_PRICING_USAGE.md)

**Read this for:** Code examples, practical integration, troubleshooting

**Key sections:**
- Job creator guide with examples
- Prover profitability evaluation
- SDK integration examples (Rust)
- API integration examples (JS/TS, Python)
- Frontend integration (Svelte)
- Best practices checklist
- Troubleshooting guide
- FAQ (15+ questions)

---

## Key Facts

### Pricing Formula by Tier

| Tier | Formula | Timeout Formula |
|------|---------|-----------------|
| **1** | Base = 0.001 SOL | 60s |
| **2** | 0.001 + (n × 0.0001) SOL | 60 + (n × 2)s |
| **3** | 0.005 SOL | 300s |
| **4a** (Average) | 0.001 + (n × 0.0002) SOL | 60 + (n × 3)s |
| **4b** (CountIf) | 0.005 + (n × 0.0003) SOL | 300 + (n × 5)s |
| **5** | 0.1 + (0.02 × m^1.5) SOL | 600 + (m × 100)s |

### Validation Happens At

1. **Backend API** - Before signing (UX: early rejection)
2. **Solana Program** - During job creation (Security: protocol-level)
3. **Prover Node** - Before claiming (Economics: ROI check)

### Files to Know

| File | Purpose | Status |
|------|---------|--------|
| `/shared/types/src/fhe.rs` | Core pricing logic | ✅ Implemented (8 operations) |
| `/programs/zyberlink/src/processor/create_job.rs` | On-chain validation | ✅ Implemented (lines 97-144) |
| `/blink-server/src/api_handlers.rs` | Cost estimation API | ✅ Implemented (lines 342-430) |
| `/prover-node/src/roi_calculator.rs` | Profitability calculator | ✅ Implemented |
| `/frontend/src/lib/components/CreateJob.svelte` | Frontend UI | ✅ Integrated |
| `/test-dynamic-pricing-e2e.sh` | E2E test suite | ✅ 7 tests passing |

---

## Frequently Asked Questions

**Q: What if SOL price changes?**
A: Pricing is in lamports (on-chain currency), not USD. If SOL price doubles, the lamport cost stays the same, but USD cost doubles.

**Q: Can I pay more than minimum?**
A: Yes! Higher payments can incentivize faster prover response and act as priority fees.

**Q: Can I use a custom timeout?**
A: Yes, but it must be >= the dynamic timeout from the operation's cost config.

**Q: Do provers negotiate prices?**
A: Not currently. Prices are deterministic based on complexity. Future enhancement could add bidding.

**Q: Are there bulk discounts?**
A: Not currently. Each job pays the same per-prover cost. Future could implement volume discounts.

See [DYNAMIC_PRICING_USAGE.md#faq](./DYNAMIC_PRICING_USAGE.md#faq) for 15+ more questions.

---

## Architecture Diagram

```
User Interface (Svelte Frontend)
    │
    ├──> Estimates Cost
    │    POST /api/estimate-cost
    │    ↓
    │    Backend API (Actix-web)
    │    ├─> Parses operation parameters
    │    ├─> Calls FheOperation::get_cost_config()
    │    └─> Returns: {tier, price, timeout, compute_ms}
    │
    ├──> Creates Job
    │    Sign transaction
    │    ↓
    │    Solana Blockchain
    │    ├─> Receives create_job instruction
    │    ├─> Validates: price >= cost_config.min × provers
    │    ├─> Rejects if underpriced
    │    └─> Creates job account if valid
    │
Prover Node
    ├─> Sees available job
    ├─> Evaluates with ROICalculator
    ├─> Checks: roi_percentage >= min_roi_threshold
    ├─> Accepts if profitable
    └─> Executes FHE computation
```

---

## Navigation Tips

1. **Lost?** Start with [DYNAMIC_PRICING.md](./DYNAMIC_PRICING.md)
2. **Need examples?** Jump to [DYNAMIC_PRICING_USAGE.md](./DYNAMIC_PRICING_USAGE.md)
3. **Want details?** Read [DYNAMIC_PRICING_TECHNICAL.md](./DYNAMIC_PRICING_TECHNICAL.md)
4. **Have questions?** Check [DYNAMIC_PRICING_USAGE.md#faq](./DYNAMIC_PRICING_USAGE.md#faq)

---

## System Status

| Component | Tests | Status |
|-----------|-------|--------|
| Core Types | 57 | ✅ Passing |
| Integration | 4 | ✅ Passing |
| E2E | 7 | ✅ Passing |
| ROI Calculator | 4 | ✅ Passing |
| **TOTAL** | **72** | **✅ Passing** |

**Version:** 1.0.0
**Last Updated:** 2025-11-21
**Maintained By:** ZyberLink Core Team

---

## Contributing

Found an issue in the pricing logic or documentation?

1. Review the relevant documentation section
2. Check [DYNAMIC_PRICING_TECHNICAL.md](./DYNAMIC_PRICING_TECHNICAL.md#edge-cases-and-limitations) for known limitations
3. Open an issue on GitHub with detailed information

---

## Related Documentation

- **[API_REFERENCE.md](./API_REFERENCE.md)** - Complete API documentation
- **[HISTOGRAM_FHE_OPTIMIZATION.md](./HISTOGRAM_FHE_OPTIMIZATION.md)** - Tier 5 complexity deep dive
- **[SUMMARY.md](./SUMMARY.md)** - Project overview

---

**Ready to get started?**

- **Creators:** Jump to [DYNAMIC_PRICING_USAGE.md - For Job Creators](./DYNAMIC_PRICING_USAGE.md#for-job-creators) (dev-job)
- **Provers:** Jump to [DYNAMIC_PRICING_USAGE.md - For Provers](./DYNAMIC_PRICING_USAGE.md#for-provers)
- **Developers:** Jump to [DYNAMIC_PRICING_USAGE.md - SDK Integration](./DYNAMIC_PRICING_USAGE.md#sdk-integration-examples)
