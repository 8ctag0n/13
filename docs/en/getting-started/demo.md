# Interactive Demo

Experience ZyberLink's multi-prover consensus in action.

## Overview

The ZyberLink demo showcases a complete multi-prover FHE computation workflow:

1. **Client** creates encrypted computation jobs
2. **Multiple provers** compete to process jobs
3. **Consensus** ensures correctness
4. **Payments** are automatically distributed

**Duration:** Self-paced (typically 5-10 minutes)

## Prerequisites

Complete the [Quickstart guide](quickstart.md) first to have:
- Local Solana validator running
- ZyberLink program deployed
- Project built successfully

## Running the Demo

### Option 1: Automated Demo (Recommended)

The easiest way to see everything in action:

```bash
cd zyb-platform
./demo.sh
```

This script automatically:
- Configures the environment
- Starts 3 prover nodes
- Launches the TUI interface
- Begins generating FHE jobs
- Shows real-time processing

**What you'll see:**

```
┌─────────────────────────────────────────────────────────────┐
│ ZyberLink Prover Node - Multi-Prover Demo                  │
├─────────────────────────────────────────────────────────────┤
│ Status: ACTIVE          Jobs: 12       Earnings: 0.024 SOL │
│                                                             │
│ Recent Jobs:                                                │
│  Job #234abc  Claimed   Computing...    [████░░░░░] 45%    │
│  Job #233def  Complete  Verified ✓      Earned: 0.002 SOL  │
│  Job #232ghi  Complete  Verified ✓      Earned: 0.002 SOL  │
│                                                             │
│ Prover Network:                                             │
│  Prover A (You):    12 jobs    97% uptime   ⭐ 4.9        │
│  Prover B:          11 jobs    98% uptime   ⭐ 5.0        │
│  Prover C:          12 jobs    95% uptime   ⭐ 4.8        │
└─────────────────────────────────────────────────────────────┘
Press 'q' to quit
```

### Option 2: Manual Control

For more control, run components separately:

#### Terminal 1: Start Prover Node

```bash
cd prover-node
cargo run --release
```

The TUI will launch showing your prover waiting for jobs.

#### Terminal 2: Generate Jobs

In a new terminal:

```bash
cd demo
./generate-jobs.sh
```

This creates FHE computation jobs and posts them to the marketplace.

#### Terminal 3: Monitor Blockchain (Optional)

Watch on-chain activity:

```bash
solana logs
```

You'll see transactions for:
- Job creation
- Prover claims
- Result submission
- Payment distribution

## Understanding the Demo

### What's Happening

1. **Job Creation**
   - Demo script generates encrypted FHE computation requests
   - Jobs are posted to the Solana marketplace
   - Each job includes encrypted witness data

2. **Prover Competition**
   - Multiple prover nodes monitor for new jobs
   - Provers claim jobs they can handle
   - Fastest claimers get the work

3. **FHE Computation**
   - Provers execute computations using TFHE-rs
   - Operations run on encrypted data
   - Results remain encrypted until finalization

4. **Consensus Verification**
   - Multiple provers submit results
   - On-chain program verifies 2-of-3 consensus
   - Matching results are accepted
   - Dishonest provers are penalized

5. **Payment Distribution**
   - Honest provers receive payment automatically
   - Reputation scores are updated
   - Job is marked complete

### TUI Interface Explained

The Terminal User Interface shows:

**Header**
- `Status`: IDLE (waiting), ACTIVE (processing), or ERROR
- `Jobs`: Total jobs processed since startup
- `Earnings`: SOL earned from completed jobs

**Recent Jobs Section**
- Job ID and current status
- Progress bars for ongoing computations
- Completion status and earnings per job

**Prover Network Section**
- All active provers in the network
- Their stats: jobs completed, uptime, reputation
- Shows your position in the network

**Controls**
- `q` - Quit gracefully
- `r` - Refresh display
- `h` - Show help

## Demo Scenarios

### Scenario 1: Single Addition

Simplest FHE operation - add two encrypted numbers:

```bash
./demo/scenarios/single-addition.sh
```

**Expected output:**
- Job posted in ~1 second
- 3 provers claim the job
- Computation completes in ~3 seconds
- Consensus reached (3/3 agreement)
- Payment distributed

### Scenario 2: Batch Operations

Multiple FHE operations in parallel:

```bash
./demo/scenarios/batch-operations.sh
```

**Expected output:**
- 10 jobs posted simultaneously
- Provers distribute work automatically
- Jobs complete in ~5-10 seconds
- High throughput demonstration

### Scenario 3: Dishonest Prover

Simulates a malicious prover submitting wrong results:

```bash
./demo/scenarios/dishonest-prover.sh
```

**Expected output:**
- One prover submits incorrect result
- Consensus detects mismatch
- Dishonest prover is NOT paid
- Honest provers split the reward
- Reputation system penalizes bad actor

## Exploring Further

### View On-Chain State

Check job details directly on Solana:

```bash
# List all jobs
solana account <PROGRAM_ID>

# View specific job
./scripts/view-job.sh <JOB_ID>
```

### Prover Dashboard

Access detailed prover statistics:

```bash
cd prover-node
cargo run --release -- stats
```

Shows:
- Earnings breakdown
- Job success rate
- Average computation time
- Reputation history

### Create Custom Jobs

Submit your own FHE computations:

```bash
cd sdk
cargo run --example custom_job -- \
  --operation add \
  --input1 42 \
  --input2 58
```

## Performance Metrics

Typical demo performance on modern hardware:

| Metric | Value |
|--------|-------|
| Job claim time | < 1 second |
| FHE addition | 2-3 seconds |
| FHE multiplication | 4-6 seconds |
| Consensus verification | < 1 second |
| Payment distribution | < 1 second |
| **Total job completion** | **5-10 seconds** |

## Troubleshooting

### TUI doesn't appear

Check that the prover node is running:
```bash
ps aux | grep prover-node
```

### Jobs not being claimed

Verify the prover is registered:
```bash
solana account <YOUR_PROVER_PUBKEY>
```

### "No jobs available"

Make sure the job generator is running:
```bash
./demo/generate-jobs.sh
```

### Consensus failures

Check that you have at least 2 provers running for 2-of-3 consensus:
```bash
./demo/run-demo.sh  # Starts 3 provers automatically
```

## What's Next?

Now that you've seen ZyberLink in action:

- **[Architecture Overview](../architecture/overview.md)** - Understand the technical design
- **[FHE Design](../architecture/fhe-design.md)** - Deep dive into encryption
- **[Deployment Guide](../guides/deployment.md)** - Run in production

## Demo Tips

**For presentations:**
- Use the automated demo script for reliability
- Pre-record a backup video in case of issues
- Focus on the real-time TUI updates
- Show the Solana Explorer alongside for transparency

**For development:**
- Run manual mode to test specific scenarios
- Use `solana logs` to debug issues
- Modify job parameters in `demo/generate-jobs.sh`
- Experiment with different consensus thresholds

---

Enjoy exploring ZyberLink! The demo showcases the core value proposition: **decentralized, verifiable, privacy-preserving computation at scale**.
