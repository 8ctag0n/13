# Dynamic Pricing System

**ZyberLink Marketplace - Intelligent FHE Operation Pricing**

---

## Overview

The Dynamic Pricing System is a comprehensive pricing mechanism for ZyberLink's decentralized marketplace that automatically determines the minimum payment required for Fully Homomorphic Encryption (FHE) operations based on their computational complexity.

Instead of using a flat fee for all operations, ZyberLink implements a **tier-based pricing model** where each FHE operation has a predetermined cost tier (1-5) reflecting its computational requirements, with prices automatically scaling based on operation parameters and the number of provers required.

## Why Dynamic Pricing?

### The Problem

Traditional compute marketplaces face several challenges:

1. **Flat pricing doesn't reflect computational cost** - Simple operations subsidize complex ones
2. **Provers lose money on expensive operations** - Leading to job abandonment and poor service quality
3. **Price discovery is manual** - Users don't know fair prices before creating jobs
4. **No protection against underpricing** - System vulnerable to spam and DoS attacks

### The Solution

ZyberLink's Dynamic Pricing System addresses these issues by:

- **Automatic cost calculation** based on operation complexity (O(1) to O(n×m))
- **Transparent pricing tiers** (1-5) visible to all participants
- **Multi-prover cost aggregation** - Total cost scales linearly with prover count
- **On-chain validation** - Solana program rejects underpriced jobs
- **ROI calculator** for provers to evaluate job profitability
- **Real-time cost estimation API** for frontend integration

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│                     DYNAMIC PRICING FLOW                        │
└─────────────────────────────────────────────────────────────────┘

Frontend (Svelte)                Backend API               Solana Program
     │                                │                           │
     │  1. Select Operation          │                           │
     │     (Add, Histogram, etc)     │                           │
     │                                │                           │
     │  2. POST /api/estimate-cost   │                           │
     ├──────────────────────────────>│                           │
     │                                │                           │
     │                                │  3. FheOperation          │
     │                                │     .get_cost_config()    │
     │                                │                           │
     │  4. Cost Response              │                           │
     │     (tier, price, timeout)     │                           │
     │<──────────────────────────────┤                           │
     │                                │                           │
     │  5. Display total cost         │                           │
     │     User confirms              │                           │
     │                                │                           │
     │  6. Create Job TX              │                           │
     ├───────────────────────────────────────────────────────────>│
     │                                │                           │
     │                                │  7. Validate price        │
     │                                │     cost_config.min × n   │
     │                                │                           │
     │  8. Success/Error              │                           │
     │<───────────────────────────────────────────────────────────┤
     │                                │                           │
```

## Complexity Tiers

ZyberLink categorizes FHE operations into 5 complexity tiers:

| Tier | Complexity | Operations | Price/Prover | Timeout | Use Cases |
|------|------------|------------|--------------|---------|-----------|
| **1** | O(1) | Add, Multiply | 0.001 SOL | 60s | Simple arithmetic |
| **2** | O(n) | Sum | 0.001 + n×0.0001 SOL | 60 + n×2s | Census counting |
| **3** | O(1) + bootstrap | Threshold, RangeCheck | 0.005 SOL | 300s | Age verification |
| **4** | O(n) + predicates | Average, CountIf | Variable | Variable | Conditional statistics |
| **5** | O(n×m) | Histogram | 0.1 + 0.02×m^1.5 SOL | 600 + m×100s | Voting, distributions |

**Note:** All prices are **per prover**. Total job cost = tier_price × number_of_provers.

## Pricing Examples

### Example 1: Simple Addition (Tier 1)
```
Operation: Add(5)
Provers: 3
───────────────────────────
Per-prover cost: 0.001 SOL (1,000,000 lamports)
Total minimum:   0.003 SOL (3,000,000 lamports)
Timeout:         60 seconds
```

### Example 2: Age Verification (Tier 3)
```
Operation: Threshold { threshold: 18, greater_or_equal: true }
Provers: 3
───────────────────────────
Per-prover cost: 0.005 SOL (5,000,000 lamports)
Total minimum:   0.015 SOL (15,000,000 lamports)
Timeout:         300 seconds (5 minutes)
```

### Example 3: Voting Histogram (Tier 5)
```
Operation: Histogram { bins: 5 }  // 5 candidates
Provers: 3
───────────────────────────
Per-prover cost: ~0.324 SOL (324,000,000 lamports)
Total minimum:   ~0.972 SOL (972,000,000 lamports)
Timeout:         1100 seconds (~18 minutes)
```

### Example 4: Census Sum (Tier 2 - Scales with count)
```
Operation: Sum { expected_count: 10,000 }
Provers: 3
───────────────────────────
Per-prover cost: 1.001 SOL (1,001,000,000 lamports)
  Base: 0.001 SOL
  Per-item: 10,000 × 0.0001 SOL = 1.0 SOL
Total minimum:   3.003 SOL (3,003,000,000 lamports)
Timeout:         20,060 seconds (~5.5 hours)
```

## Key Features

### 1. Automatic Cost Configuration

Every `FheOperation` has a built-in `get_cost_config()` method that returns:

```rust
pub struct OperationCostConfig {
    pub min_payment_lamports: u64,  // Minimum price per prover
    pub timeout_seconds: i64,        // Dynamic timeout
    pub complexity_tier: u8,         // 1-5 tier
}
```

### 2. On-Chain Validation

The Solana program validates pricing during job creation:

```rust
// In create_job processor (lines 109-128)
let cost_config = fhe_op.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (required_provers as u64);

if price_lamports < total_min_price {
    return Err(ZyberLinkProgramError::InvalidPrice);
}
```

**Result:** Underpriced jobs are rejected at the protocol level.

### 3. Cost Estimation API

Backend provides `/api/estimate-cost` endpoint for real-time pricing:

```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 10,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 732050808,
  "min_payment_sol": 0.732050808,
  "total_min_payment_lamports": 2196152424,
  "total_min_payment_sol": 2.196152424,
  "timeout_seconds": 1600,
  "estimated_compute_ms": 30500
}
```

### 4. ROI Calculator for Provers

Prover nodes include an intelligent ROI calculator to evaluate job profitability:

```rust
let calculator = ROICalculator::new(20.0, 1.5);  // 20% min ROI, 1.5x overhead
let roi = calculator.evaluate_job(&circuit_type, price_lamports, required_provers);

if roi.is_profitable {
    println!("Expected ROI: {:.1}%", roi.roi_percentage);
    // Accept job
}
```

### 5. Frontend Integration

The Create Job UI dynamically displays costs as users configure operations:

- **Operation dropdown** - Select FHE operation
- **Parameter inputs** - Operation-specific parameters (bins, thresholds, etc.)
- **Prover slider** - Adjust number of provers
- **Real-time cost display** - Shows total SOL cost before submission
- **Warning indicators** - Alerts for expensive operations

## Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| Core Types | ✅ Implemented | `/shared/types/src/fhe.rs` |
| Solana Validation | ✅ Implemented | `/programs/zyberlink/src/processor/create_job.rs` |
| Backend API | ✅ Implemented | `/blink-server/src/api_handlers.rs` (lines 342-432) |
| Frontend UI | ✅ Implemented | `/frontend/src/lib/components/CreateJob.svelte` |
| SDK Builders | ✅ Implemented | `/sdk/src/instructions/marketplace.rs` |
| ROI Calculator | ✅ Implemented | `/prover-node/src/roi_calculator.rs` |
| Unit Tests | ✅ 57 tests passing | `/shared/types/src/fhe.rs` |
| Integration Tests | ✅ 4 tests passing | `/programs/zyberlink/tests/dynamic_pricing_tests.rs` |
| E2E Script | ✅ Implemented | `/test-dynamic-pricing-e2e.sh` |

## Testing

### Run All Tests

```bash
# Unit tests (shared types)
cd /home/deploy/experimental/zyberlink-demo/shared/types
cargo test --lib

# Program integration tests
cd /home/deploy/experimental/zyberlink-demo/programs/zyberlink
cargo test-sbf

# ROI calculator tests
cd /home/deploy/experimental/zyberlink-demo/prover-node
cargo test roi_calculator::tests

# E2E test (requires running backend)
cd /home/deploy/experimental/zyberlink-demo
./test-dynamic-pricing-e2e.sh
```

### Test Coverage

- **57 unit tests** in `shared/types` covering all 5 tiers
- **4 integration tests** validating on-chain pricing enforcement
- **7 E2E tests** verifying API cost estimation
- **4 ROI calculator tests** ensuring profitability calculations

## Getting Started

### For Job Creators

1. **Estimate costs** before creating jobs:
   ```bash
   curl -X POST http://localhost:8080/api/estimate-cost \
     -H "Content-Type: application/json" \
     -d '{"operation": "add", "operation_value": 5, "required_provers": 3}'
   ```

2. **Use SDK** with validated pricing:
   ```rust
   let operation = FheOperation::Add(5);
   let cost_config = operation.get_cost_config();
   let total_price = cost_config.min_payment_lamports * 3;  // 3 provers

   let ix = builder.create_fhe_job(
       creator,
       job_id,
       &encrypted_data,
       fhe_config,
       total_price,
       cost_config.timeout_seconds
   )?;
   ```

3. **Frontend integration** - Use the CreateJob component with real-time estimation

### For Provers

1. **Configure ROI thresholds** in prover node:
   ```rust
   let calculator = ROICalculator::new(
       15.0,   // 15% minimum ROI
       1.2     // 20% operational overhead
   );
   ```

2. **Evaluate jobs** before claiming:
   ```rust
   let roi = calculator.evaluate_job(circuit_type, price, provers);
   if !roi.is_profitable {
       log::warn!("Job not profitable, skipping");
       continue;
   }
   ```

## Security Guarantees

1. **Protocol-level enforcement** - Underpriced jobs rejected on-chain
2. **Front-running protection** - Minimum prices are deterministic
3. **Prover protection** - ROI calculator prevents unprofitable work
4. **DoS mitigation** - Expensive operations require proportional payment

## Roadmap

### Phase 7 (Future)
- [ ] Dynamic tier adjustment based on network load
- [ ] Historical pricing analytics dashboard
- [ ] Prover bidding system for premium rates
- [ ] Multi-token pricing (USDC, wZEC support for dynamic pricing)

## Further Reading

- **[Technical Implementation Guide](./dynamic-pricing-technical.md)** - Deep dive into algorithms and data structures
- **[Usage Guide with Examples](./dynamic-pricing-usage.md)** - Practical integration examples
- **[API Reference](./api-reference.md)** - Complete API documentation
- **[Histogram Optimization](../architecture/histogram-fhe-optimization.md)** - Tier 5 complexity analysis

## Contributing

Found an issue with pricing? See incorrect costs? Open an issue or PR at [ZyberLink GitHub](https://github.com/yourorg/zyberlink).

---

**Last Updated:** 2025-11-21
**Version:** 1.0.0
**Status:** Production Ready
