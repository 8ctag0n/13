# Dynamic Pricing System - Usage Guide

**Practical Integration Guide with Real-World Examples**

---

## Table of Contents

1. [For Job Creators](#for-job-creators)
2. [For Provers](#for-provers)
3. [SDK Integration Examples](#sdk-integration-examples)
4. [API Integration Examples](#api-integration-examples)
5. [Frontend Integration](#frontend-integration)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

---

## For Job Creators

### Quick Start: Estimate Job Costs

Before creating a job, always estimate the cost:

```bash
# Example: Age verification (Tier 3)
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Threshold",
  "complexity_tier": 3,
  "min_payment_lamports": 5000000,
  "min_payment_sol": 0.005,
  "total_min_payment_lamports": 15000000,
  "total_min_payment_sol": 0.015,
  "timeout_seconds": 300,
  "estimated_compute_ms": 300
}
```

### Use Case Examples

#### 1. Age Verification for ZK-Passport

**Scenario:** Verify a user is over 18 without revealing their actual age.

**Operation:** `Threshold`

```rust
use zyberlink_types::fhe::{FheOperation, FheConsensusConfig};
use zyberlink_sdk::instructions::InstructionBuilder;

// Setup
let operation = FheOperation::Threshold {
    threshold: 18,
    greater_or_equal: true,
};

// Get pricing
let cost_config = operation.get_cost_config();
println!("Operation: {}", operation.name());
println!("Tier: {}", cost_config.complexity_tier);
println!("Cost per prover: {} SOL", cost_config.min_payment_lamports as f64 / 1_000_000_000.0);
println!("Timeout: {}s", cost_config.timeout_seconds);

// Create FHE config
let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

// Calculate total price
let total_price = cost_config.min_payment_lamports * 3; // 3 provers = 15,000,000 lamports (0.015 SOL)

// Build instruction
let builder = InstructionBuilder::new(program_id);
let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_age_data,
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;

// Sign and send transaction
```

**Expected Cost:** 0.015 SOL (0.005 SOL × 3 provers)
**Expected Time:** ~5 minutes

#### 2. Private Census Count (Tier 2)

**Scenario:** Count the number of people in a region without revealing individual data.

**Operation:** `Sum`

```rust
// For 1,000 people
let operation = FheOperation::Sum {
    expected_count: 1000,
};

let cost_config = operation.get_cost_config();

// Cost calculation:
// Base: 0.001 SOL
// Per-item: 1000 × 0.0001 SOL = 0.1 SOL
// Total per prover: 0.101 SOL
// 3 provers: 0.303 SOL

let total_price = cost_config.min_payment_lamports * 3; // 303,000,000 lamports

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds, // 2,060 seconds
    operation: operation.clone(),
};

let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_census_data, // Array of 1,000 encrypted values
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;
```

**Expected Cost:** 0.303 SOL
**Expected Time:** ~34 minutes

#### 3. Private Voting (Tier 5)

**Scenario:** Conduct a vote among 5 candidates with encrypted ballots.

**Operation:** `Histogram`

```rust
use zyberlink_types::fhe::HistogramBin;

// Define 5 candidates
let operation = FheOperation::Histogram {
    bins: vec![
        HistogramBin::new(0, 0, "Candidate A"),
        HistogramBin::new(1, 1, "Candidate B"),
        HistogramBin::new(2, 2, "Candidate C"),
        HistogramBin::new(3, 3, "Candidate D"),
        HistogramBin::new(4, 4, "Candidate E"),
    ],
};

let cost_config = operation.get_cost_config();

// Cost calculation:
// Base: 0.1 SOL
// Exponential: 0.02 × 5^1.5 ≈ 0.224 SOL
// Total per prover: ~0.324 SOL
// 3 provers: ~0.972 SOL

let total_price = cost_config.min_payment_lamports * 3; // ~972,000,000 lamports

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds, // 1,100 seconds
    operation: operation.clone(),
};

let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_votes, // Encrypted vote choices (0-4)
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;
```

**Expected Cost:** ~0.972 SOL
**Expected Time:** ~18 minutes

**Result Format:**
```
[
  encrypt(count_A),  // Number of votes for Candidate A
  encrypt(count_B),  // Number of votes for Candidate B
  encrypt(count_C),
  encrypt(count_D),
  encrypt(count_E)
]
```

#### 4. Token Payment Example (wZEC)

**Scenario:** Pay for FHE job using wZEC tokens instead of SOL.

```rust
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;

let operation = FheOperation::Add(5);
let cost_config = operation.get_cost_config();

// Convert SOL price to wZEC (1 wZEC = ~$25, 1 SOL = ~$100)
// wZEC has 8 decimals (like Bitcoin)
let sol_price = cost_config.min_payment_lamports;
let wzec_price = (sol_price * 4) / 1; // Rough 4:1 ratio, adjust for real market

let wzec_mint = Pubkey::from_str("ZECpv6hqVqwz4c3c9RMz8N5SLKFx9Qz...")
    .expect("Valid wZEC mint");

let creator_token_account = get_associated_token_address(
    &creator_pubkey,
    &wzec_mint,
);

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

let ix = builder.create_fhe_job_with_token(
    creator_pubkey,
    job_id,
    &encrypted_data,
    fhe_config,
    wzec_price * 3, // Price in zatoshis (wZEC base units)
    cost_config.timeout_seconds,
    wzec_mint,
    creator_token_account,
)?;
```

---

## For Provers

### Quick Start: Evaluate Job Profitability

Before claiming a job, check if it's profitable:

```rust
use zyberlink_prover::roi_calculator::ROICalculator;

// Configure your ROI requirements
let calculator = ROICalculator::new(
    15.0,   // 15% minimum ROI
    1.2     // 20% operational overhead (electricity, hardware wear)
);

// Evaluate a job
let job_roi = calculator.evaluate_job(
    &job.circuit_type,
    job.price_lamports,
    job.fhe_config.required_provers,
);

if job_roi.is_profitable {
    println!("✅ PROFITABLE JOB");
    println!("   Revenue per prover: {} SOL", job_roi.revenue_per_prover as f64 / 1e9);
    println!("   Estimated cost: {} SOL", job_roi.estimated_cost as f64 / 1e9);
    println!("   Profit: {} SOL", job_roi.profit as f64 / 1e9);
    println!("   ROI: {:.1}%", job_roi.roi_percentage);
    println!("   Tier: {}", job_roi.complexity_tier);

    // Claim the job
    claim_job(&job)?;
} else {
    println!("❌ NOT PROFITABLE");
    println!("   Expected ROI: {:.1}% (min: 15%)", job_roi.roi_percentage);
    println!("   Skip this job");
}
```

### Prover Configuration Examples

#### 1. Conservative Prover (High Margin)

```rust
// Only accept very profitable jobs
let conservative_calculator = ROICalculator::new(
    50.0,   // 50% minimum ROI
    2.0     // 100% overhead (2x safety margin)
);

// This prover will only accept jobs paying at least 3x the base cost
```

**Use Case:** Small operations, low risk tolerance, high electricity costs

#### 2. Aggressive Prover (Low Margin)

```rust
// Accept most jobs above cost
let aggressive_calculator = ROICalculator::new(
    10.0,   // 10% minimum ROI
    1.1     // 10% overhead (tight margins)
);

// This prover accepts more jobs but requires efficient operations
```

**Use Case:** High-performance hardware, low electricity costs, volume strategy

#### 3. Tier-Specific Strategy

```rust
// Accept different margins for different tiers
fn should_accept_job(job: &Job) -> bool {
    let base_calculator = ROICalculator::new(20.0, 1.2);
    let roi = base_calculator.evaluate_job(
        &job.circuit_type,
        job.price_lamports,
        job.fhe_config.required_provers,
    );

    match roi.complexity_tier {
        1 | 2 => {
            // Accept Tier 1-2 if profitable at all
            roi.roi_percentage > 5.0
        }
        3 | 4 => {
            // Standard margin for Tier 3-4
            roi.roi_percentage > 15.0
        }
        5 => {
            // Higher margin for expensive Tier 5
            roi.roi_percentage > 30.0
        }
        _ => false,
    }
}
```

### Minimum Price Calculation

Get the absolute minimum price you'd accept:

```rust
let calculator = ROICalculator::new(20.0, 1.5);

let min_acceptable = calculator.get_minimum_price(
    &CircuitType::FheComputation(FheOperation::Threshold {
        threshold: 18,
        greater_or_equal: true,
    }),
    3, // 3 provers
);

println!("Minimum acceptable price: {} SOL", min_acceptable as f64 / 1e9);
// Output: Minimum acceptable price: 0.027 SOL
// (0.005 × 1.5 × 1.2 × 3 = 0.027)
```

---

## SDK Integration Examples

### Example 1: Simple Addition Job

```rust
use zyberlink_sdk::instructions::InstructionBuilder;
use zyberlink_types::fhe::{FheOperation, FheConsensusConfig};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;

// Setup
let rpc_url = "https://api.devnet.solana.com";
let client = RpcClient::new(rpc_url);
let program_id = Pubkey::from_str("YOUR_PROGRAM_ID")?;
let creator = Keypair::new();
let builder = InstructionBuilder::new(program_id);

// Define operation
let operation = FheOperation::Add(10);
let cost_config = operation.get_cost_config();

// Create FHE config
let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

// Prepare encrypted data (this would come from tfhe-rs encryption)
let encrypted_data = vec![/* encrypted value */];

// Calculate price
let total_price = cost_config.min_payment_lamports * 3;

// Build instruction
let ix = builder.create_fhe_job(
    creator.pubkey(),
    1, // job_id
    &encrypted_data,
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;

// Create and send transaction
let recent_blockhash = client.get_latest_blockhash()?;
let tx = Transaction::new_signed_with_payer(
    &[ix],
    Some(&creator.pubkey()),
    &[&creator],
    recent_blockhash,
);

let signature = client.send_and_confirm_transaction(&tx)?;
println!("Job created: {}", signature);
```

### Example 2: Batch Job Creation

```rust
// Create multiple jobs with different operations
let operations = vec![
    FheOperation::Add(5),
    FheOperation::Multiply(3),
    FheOperation::Threshold { threshold: 18, greater_or_equal: true },
];

for (idx, operation) in operations.iter().enumerate() {
    let cost_config = operation.get_cost_config();
    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: cost_config.timeout_seconds,
        operation: operation.clone(),
    };

    let total_price = cost_config.min_payment_lamports * 3;

    let ix = builder.create_fhe_job(
        creator.pubkey(),
        idx as u64,
        &encrypted_data[idx],
        fhe_config,
        total_price,
        cost_config.timeout_seconds,
    )?;

    // Create and send transaction
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&creator.pubkey()),
        &[&creator],
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx)?;
    println!("Job {} created ({}): {}", idx, operation.name(), sig);
}
```

---

## API Integration Examples

### Example 1: JavaScript/TypeScript Frontend

```typescript
// estimate-cost.ts
interface EstimateCostRequest {
  operation: string;
  operation_value: number;
  expected_count?: number;
  bins?: number;
  required_provers: number;
}

interface EstimateCostResponse {
  operation: string;
  complexity_tier: number;
  min_payment_lamports: number;
  min_payment_sol: number;
  total_min_payment_lamports: number;
  total_min_payment_sol: number;
  timeout_seconds: number;
  estimated_compute_ms: number;
}

async function estimateJobCost(
  operation: string,
  params: Partial<EstimateCostRequest>
): Promise<EstimateCostResponse> {
  const request: EstimateCostRequest = {
    operation,
    operation_value: params.operation_value || 0,
    required_provers: params.required_provers || 3,
    ...params,
  };

  const response = await fetch('http://localhost:8080/api/estimate-cost', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Cost estimation failed: ${response.statusText}`);
  }

  return await response.json();
}

// Usage examples
async function main() {
  // Example 1: Simple addition
  const addCost = await estimateJobCost('add', {
    operation_value: 5,
    required_provers: 3,
  });
  console.log(`Add operation: ${addCost.total_min_payment_sol} SOL`);

  // Example 2: Histogram
  const histCost = await estimateJobCost('histogram', {
    bins: 5,
    required_provers: 3,
  });
  console.log(`Histogram (5 bins): ${histCost.total_min_payment_sol} SOL`);
  console.log(`Estimated time: ${histCost.timeout_seconds}s`);

  // Example 3: Census sum
  const sumCost = await estimateJobCost('sum', {
    expected_count: 10000,
    required_provers: 5,
  });
  console.log(`Sum (10k items, 5 provers): ${sumCost.total_min_payment_sol} SOL`);
}
```

### Example 2: Python Backend

```python
import requests
from typing import Optional, Dict

class ZyberLinkPricingClient:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url

    def estimate_cost(
        self,
        operation: str,
        operation_value: int = 0,
        expected_count: Optional[int] = None,
        bins: Optional[int] = None,
        required_provers: int = 3
    ) -> Dict:
        """Estimate cost for FHE operation."""
        payload = {
            "operation": operation,
            "operation_value": operation_value,
            "required_provers": required_provers,
        }

        if expected_count is not None:
            payload["expected_count"] = expected_count
        if bins is not None:
            payload["bins"] = bins

        response = requests.post(
            f"{self.base_url}/api/estimate-cost",
            json=payload
        )
        response.raise_for_status()
        return response.json()

    def calculate_job_budget(
        self,
        operations: list[tuple[str, dict]]
    ) -> Dict:
        """Calculate total budget for multiple operations."""
        total_lamports = 0
        job_estimates = []

        for op_name, params in operations:
            estimate = self.estimate_cost(op_name, **params)
            total_lamports += estimate["total_min_payment_lamports"]
            job_estimates.append(estimate)

        return {
            "total_lamports": total_lamports,
            "total_sol": total_lamports / 1e9,
            "jobs": job_estimates,
        }

# Usage
if __name__ == "__main__":
    client = ZyberLinkPricingClient()

    # Single job estimate
    cost = client.estimate_cost(
        operation="threshold",
        operation_value=18,
        required_provers=3
    )
    print(f"Age verification cost: {cost['total_min_payment_sol']} SOL")

    # Multiple jobs budget
    budget = client.calculate_job_budget([
        ("add", {"operation_value": 5, "required_provers": 3}),
        ("multiply", {"operation_value": 10, "required_provers": 3}),
        ("histogram", {"bins": 5, "required_provers": 3}),
    ])
    print(f"Total budget: {budget['total_sol']} SOL")
```

---

## Frontend Integration

### Svelte Component Example

```svelte
<!-- CreateJobForm.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';

  type Operation = 'add' | 'multiply' | 'sum' | 'threshold' | 'histogram';

  let selectedOperation: Operation = 'add';
  let operationValue = 5;
  let expectedCount = 100;
  let bins = 5;
  let requiredProvers = 3;

  let costEstimate: any = null;
  let isEstimating = false;

  async function estimateCost() {
    isEstimating = true;

    const params: any = {
      operation: selectedOperation,
      operation_value: operationValue,
      required_provers: requiredProvers,
    };

    if (['sum', 'average', 'count_if'].includes(selectedOperation)) {
      params.expected_count = expectedCount;
    }

    if (selectedOperation === 'histogram') {
      params.bins = bins;
    }

    try {
      const response = await fetch('/api/estimate-cost', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
      });

      costEstimate = await response.json();
    } catch (error) {
      console.error('Cost estimation failed:', error);
    } finally {
      isEstimating = false;
    }
  }

  // Re-estimate on parameter change
  $: if (selectedOperation || operationValue || expectedCount || bins || requiredProvers) {
    estimateCost();
  }
</script>

<div class="job-form">
  <h2>Create FHE Job</h2>

  <label>
    Operation:
    <select bind:value={selectedOperation}>
      <option value="add">Add (Tier 1)</option>
      <option value="multiply">Multiply (Tier 1)</option>
      <option value="sum">Sum (Tier 2)</option>
      <option value="threshold">Threshold (Tier 3)</option>
      <option value="histogram">Histogram (Tier 5)</option>
    </select>
  </label>

  {#if ['add', 'multiply', 'threshold'].includes(selectedOperation)}
    <label>
      Value:
      <input type="number" bind:value={operationValue} min="0" max="255" />
    </label>
  {/if}

  {#if ['sum', 'average'].includes(selectedOperation)}
    <label>
      Expected Count:
      <input type="number" bind:value={expectedCount} min="1" max="10000" />
    </label>
  {/if}

  {#if selectedOperation === 'histogram'}
    <label>
      Number of Bins:
      <input type="number" bind:value={bins} min="2" max="50" />
      {#if bins > 10}
        <span class="warning">⚠️ High bin count = expensive operation</span>
      {/if}
    </label>
  {/if}

  <label>
    Required Provers:
    <input type="range" bind:value={requiredProvers} min="2" max="10" />
    <span>{requiredProvers} provers</span>
  </label>

  {#if costEstimate && !isEstimating}
    <div class="cost-display">
      <h3>Cost Estimate</h3>
      <div class="cost-details">
        <div class="tier">Tier {costEstimate.complexity_tier}</div>
        <div class="price">
          <strong>{costEstimate.total_min_payment_sol.toFixed(6)} SOL</strong>
          <small>({costEstimate.total_min_payment_lamports.toLocaleString()} lamports)</small>
        </div>
        <div class="breakdown">
          {costEstimate.min_payment_sol.toFixed(6)} SOL × {requiredProvers} provers
        </div>
        <div class="timeout">
          Timeout: {costEstimate.timeout_seconds}s
          ({Math.floor(costEstimate.timeout_seconds / 60)} min)
        </div>
      </div>

      {#if costEstimate.total_min_payment_sol > 1.0}
        <div class="warning">
          ⚠️ This is an expensive operation. Consider reducing provers or bins.
        </div>
      {/if}
    </div>
  {/if}

  <button on:click={createJob} disabled={isEstimating || !costEstimate}>
    Create Job
  </button>
</div>

<style>
  .warning { color: orange; font-size: 0.9em; }
  .cost-display {
    background: #f5f5f5;
    padding: 1rem;
    border-radius: 8px;
    margin: 1rem 0;
  }
  .tier {
    display: inline-block;
    background: #007bff;
    color: white;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8em;
  }
  .price strong { font-size: 1.5em; }
  .breakdown { color: #666; font-size: 0.9em; }
</style>
```

---

## Best Practices

### 1. Always Estimate Before Creating Jobs

```rust
// ❌ DON'T: Create job without checking price
let ix = builder.create_fhe_job(creator, job_id, data, config, 1_000_000, 60)?;
// This will likely fail on-chain

// ✅ DO: Calculate minimum price first
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;
let ix = builder.create_fhe_job(creator, job_id, data, config, min_price, cost_config.timeout_seconds)?;
```

### 2. Use Dynamic Timeouts

```rust
// ❌ DON'T: Use hardcoded timeouts
let ix = builder.create_fhe_job(creator, job_id, data, config, price, 300)?;

// ✅ DO: Use dynamic timeout from cost config
let cost_config = operation.get_cost_config();
let ix = builder.create_fhe_job(creator, job_id, data, config, price, cost_config.timeout_seconds)?;
```

### 3. Warn Users About Expensive Operations

```typescript
// In your frontend
if (costEstimate.complexity_tier >= 5) {
  alert(`Warning: Tier ${costEstimate.complexity_tier} operations are expensive. ` +
        `This job will cost ${costEstimate.total_min_payment_sol} SOL.`);
}
```

### 4. Batch Similar Operations

```rust
// ❌ DON'T: Create separate jobs for similar operations
for value in vec![5, 10, 15, 20] {
    create_job(FheOperation::Add(value))?; // 4 jobs = 4× cost
}

// ✅ DO: Process multiple values in one job if possible
// Or at least warn users about total cost
let total_cost = estimate_cost(Add(5)) * 4;
println!("Creating 4 jobs will cost {} SOL", total_cost);
```

### 5. Handle Prover Count Changes

```typescript
// Recalculate cost when prover count changes
proverCountSlider.addEventListener('change', async (e) => {
  const newProverCount = e.target.value;
  const newEstimate = await estimateCost({
    ...currentParams,
    required_provers: newProverCount
  });
  updateCostDisplay(newEstimate);
});
```

---

## Troubleshooting

### Error: InvalidPrice on Job Creation

**Symptom:**
```
Transaction failed: InvalidPrice
Price too low for FHE operation 'Threshold' (tier 3): 5000000 < 15000000
```

**Cause:** Job price is below the minimum required for the operation and number of provers.

**Solution:**
```rust
// Get minimum price from cost config
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;

// Use minimum or higher
let ix = builder.create_fhe_job(..., min_price, ...)?;
```

### Cost Estimate Returns Error

**Symptom:**
```json
{"error": "Unknown operation: threshhold"}
```

**Cause:** Typo in operation name or unsupported operation.

**Solution:** Use exact operation names:
- `"add"`, `"multiply"`, `"sum"`, `"threshold"`, `"range_check"`, `"average"`, `"count_if"`, `"histogram"`

### Job Times Out Before Completion

**Symptom:** Job expires before provers can submit results.

**Cause:** Using hardcoded timeout that's too short for the operation.

**Solution:**
```rust
// Use dynamic timeout
let cost_config = operation.get_cost_config();
let ix = builder.create_fhe_job(
    creator,
    job_id,
    data,
    config,
    price,
    cost_config.timeout_seconds  // ✅ Use this
)?;
```

### Prover Rejects All Jobs

**Symptom:** Prover node logs show all jobs as "not profitable".

**Cause:** ROI thresholds too high or operational cost multiplier too high.

**Solution:**
```rust
// Adjust ROI calculator parameters
let calculator = ROICalculator::new(
    10.0,  // Lower min ROI (was 50%)
    1.2    // Lower overhead (was 2.0)
);
```

---

## FAQ

### Q: Can I pay more than the minimum price?

**A:** Yes! Paying above minimum can:
- Incentivize faster prover response
- Act as a priority fee
- Compensate for market fluctuations

```rust
let min_price = cost_config.min_payment_lamports * 3;
let premium_price = min_price + 5_000_000; // +0.005 SOL bonus
let ix = builder.create_fhe_job(..., premium_price, ...)?;
```

### Q: What happens if SOL price changes?

**A:** Dynamic pricing is denominated in lamports (on-chain currency), not USD. If SOL price doubles:
- Operation still costs same lamports
- But USD cost doubles
- Future: Consider implementing USD-pegged stablecoin pricing

### Q: Can I use custom timeout values?

**A:** Yes, but must be >= dynamic timeout:

```rust
let cost_config = operation.get_cost_config();
let custom_timeout = cost_config.timeout_seconds * 2; // 2x buffer
let ix = builder.create_fhe_job(..., price, custom_timeout)?;
```

### Q: How do I know which tier an operation is?

**A:**
```rust
let operation = FheOperation::Histogram { bins: vec![...] };
let config = operation.get_cost_config();
println!("Tier: {}", config.complexity_tier);
```

Or check the [main documentation](./DYNAMIC_PRICING.md#complexity-tiers).

### Q: Are there bulk discounts for multiple jobs?

**A:** Not currently. Each job pays per-prover cost. Future enhancement could implement:
- Volume discounts
- Subscription models
- Stake-based discounts

### Q: Can provers negotiate prices?

**A:** Not in the current implementation. Prices are deterministic based on operation complexity. Future could implement:
- Prover bidding system
- Premium/budget tiers
- Dutch auction mechanics

---

## Next Steps

- **[Technical Deep Dive](./DYNAMIC_PRICING_TECHNICAL.md)** - Understand the algorithms
- **[API Reference](./API_REFERENCE.md)** - Complete API documentation
- **[Main Documentation](./DYNAMIC_PRICING.md)** - System overview

---

**Last Updated:** 2025-11-21
**Questions?** Open an issue on GitHub or ask in Discord
