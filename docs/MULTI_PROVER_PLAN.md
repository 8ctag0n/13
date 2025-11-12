# Multi-Prover Competition - Implementation Plan

## Current State Analysis

### ✅ What Already Works

1. **Prover Registration** - Multiple provers can register independently
2. **Individual Stake** - Each prover has their own stake account
3. **Reputation System** - Provers have reputation scores
4. **Job Status** - Jobs track which prover claimed them
5. **First-Come-First-Served** - ClaimJob is atomic (first to claim wins)

### 🔍 What Needs Testing/Improvement

1. **Race Conditions** - Multiple provers trying to claim same job simultaneously
2. **Job Distribution** - Fair distribution among multiple provers
3. **Prover Selection** - Should we prefer high-reputation provers?
4. **Load Balancing** - Prevent one prover from monopolizing all jobs
5. **Concurrent Processing** - Multiple provers working on different jobs
6. **Prover Discovery** - Efficient way to find available provers

## Implementation Phases

### Phase 1: Concurrency Testing (Current Sprint)

**Goal**: Verify the system handles multiple provers competing for jobs without race conditions.

**Tasks**:
- [ ] Test: Multiple provers racing to claim single job
- [ ] Test: Multiple provers processing different jobs simultaneously
- [ ] Test: Job queue with N provers and M jobs (N > M, N < M, N = M)
- [ ] Test: Prover node pool (3-5 provers running concurrently)
- [ ] Analyze: ClaimJob instruction for race condition safety
- [ ] Verify: No double-claiming possible
- [ ] Verify: No job left unclaimed when provers available

**Files to Create**:
```
e2e-tests/tests/
  └── test_multi_prover_competition.rs  (race condition tests)
  └── test_prover_pool.rs               (concurrent prover pool)
  └── test_job_distribution.rs          (fair distribution)
```

---

### Phase 2: Prover Selection Strategy (Next Sprint)

**Goal**: Implement intelligent prover selection beyond first-come-first-served.

**Current Behavior**:
- Any prover can claim any pending job
- First transaction to hit the chain wins

**Proposed Strategies** (pick one or make configurable):

#### Option A: First-Come-First-Served (Current)
✅ **Pros**: Simple, fair, no centralization
❌ **Cons**: No quality control, no incentive to build reputation

#### Option B: Reputation-Weighted Selection
- Jobs can specify `min_reputation_score` requirement
- Higher reputation provers get priority (time window)
- Falls back to FCFS after priority window

✅ **Pros**: Incentivizes good behavior, quality control
❌ **Cons**: More complex, may disadvantage new provers

#### Option C: Stake-Weighted Selection
- Jobs can specify `min_stake_amount` requirement
- Higher stake provers get priority (time window)
- Falls back to FCFS after priority window

✅ **Pros**: Economic security, sybil resistance
❌ **Cons**: May centralize to wealthy provers

#### Option D: Hybrid (Reputation + Stake)
- Combine reputation and stake into a "prover score"
- Priority window based on score
- Falls back to FCFS

✅ **Pros**: Best of both worlds
❌ **Cons**: Most complex

**Recommendation**: Start with **Option A** (current), add **Option B** as optional enhancement.

**Implementation**:
```rust
// Job creation with prover requirements
pub struct JobRequirements {
    pub min_reputation_score: Option<u32>,
    pub min_stake_amount: Option<u64>,
    pub preferred_provers: Option<Vec<Pubkey>>, // Whitelist
    pub priority_window_seconds: i64,           // Time before FCFS
}
```

---

### Phase 3: Job Discovery & Matchmaking (Future)

**Goal**: Efficient job discovery and prover-job matching.

**Current Approach**:
- Provers poll all accounts with `getProgramAccounts`
- Filter jobs by status
- O(N) where N = total jobs

**Proposed Improvements**:

1. **Indexed Job Queue** (Off-chain)
   - Redis/PostgreSQL maintaining pending job list
   - Provers subscribe via WebSocket
   - Push notifications when new jobs arrive

2. **Job Categories** (On-chain)
   - Circuit-specific job PDAs: `[b"job", b"zcash-orchard", &job_id]`
   - Provers specialize in specific circuits
   - More efficient scanning

3. **Prover Marketplace** (Off-chain)
   - Provers advertise capabilities, prices
   - Clients select provers before creating job
   - Job created with `preferred_prover` field

---

## Testing Strategy

### Test Scenarios

#### 1. Race Condition Tests
```rust
// Scenario: 3 provers, 1 job
// Expected: Only 1 claims successfully, others fail gracefully
test_three_provers_one_job()

// Scenario: 5 provers all send ClaimJob at same time
// Expected: Exactly 1 succeeds, 4 fail with "already claimed"
test_simultaneous_claim_attempts()
```

#### 2. Concurrent Processing Tests
```rust
// Scenario: 3 provers, 5 jobs
// Expected: All 3 provers work simultaneously, all 5 jobs complete
test_concurrent_job_processing()

// Scenario: 10 provers, 3 jobs
// Expected: 3 provers claim jobs, 7 remain idle, no race conditions
test_more_provers_than_jobs()
```

#### 3. Load Distribution Tests
```rust
// Scenario: 2 provers, 10 jobs arrive sequentially
// Expected: ~5 jobs per prover (fair distribution)
test_sequential_job_distribution()

// Scenario: 3 provers with different speeds (fast, medium, slow)
// Expected: Fast prover gets more jobs (natural load balancing)
test_performance_based_distribution()
```

#### 4. Prover Pool Tests
```rust
// Scenario: Start 5 prover nodes, create 20 jobs over 60 seconds
// Expected: All jobs completed, no crashes, logs show distribution
test_real_prover_pool()
```

---

## Race Condition Analysis

### ClaimJob Instruction Safety

**Current Implementation** (from `programs/cypherlink/src/processor/claim_job.rs`):

```rust
// 1. Load job account
let mut job: JobAccount = borsh::from_slice(&job_info.data.borrow())?;

// 2. Check job is Pending
if job.status != JobStatus::Pending {
    msg!("Job is not pending");
    return Err(CypherLinkProgramError::JobNotPending.into());
}

// 3. Validate prover (stake, reputation, active)
// ...

// 4. Update job status
job.status = JobStatus::Claimed;
job.prover = Some(*prover_authority_info.key);
job.claimed_at = Some(current_time);

// 5. Serialize back to account
borsh::to_writer(&mut job_data[..], &job)?;
```

**Safety Analysis**:
- ✅ **Atomic**: Solana transactions are atomic (all-or-nothing)
- ✅ **Read-Modify-Write**: Standard pattern, Solana prevents interleaving
- ✅ **Status Check**: Only Pending jobs can be claimed
- ✅ **Single Writer**: Only one transaction can modify account at a time

**Conclusion**: ClaimJob is **race-condition safe** by design. Multiple provers racing will result in:
- First transaction: SUCCESS (claims job)
- Subsequent transactions: FAIL (job not pending)

**No changes needed** for basic safety ✓

---

## Performance Considerations

### Current Bottlenecks

1. **Job Polling** - O(N) scan of all jobs every poll interval
   - **Impact**: Moderate (acceptable for <1000 jobs)
   - **Solution**: Job indexing (Phase 3)

2. **Proof Generation** - 10-30 seconds per job (CPU-bound)
   - **Impact**: High (limits throughput)
   - **Solution**: More provers, GPU acceleration (future)

3. **On-chain Transactions** - ~400ms per transaction
   - **Impact**: Low (2-3 tx per job: claim, submit)
   - **Solution**: Transaction batching (future)

### Scaling Estimates

| Provers | Jobs/Hour | Throughput | Notes |
|---------|-----------|------------|-------|
| 1       | 3-6       | ~0.1 jobs/min | Single prover baseline |
| 3       | 9-18      | ~0.3 jobs/min | Linear scaling |
| 10      | 30-60     | ~1 job/min | Good distribution |
| 50      | 150-300   | ~5 jobs/min | Network/indexing becomes bottleneck |
| 100+    | 300-600   | ~10 jobs/min | Need indexing solution |

**Target for Phase 1**: Support 10 concurrent provers smoothly

---

## Success Criteria

### Phase 1 (Multi-Prover Testing)

- [ ] 10 provers can run concurrently without crashes
- [ ] No race conditions detected in 100+ job claims
- [ ] Fair job distribution (no prover gets >40% of jobs)
- [ ] All jobs completed when provers available
- [ ] Tests pass consistently (no flaky tests)

### Phase 2 (Prover Selection)

- [ ] Jobs can specify reputation requirements
- [ ] High-reputation provers get priority window
- [ ] Falls back to FCFS after priority
- [ ] Backward compatible (works without requirements)

### Phase 3 (Discovery)

- [ ] Job discovery <500ms (vs current ~2s polling)
- [ ] Support 100+ concurrent provers
- [ ] Push notifications for new jobs
- [ ] Circuit-specific specialization

---

## Implementation Checklist - Phase 1

### Week 1: Analysis & Planning
- [x] Create branch `feat/multi-prover-competition`
- [x] Document current state and plan
- [ ] Review ClaimJob for race conditions
- [ ] Design test scenarios
- [ ] Set up test infrastructure

### Week 2: Concurrency Tests
- [ ] Implement `test_multi_prover_competition.rs`
  - [ ] Test: 2 provers, 1 job (race)
  - [ ] Test: 3 provers, 3 jobs (concurrent)
  - [ ] Test: 5 provers, 2 jobs (more provers than jobs)
  - [ ] Test: 2 provers, 10 jobs (load distribution)
- [ ] Run tests 100+ times to catch flaky behavior
- [ ] Document any issues found

### Week 3: Prover Pool Test
- [ ] Implement `test_prover_pool.rs`
  - [ ] Spawn 5 real prover node processes
  - [ ] Create 20 jobs over 60 seconds
  - [ ] Monitor logs for errors
  - [ ] Verify all jobs completed
  - [ ] Analyze distribution statistics
- [ ] Performance profiling
- [ ] Identify bottlenecks

### Week 4: Fixes & Optimization
- [ ] Fix any race conditions found
- [ ] Optimize job polling if needed
- [ ] Add metrics/monitoring
- [ ] Update documentation
- [ ] Merge to main

---

## Open Questions

1. **Job Priority**: Should some jobs be higher priority than others?
2. **Prover Banning**: Automatic ban for repeated failures?
3. **Dynamic Pricing**: Should provers compete on price?
4. **Job Expiry**: Auto-cancel jobs after X time unclaimed?
5. **Prover Capacity**: Should provers advertise max concurrent jobs?

## Resources

- [Solana Transaction Atomicity](https://docs.solana.com/developing/programming-model/transactions)
- [Concurrent Programming Patterns](https://docs.rs/tokio/latest/tokio/sync/)
- [Load Balancing Algorithms](https://en.wikipedia.org/wiki/Load_balancing_(computing))

---

**Next Action**: Implement `test_multi_prover_competition.rs` with basic race condition tests.
