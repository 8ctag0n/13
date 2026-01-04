# Dynamic Pricing System - Technical Implementation Guide

**Deep Dive into Architecture, Algorithms, and Data Structures**

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Core Data Structures](#core-data-structures)
3. [Pricing Algorithm](#pricing-algorithm)
4. [Validation Flow](#validation-flow)
5. [Integration Points](#integration-points)
6. [Edge Cases and Limitations](#edge-cases-and-limitations)
7. [Performance Considerations](#performance-considerations)

---

## System Architecture

### Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                      LAYER 1: SHARED TYPES                      │
│                 (/shared/types/src/fhe.rs)                      │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ FheOperation enum                                       │    │
│  │  - Add(u8)                                              │    │
│  │  - Multiply(u8)                                         │    │
│  │  - Sum { expected_count }                               │    │
│  │  - Threshold { threshold, greater_or_equal }            │    │
│  │  - RangeCheck { min, max }                              │    │
│  │  - Average { expected_count }                           │    │
│  │  - CountIf { predicate, expected_count }                │    │
│  │  - Histogram { bins }                                   │    │
│  │                                                          │    │
│  │ impl FheOperation {                                      │    │
│  │     fn get_cost_config(&self) -> OperationCostConfig    │    │
│  │     fn name(&self) -> &str                              │    │
│  │     fn estimated_compute_time_ms(&self) -> u32          │    │
│  │ }                                                        │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ OperationCostConfig struct                              │    │
│  │  - min_payment_lamports: u64                            │    │
│  │  - timeout_seconds: i64                                 │    │
│  │  - complexity_tier: u8                                  │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Import
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   LAYER 2: SOLANA PROGRAM                       │
│         (/programs/zyberlink/src/processor/)                   │
│                                                                  │
│  create_job.rs (lines 98-144):                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ match circuit_type {                                    │    │
│  │   CircuitType::FheComputation(fhe_op) => {              │    │
│  │     let cost_config = fhe_op.get_cost_config();         │    │
│  │     let min = cost_config.min_payment_lamports;         │    │
│  │     let total = min * required_provers;                 │    │
│  │                                                          │    │
│  │     if price_lamports < total {                         │    │
│  │       return Err(InvalidPrice);                         │    │
│  │     }                                                    │    │
│  │   }                                                      │    │
│  │ }                                                        │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Parallel
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 3: BACKEND API                         │
│              (/blink-server/src/api_handlers.rs)                │
│                                                                  │
│  POST /api/estimate-cost (lines 342-432):                       │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ 1. Parse operation from request JSON                    │    │
│  │ 2. Construct FheOperation enum variant                  │    │
│  │ 3. Call operation.get_cost_config()                     │    │
│  │ 4. Calculate total = min × required_provers             │    │
│  │ 5. Return JSON response with cost details               │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP API
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 4: FRONTEND UI                         │
│         (/frontend/src/lib/components/CreateJob.svelte)         │
│                                                                  │
│  - Operation selector dropdown                                  │
│  - Dynamic parameter inputs (bins, thresholds, etc.)            │
│  - Prover count slider                                          │
│  - Real-time cost estimation display                            │
│  - Submit validation                                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Data Structures

### 1. OperationCostConfig

**Location:** `/shared/types/src/fhe.rs` (lines 5-17)

```rust
#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct OperationCostConfig {
    /// Minimum payment required in lamports PER PROVER
    pub min_payment_lamports: u64,

    /// Timeout for this operation in seconds
    pub timeout_seconds: i64,

    /// Complexity tier (1-5, where 5 is most complex)
    pub complexity_tier: u8,
}
```

**Key Properties:**
- **Serialization:** Supports both Borsh (on-chain) and Serde (off-chain)
- **Size:** 21 bytes (8 + 8 + 1 + padding)
- **Immutability:** Returned by value from `get_cost_config()`, preventing modification

### 2. FheOperation Enum

**Location:** `/shared/types/src/fhe.rs` (lines 62-106)

```rust
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FheOperation {
    Add(u8),
    Multiply(u8),
    Sum { expected_count: u16 },
    Threshold { threshold: u8, greater_or_equal: bool },
    RangeCheck { min: u8, max: u8 },
    Average { expected_count: u16 },
    CountIf { predicate: FhePredicate, expected_count: u16 },
    Histogram { bins: Vec<HistogramBin> },
}
```

**Variant Size Analysis:**

| Variant | Discriminant | Data | Total Size |
|---------|--------------|------|------------|
| Add | 1 byte | 1 byte | 2 bytes |
| Multiply | 1 byte | 1 byte | 2 bytes |
| Sum | 1 byte | 2 bytes | 3 bytes |
| Threshold | 1 byte | 2 bytes | 3 bytes |
| RangeCheck | 1 byte | 2 bytes | 3 bytes |
| Average | 1 byte | 2 bytes | 3 bytes |
| CountIf | 1 byte | predicate + 2 | ~5 bytes |
| Histogram | 1 byte | Vec header + bins | Variable |

**Important:** Histogram is the only variant with unbounded size due to `Vec<HistogramBin>`.

---

## Pricing Algorithm

### Tier 1: O(1) Arithmetic Operations

**Operations:** `Add(u8)`, `Multiply(u8)`

**Algorithm:**
```rust
// Location: /shared/types/src/fhe.rs (lines 155-159)
FheOperation::Add(_) | FheOperation::Multiply(_) => OperationCostConfig {
    min_payment_lamports: LAMPORTS_PER_SOL / 1000,  // 0.001 SOL
    timeout_seconds: 60,
    complexity_tier: 1,
}
```

**Rationale:**
- **Constant time:** No loops, fixed number of FHE operations
- **Bootstrapping:** Minimal (1-2 bootstrap operations)
- **Memory:** O(1) ciphertext size
- **Benchmark:** ~150-200ms on AMD Ryzen 7900X

### Tier 2: O(n) Linear Aggregation

**Operations:** `Sum { expected_count }`

**Algorithm:**
```rust
// Location: /shared/types/src/fhe.rs (lines 164-174)
FheOperation::Sum { expected_count } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 1000;        // 0.001 SOL
    let per_item_cost = LAMPORTS_PER_SOL / 10000;   // 0.0001 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 60 + (n as i64 * 2),
        complexity_tier: 2,
    }
}
```

**Cost Formula:**
```
Cost(Sum, n) = 0.001 + (n × 0.0001) SOL
Timeout(Sum, n) = 60 + (n × 2) seconds
```

**Examples:**
- `Sum { expected_count: 10 }` → 0.002 SOL, 80s
- `Sum { expected_count: 100 }` → 0.011 SOL, 260s
- `Sum { expected_count: 10000 }` → 1.001 SOL, 20060s

**Rationale:**
- Linear cost scaling with input size
- Base cost covers setup + result extraction
- Per-item cost reflects homomorphic addition

### Tier 3: O(1) + Bootstrap Operations

**Operations:** `Threshold`, `RangeCheck`

**Algorithm:**
```rust
// Location: /shared/types/src/fhe.rs (lines 178-184)
FheOperation::Threshold { .. } | FheOperation::RangeCheck { .. } => {
    OperationCostConfig {
        min_payment_lamports: LAMPORTS_PER_SOL / 200,  // 0.005 SOL
        timeout_seconds: 300,  // 5 minutes
        complexity_tier: 3,
    }
}
```

**Rationale:**
- **Expensive bootstrapping:** Comparison operations require multiple bootstraps
- **Constant input size:** Threshold is single value comparison
- **Higher timeout:** Bootstrap operations are slower (~300ms each)
- **Use case:** Age verification, value bounds checking

### Tier 4: O(n) + Predicates/Division

**Operations:** `Average { expected_count }`, `CountIf { predicate, expected_count }`

#### Average Algorithm:
```rust
// Location: /shared/types/src/fhe.rs (lines 187-197)
FheOperation::Average { expected_count } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 1000;        // 0.001 SOL
    let per_item_cost = LAMPORTS_PER_SOL / 5000;    // 0.0002 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 60 + (n as i64 * 3),
        complexity_tier: 4,
    }
}
```

**Cost Formula:**
```
Cost(Average, n) = 0.001 + (n × 0.0002) SOL
Timeout(Average, n) = 60 + (n × 3) seconds
```

#### CountIf Algorithm:
```rust
// Location: /shared/types/src/fhe.rs (lines 199-209)
FheOperation::CountIf { expected_count, .. } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 200;         // 0.005 SOL
    let per_item_cost = (LAMPORTS_PER_SOL * 3) / 10_000;  // 0.0003 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 300 + (n as i64 * 5),
        complexity_tier: 4,
    }
}
```

**Cost Formula:**
```
Cost(CountIf, n) = 0.005 + (n × 0.0003) SOL
Timeout(CountIf, n) = 300 + (n × 5) seconds
```

**Rationale:**
- Higher per-item cost than Tier 2 due to predicate evaluation
- CountIf requires comparison for each element
- Average requires division (approximated in FHE)

### Tier 5: O(n×m) Exponential Complexity

**Operation:** `Histogram { bins }`

**Algorithm:**
```rust
// Location: /shared/types/src/fhe.rs (lines 215-232)
FheOperation::Histogram { bins } => {
    let m = bins.len() as u64;

    // Base cost for histogram operation
    let base_cost = LAMPORTS_PER_SOL / 10;  // 0.1 SOL

    // Exponential cost scales with bins^1.5
    // Using m^1.5 = sqrt(m^3) for integer math
    let m_cubed = m * m * m;
    let m_power_1_5 = (m_cubed as f64).sqrt() as u64;
    let exponential_cost = (LAMPORTS_PER_SOL / 50) * m_power_1_5;

    OperationCostConfig {
        min_payment_lamports: base_cost + exponential_cost,
        timeout_seconds: 600 + (m as i64 * 100),
        complexity_tier: 5,
    }
}
```

**Cost Formula:**
```
Cost(Histogram, m) = 0.1 + (0.02 × m^1.5) SOL
Timeout(Histogram, m) = 600 + (m × 100) seconds
```

**Examples:**
| Bins | Cost/Prover | Timeout | Rationale |
|------|-------------|---------|-----------|
| 3 | ~0.204 SOL | 900s | Small voting (3 options) |
| 5 | ~0.324 SOL | 1100s | Medium voting (5 candidates) |
| 10 | ~0.732 SOL | 1600s | Large distribution |
| 20 | ~2.032 SOL | 2600s | Census age brackets |

**Rationale:**
- **Exponential growth:** Each bin requires comparison against every input
- **True complexity:** O(n×m) where n=inputs, m=bins
- **Sublinear scaling:** m^1.5 instead of m^2 to balance cost vs. utility
- **Real benchmark:** ~3 seconds per bin for 100 inputs

---

## Validation Flow

### On-Chain Validation (Solana Program)

**Location:** `/programs/zyberlink/src/processor/create_job.rs` (lines 97-144)

```
                     CreateJob Instruction
                              │
                              ▼
           ┌──────────────────────────────────┐
           │  Parse circuit_type from IX data │
           └──────────────────────────────────┘
                              │
                              ▼
           ┌──────────────────────────────────┐
           │  Is FheComputation?              │
           └──────────────────────────────────┘
                      │              │
                   Yes│              │No (ZK job)
                      ▼              ▼
           ┌─────────────────┐   Skip validation
           │ Extract FheOp   │
           └─────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ fhe_op.get_cost_config()         │
           │                                  │
           │ Returns:                         │
           │  - min_payment_lamports          │
           │  - timeout_seconds               │
           │  - complexity_tier               │
           └─────────────────────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ Calculate total minimum:         │
           │                                  │
           │ total_min = min_payment          │
           │           × required_provers     │
           └─────────────────────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ if price_lamports < total_min { │
           │   msg!("Price too low");         │
           │   return Err(InvalidPrice);      │
           │ }                                │
           └─────────────────────────────────┘
                      │
                   Success
                      ▼
           ┌─────────────────────────────────┐
           │ Use dynamic timeout from config  │
           │ Create job with validated price  │
           └─────────────────────────────────┘
```

**Key Code:**
```rust
// Lines 109-128
let cost_config = fhe_op.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (fhe_consensus_config.required_provers as u64);

if price_lamports < total_min_price {
    msg!(
        "Price too low for FHE operation '{}' (tier {}): {} < {} ({}×{} provers)",
        fhe_op.name(),
        cost_config.complexity_tier,
        price_lamports,
        total_min_price,
        min_price_per_prover,
        fhe_consensus_config.required_provers
    );
    return Err(ZyberLinkProgramError::InvalidPrice.into());
}
```

### Off-Chain Validation (Backend API)

**Location:** `/blink-server/src/api_handlers.rs` (lines 438-475)

```rust
// Validate pricing against dynamic cost model
let cost_config = operation.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (validated.required_provers as u64);

if validated.price_lamports < total_min_price {
    return Err(anyhow::anyhow!(
        "Price too low for operation '{}' (tier {}): {} < {} lamports",
        operation.name(),
        cost_config.complexity_tier,
        validated.price_lamports,
        total_min_price
    ));
}
```

**Why Validate Twice?**
1. **Backend validation** - Early rejection, better UX (before signing)
2. **On-chain validation** - Security guarantee (canonical truth)

---

## Integration Points

### 1. SDK Integration

**Location:** `/sdk/src/instructions/marketplace.rs` (lines 211-247)

```rust
pub fn create_fhe_job(
    &self,
    creator: Pubkey,
    job_id: u64,
    encrypted_input: &[u8],
    fhe_config: FheConsensusConfig,
    price_lamports: u64,
    timeout_seconds: i64,
) -> Result<Instruction> {
    // Note: SDK does NOT validate pricing
    // Validation happens on-chain for security

    self.create_job(
        creator,
        job_id,
        CircuitType::FheComputation(fhe_config.operation.clone()),
        witness_commitment,
        encrypted_input.len() as u32,
        price_lamports,
        timeout_seconds,
        Some(fhe_config),
    )
}
```

**Best Practice:**
```rust
// Application should calculate minimum price
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * 3; // 3 provers

// Use minimum or higher
let ix = builder.create_fhe_job(
    creator,
    job_id,
    &encrypted_data,
    fhe_config,
    min_price,  // Or higher for priority
    cost_config.timeout_seconds
)?;
```

### 2. ROI Calculator Integration

**Location:** `/prover-node/src/roi_calculator.rs` (lines 67-137)

```rust
pub fn evaluate_job(
    &self,
    circuit_type: &CircuitType,
    price_lamports: u64,
    required_provers: u8,
) -> JobROI {
    let fhe_operation = match circuit_type {
        CircuitType::FheComputation(op) => op,
        _ => return self.evaluate_simple_job(price_lamports, required_provers),
    };

    // Get cost config from operation
    let cost_config = fhe_operation.get_cost_config();

    // Calculate revenue per prover
    let revenue_per_prover = price_lamports / (required_provers as u64);

    // Calculate total cost including operational overhead
    let base_cost = cost_config.min_payment_lamports;
    let total_cost = (base_cost as f64 * self.operational_cost_multiplier) as u64;

    // Calculate profit and ROI
    let profit = revenue_per_prover as i64 - total_cost as i64;
    let roi_percentage = if total_cost > 0 {
        (profit as f64 / total_cost as f64) * 100.0
    } else {
        0.0
    };

    let is_profitable = roi_percentage >= self.min_roi_percentage;

    JobROI {
        revenue_per_prover,
        estimated_cost: total_cost,
        profit,
        roi_percentage,
        complexity_tier: cost_config.complexity_tier,
        timeout_seconds: cost_config.timeout_seconds,
        is_profitable,
    }
}
```

**Configuration:**
```rust
// Default: 20% min ROI, 50% overhead
let calculator = ROICalculator::default();

// Custom: 15% min ROI, 20% overhead
let calculator = ROICalculator::new(15.0, 1.2);
```

---

## Edge Cases and Limitations

### 1. Maximum Input Sizes

**Problem:** Linear operations (Sum, Average) scale with `expected_count`, which is `u16` (max 65,535).

**Impact:**
```rust
// Maximum Sum cost
let max_sum = FheOperation::Sum { expected_count: 65535 };
let config = max_sum.get_cost_config();
// Cost: 0.001 + (65535 × 0.0001) = 6.5545 SOL per prover
// Timeout: 60 + (65535 × 2) = 131,130 seconds (~36 hours)
```

**Mitigation:**
- Frontend should warn on counts > 10,000
- Consider splitting large operations into batches
- Future: Implement batch processing primitives

### 2. Histogram Bin Limits

**Problem:** Histogram cost grows as O(m^1.5), making large bin counts expensive.

**Impact:**
```rust
// 100 bins histogram
let large_hist = FheOperation::Histogram {
    bins: vec![HistogramBin::new(i, i+1, format!("bin{}", i)); 100],
};
let config = large_hist.get_cost_config();
// Cost: 0.1 + (0.02 × 100^1.5) = 0.1 + 20 = 20.1 SOL per prover
// Timeout: 600 + (100 × 100) = 10,600 seconds (~3 hours)
```

**Mitigation:**
- Frontend limits bins to 50 maximum
- Recommend 5-10 bins for most use cases
- See [Histogram FHE Optimization](../architecture/histogram-fhe-optimization.md) for alternatives

### 3. Price Frontrunning

**Problem:** Minimum prices are deterministic and public.

**Attack:** Malicious actor could create jobs at exact minimum price to squeeze out legitimate users.

**Mitigation:**
- Users can pay above minimum for priority
- Future: Implement priority fee mechanism
- Prover selection could factor in historical reputation

### 4. Timeout Accuracy

**Problem:** Dynamic timeouts are estimates based on benchmarks, actual execution time may vary.

**Impact:** Jobs might timeout on slower hardware.

**Mitigation:**
- Timeouts are conservative (include buffer)
- Provers should benchmark and reject unprofitable jobs
- Future: Implement hardware-class tiers

### 5. Zero-Cost Operations

**Problem:** `expected_count: 0` operations have base cost only.

```rust
let zero_sum = FheOperation::Sum { expected_count: 0 };
// Cost: 0.001 SOL (just base cost)
```

**Impact:** May be used for spam, though still requires minimum payment.

**Mitigation:**
- Frontend validates count > 0
- Program should validate expected_count > 0 (TODO)

---

## Performance Considerations

### 1. Cost Calculation Performance

**Benchmark:** `get_cost_config()` execution time

```rust
// Tier 1-4: O(1) calculation
// Time: < 100ns (negligible)

// Tier 5: O(1) with sqrt() call
// Time: ~500ns (still negligible)
```

**Conclusion:** Cost calculation is not a performance bottleneck.

### 2. On-Chain Compute Units

**Solana Compute Budget:**

```rust
// CreateJob instruction compute units
Base overhead:       ~5,000 CU
Dynamic pricing:     ~500 CU
Escrow creation:     ~10,000 CU
Account creation:    ~20,000 CU
──────────────────────────────
Total:               ~35,500 CU
```

**Limit:** 200,000 CU per transaction (well within limits)

### 3. Serialization Overhead

**Borsh Serialization:**

| Type | Serialized Size |
|------|-----------------|
| `OperationCostConfig` | 21 bytes |
| `FheOperation::Add` | 2 bytes |
| `FheOperation::Sum` | 3 bytes |
| `FheOperation::Histogram(10 bins)` | ~240 bytes |

**Network Impact:** Minimal, Histogram is largest variant but still < 1KB.

### 4. Backend API Latency

**Endpoint:** `/api/estimate-cost`

```
Request parsing:     ~50μs
Cost calculation:    ~1μs
JSON serialization:  ~100μs
──────────────────────────────
Total:               ~150μs
```

**Conclusion:** Sub-millisecond latency, negligible for UX.

---

## Testing Strategy

### Unit Tests

**Location:** `/shared/types/src/fhe.rs` (lines 598-811)

**Coverage:**
- Tier 1: 2 tests (Add, Multiply)
- Tier 2: 1 test (Sum scaling)
- Tier 3: 2 tests (Threshold, RangeCheck)
- Tier 4: 2 tests (Average, CountIf)
- Tier 5: 2 tests (Histogram small/large)
- Cross-tier: 1 test (ordering verification)
- Realistic scenarios: 1 test (census, voting, age check)

### Integration Tests

**Location:** `/programs/zyberlink/tests/dynamic_pricing_tests.rs`

**Test Cases:**
1. `test_tier1_add_sufficient_price` - Accept valid Tier 1 price
2. `test_tier1_add_insufficient_price` - Reject underpriced Tier 1
3. `test_tier3_threshold_higher_price` - Verify Tier 3 requires more
4. `test_pricing_scales_with_provers` - Linear scaling validation

### E2E Tests

**Location:** `/test-dynamic-pricing-e2e.sh`

**Test Flow:**
1. API Tier 1 estimation (Add)
2. API Tier 3 estimation (Threshold)
3. API Tier 5 estimation (Histogram)
4. Prover count scaling verification
5. Shared types unit tests
6. ROI calculator unit tests

---

## Future Improvements

### 1. Dynamic Tier Adjustment

```rust
// Adjust tiers based on network congestion
pub fn get_dynamic_cost_config(
    &self,
    network_load: f64  // 0.0 - 1.0
) -> OperationCostConfig {
    let base_config = self.get_cost_config();
    let multiplier = 1.0 + (network_load * 2.0);  // Up to 3x during peak

    OperationCostConfig {
        min_payment_lamports: (base_config.min_payment_lamports as f64 * multiplier) as u64,
        ..base_config
    }
}
```

### 2. Prover Hardware Classes

```rust
pub enum HardwareClass {
    Basic,      // 1x base price
    Standard,   // 1.5x base price, 0.7x timeout
    Premium,    // 2x base price, 0.5x timeout
}

// Provers bid with hardware class
// Jobs can specify required class
```

### 3. Historical Analytics

```sql
CREATE TABLE pricing_history (
    operation TEXT,
    tier INT,
    avg_price_lamports BIGINT,
    avg_completion_time_secs INT,
    date DATE
);

-- Track actual costs vs. estimates
-- Adjust tiers quarterly based on data
```

---

## Conclusion

The Dynamic Pricing System provides:

1. **Fair compensation** for provers based on computational cost
2. **Transparent pricing** visible to all participants
3. **Protocol-level enforcement** preventing underpricing
4. **Scalability** from simple arithmetic to complex distributions
5. **Extensibility** for future operation types and pricing models

**Next Steps:**
- Read [Usage Guide](./dynamic-pricing-usage.md) for integration examples
- Review [API Reference](./api-reference.md) for endpoint details
- See [Histogram Optimization](../architecture/histogram-fhe-optimization.md) for Tier 5 deep dive

---

**Last Updated:** 2025-11-21
**Version:** 1.0.0
**Maintained By:** ZyberLink Core Team
