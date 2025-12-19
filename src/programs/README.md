# Solana Programs Architecture

This directory contains the ZyberLink on-chain programs suite for privacy-preserving computation and prediction markets on Solana.

## Overview

ZyberLink is a decentralized network for Zero-Knowledge (ZK) proof generation, Fully Homomorphic Encryption (FHE) computation, and private prediction markets. The architecture consists of five core programs that work together to provide privacy-preserving computation and governance.

```
                         ZYBERLINK ARCHITECTURE

┌─────────────────────────────────────────────────────────────────────┐
│                           BEDROCK                                   │
│                    (Core Registry & Config)                         │
│  - Prover Registry & Staking                                        │
│  - Validator Registry (for threshold encryption)                    │
│  - Global Config (program IDs, fees)                                │
│  - Reputation Tracking                                              │
└───────────────────────────┬─────────────────────────────────────────┘
                            │
                            │ CPI (verify_prover, update_stats)
                            │
        ┌───────────────────┼──────────────────┐
        │                   │                  │
        ▼                   ▼                  ▼
┌───────────────┐  ┌────────────────┐  ┌──────────────────┐
│FHE-GENERATOR  │  │ ZK-GENERATOR   │  │ FUTARCHY-MARKETS │
│               │  │                │  │                  │
│FHE Jobs       │  │ZK Proof Jobs   │  │Prediction Markets│
│Multi-prover   │  │Single-prover   │  │Private Betting   │
│Consensus      │  │Optimistic      │◄─┤ZK Proofs for     │
│               │  │Verification    │  │bets/claims       │
└───────────────┘  └────────┬───────┘  └──────────────────┘
                            │
                            │ reads job state
                            │
                            ▼
                   ┌────────────────┐
                   │   THRESHOLD    │
                   │                │
                   │ Key Share      │
                   │ Coordination   │
                   │ (3-of-5)       │
                   └────────────────┘

DEPRECATED:
┌──────────────────────────────────────────────────────────────────┐
│  ZYBERLINK (DEPRECATED - functionality migrated to generators)   │
└──────────────────────────────────────────────────────────────────┘
```

## Program Summary

| Program | Program ID | Purpose | Key Features |
|---------|-----------|---------|--------------|
| **bedrock** | `TBD` | Core registry and configuration | Prover/validator registration, staking, reputation |
| **fhe-generator** | `TBD` | FHE computation jobs | Multi-prover consensus (2-5 provers), encrypted operations |
| **zk-generator** | `TBD` | ZK proof generation jobs | Single prover, optimistic verification, dispute mechanism |
| **futarchy-markets** | `FutMkts111...` | Prediction markets | Private bets with ZK proofs, governance integration |
| **threshold** | `Thre5Z4k...` | Threshold encryption coordinator | 3-of-5 key share distribution for witness decryption |
| **zyberlink** | `N/A` | DEPRECATED | Migrated to fhe-generator and zk-generator |

## Program Descriptions

### 1. Bedrock (Core Registry)

**Purpose**: Central registry and configuration for the entire ZyberLink network.

**Instructions**:
- `Initialize` - Set up global config with generator program IDs
- `RegisterProver` - Register a prover with minimum stake (0.1 SOL)
- `RegisterValidator` - Register a validator for threshold encryption
- `SlashProver` - Slash misbehaving prover (called via CPI from generators)
- `SlashValidator` - Slash validator (admin only)
- `UpdateProverStats` - Update prover reputation after job completion (CPI)

**PDAs** (Program Derived Addresses):
```
Config:     ["config"]
Prover:     ["prover", prover_wallet]
Validator:  ["validator", validator_wallet]
```

**State Accounts**:

```rust
BedrockConfig {
    admin: Pubkey,
    zk_generator_program: Pubkey,
    fhe_generator_program: Pubkey,
    threshold_pubkey: Pubkey,
    next_job_id: u64,               // Global job ID counter
    total_provers: u64,
    total_validators: u64,
    min_validator_stake: u64,       // Default: 0.1 SOL
    total_jobs_completed: u64,
}

ProverAccount {
    authority: Pubkey,
    stake_lamports: u64,            // Min: 0.1 SOL
    jobs_completed: u64,
    jobs_failed: u64,
    total_earnings: u64,
    is_active: bool,
    registered_at: i64,
    last_active_at: i64,
}

ValidatorAccount {
    authority: Pubkey,
    endpoint: String,               // HTTP endpoint for off-chain coordination
    region: ValidatorRegion,        // Geographic region
    stake: u64,
    is_active: bool,
    jobs_served: u64,
    last_heartbeat: i64,
}
```

**Dependencies**: None (foundation layer)

---

### 2. FHE-Generator (Fully Homomorphic Encryption)

**Purpose**: Manage FHE computation jobs with multi-prover consensus to ensure correct encrypted computations.

**Instructions**:
- `CreateJob` - Create FHE computation job with encrypted witness
- `ClaimJob` - Prover claims job (up to N provers per job)
- `SubmitResult` - Prover submits encrypted result hash
- `FinalizeJob` - Check consensus and distribute payments
- `CancelJob` - Creator cancels pending job

**PDAs**:
```
FheJob:         ["fhe_job", creator, job_id]
ConsensusData:  ["fhe_consensus", creator, job_id]
Escrow:         ["fhe_escrow", creator, job_id]
```

**State Accounts**:

```rust
FheJob {
    common: JobCommon,              // Shared job fields (status, creator, claimer, etc.)
    circuit_type: u8,               // FHE operation type (4-11)
    fhe_consensus_bump: u8,
}

FheConsensusData {
    required_provers: u8,           // 2-5 provers must claim
    consensus_threshold: u8,        // Minimum matching results for consensus
    provers: Vec<Pubkey>,           // Provers who claimed
    results: Vec<[u8; 32]>,        // Result hashes submitted
    consensus_hash: Option<[u8; 32]>, // Agreed-upon result
}
```

**Supported Operations** (Circuit Types 4-11):
- `CIRCUIT_FHE_ADD` (4) - Encrypted addition
- `CIRCUIT_FHE_MULTIPLY` (5) - Encrypted multiplication
- `CIRCUIT_FHE_SUM` (6) - Sum of encrypted values
- `CIRCUIT_FHE_THRESHOLD` (7) - Threshold check
- `CIRCUIT_FHE_RANGE_CHECK` (8) - Range verification
- `CIRCUIT_FHE_AVERAGE` (9) - Average computation
- `CIRCUIT_FHE_COUNT_IF` (10) - Conditional counting
- `CIRCUIT_FHE_HISTOGRAM` (11) - Distribution histogram

**Consensus Mechanism**:
1. Job creator specifies `required_provers` (2-5) and `consensus_threshold`
2. Multiple provers claim the same job
3. Each prover computes FHE operation and submits result hash
4. Results are compared - if `consensus_threshold` provers agree, job succeeds
5. Provers with matching results split payment
6. Provers with mismatching results get slashed

**CPI Dependencies**:
- `bedrock::verify_prover()` - Check prover registration before claim
- `bedrock::update_prover_stats()` - Update reputation after finalization

---

### 3. ZK-Generator (Zero-Knowledge Proofs)

**Purpose**: Manage ZK proof generation jobs with single-prover optimistic verification.

**Instructions**:
- `CreateJob` - Create ZK proof job with encrypted witness
- `ClaimJob` - Single prover claims job
- `SubmitProof` - Prover submits proof hash (optimistic)
- `DisputeProof` - Anyone can dispute with full proof (24h window)
- `CancelJob` - Creator cancels pending job
- `RegisterProver` - Register prover with stake (0.5 SOL minimum)
- `DepositStake` - Add more stake
- `WithdrawStake` - Withdraw available stake

**PDAs**:
```
ZkJob:         ["zk_job", creator, job_id]
Escrow:        ["zk_escrow", creator, job_id]
DisputeData:   ["zk_dispute", job_id]
```

**State Accounts**:

```rust
ZkJob {
    common: JobCommon,              // Shared job fields
    circuit_type: u8,               // Circuit type (see below)
}

DisputeData {
    job_id: u64,
    disputer: Pubkey,
    full_proof: Vec<u8>,            // 256 bytes for Groth16
    public_inputs: Vec<u8>,
    dispute_time: i64,
    is_valid: Option<bool>,         // Result of on-chain verification
}
```

**Supported Circuits**:

**Legacy (v1.x)**:
- `ZcashOrchard` (0)
- `ZcashSapling` (1)
- `AnonymousVote` (2)
- `Credential` (3)

**Core Primitives (v2.0)**:
- `ProofOfInnocence` (10) - Merkle non-membership proof (blacklist exclusion)

**Verticals (v2.0)**:
- `PrivateVote` (20) - Anonymous DAO voting
- `PrivateVoteWithPoI` (21) - Voting with conflict of interest check
- `MarketBet` (30) - Private prediction market bet
- `MarketBetWithPoI` (31) - Bet with insider trading prevention
- `MarketClaim` (32) - Claim market winnings privately
- `PortfolioCompliance` (40) - Regulatory compliance proof
- `PortfolioNetWorth` (41) - Net worth threshold proof

**Optimistic Verification**:
1. Prover submits only proof hash (not full proof)
2. Payment released immediately (optimistic trust)
3. 24-hour dispute window opens
4. Anyone can challenge by providing full proof for on-chain verification
5. If dispute succeeds, prover is slashed

**CPI Dependencies**:
- `bedrock::verify_prover()` - Check prover registration
- `bedrock::update_prover_stats()` - Update reputation

---

### 4. Futarchy-Markets (Prediction Markets)

**Purpose**: Private prediction markets with ZK proofs for bets and claims, with optional governance integration.

**Instructions**:
- `CreateMarket` - Create YES/NO prediction market
- `PlaceBet` - Place bet with ZK proof (circuit 30/31)
- `SettleMarket` - Oracle settles market with outcome
- `ClaimPayout` - Claim winnings with ZK proof (circuit 32)
- `CancelMarket` - Cancel market before settlement
- `UpdatePool` - Update encrypted pools after FHE computation
- `RegisterUser` - Register with Proof of Innocence (PoI)
- `CreateMarketWithGovernance` - Create market with executable action
- `ExecuteGovernanceAction` - Execute action after settlement + timelock
- `DepositToMarket` - Deposit to user escrow
- `WithdrawFromEscrow` - Withdraw from escrow

**PDAs**:
```
Market:          ["market", authority, market_id]
Position:        ["position", user, market_id, bet_commitment]
Escrow:          ["escrow", market_id]
UserEligibility: ["user_eligibility", user]
UserEscrow:      ["user_escrow", user, market_id]
```

**State Accounts**:

```rust
Market {
    authority: Pubkey,
    market_id: u64,
    oracle: Pubkey,                 // Can settle market
    question_hash: [u8; 32],
    end_time: i64,
    status: MarketStatus,           // Active, Paused, Settled, Cancelled
    max_bet: u64,

    // Pools (transparent in MVP)
    total_yes_bets: u64,
    total_no_bets: u64,

    // FHE encrypted pools (future)
    encrypted_pool_yes: Vec<u8>,
    encrypted_pool_no: Vec<u8>,
    pending_pool_update_job: Option<u64>,

    // Resolution
    resolution: Option<bool>,       // true = YES won, false = NO won
    settled_at: Option<i64>,

    // Privacy
    claim_nullifiers: Vec<[u8; 32]>, // Prevent double claims

    // Governance
    executable_action: ExecutableAction,
    execution_threshold: u8,        // % of YES votes needed (0-100)
    timelock_duration: i64,
    timelock_expires_at: Option<i64>,
    action_executed: bool,
}

Position {
    user: Pubkey,
    market_id: u64,
    bet_commitment: [u8; 32],       // From ZK proof
    created_at: i64,
}

UserEligibility {
    user: Pubkey,
    blacklist_root: [u8; 32],       // PoI blacklist used
    blacklist_version: u32,
    registered_at: i64,
    expiry: i64,
}
```

**Privacy Features**:
- Bet amounts are hidden via ZK commitments
- MarketBet circuit (30) proves valid bet without revealing amount
- MarketBetWithPoI circuit (31) adds insider trading prevention
- MarketClaim circuit (32) proves winning bet for payout
- Claim nullifiers prevent double claiming

**Governance Integration**:
- Markets can have executable actions (e.g., update program params)
- If YES votes >= threshold, action becomes executable after timelock
- Supports: ParamChange, TransferFunds, ProgramUpgrade, CustomCPI

**CPI Dependencies**:
- `zk-generator::verify_proof()` - Verify MarketBet/MarketClaim proofs
- `fhe-generator::create_job()` - Optional FHE pool encryption (future)
- `bedrock::verify_prover()` - Optional prover verification

---

### 5. Threshold (Key Share Coordination)

**Purpose**: Coordinate 3-of-5 threshold encryption for witness data decryption.

**Instructions**:
- `RequestKeyShare` - Prover requests key shares for encrypted witness
- `SubmitKeyShare` - Validator submits encrypted key share

**PDAs**:
```
KeyShareRequest:  ["key_share_request", job_id]
KeyShareResponse: ["key_share_response", request, validator]
```

**State Accounts**:

```rust
KeyShareRequest {
    job_id: u64,
    prover: Pubkey,
    encrypted_witness_cid: String,  // IPFS CID
    required_shares: u8,            // Default: 3
    received_shares: u8,
    status: RequestStatus,          // Pending, Ready, Expired
    created_at: i64,
    expires_at: i64,                // 5 minutes from creation
}

KeyShareResponse {
    request: Pubkey,
    validator: Pubkey,
    encrypted_share: Vec<u8>,       // Encrypted for prover's pubkey
    timestamp: i64,
}
```

**Protocol Flow**:
1. Prover claims ZK job with encrypted witness (stored on IPFS)
2. Prover calls `RequestKeyShare` with witness CID
3. 5 registered validators monitor requests off-chain
4. At least 3 validators call `SubmitKeyShare` with their encrypted key shares
5. Once threshold met (3+), request status becomes `Ready`
6. Prover retrieves all responses, decrypts shares, reconstructs decryption key
7. Prover decrypts witness and generates proof

**Security**:
- Only validators staked in Bedrock can submit shares
- Each validator can respond only once per request
- Shares are encrypted for prover's public key (off-chain encryption)
- Requests expire after 5 minutes

**Dependencies**:
- Reads `bedrock::ValidatorAccount` to verify validator registration
- Reads `zk-generator::ZkJob` to verify job exists and prover is claimer

---

### 6. Zyberlink (DEPRECATED)

**Status**: DEPRECATED - Functionality has been migrated to `fhe-generator` and `zk-generator`.

This program is no longer actively developed or deployed. All job management has been split into specialized generator programs for better modularity and feature support.

---

## Data Flow Diagrams

### Flow 1: FHE Job (Encrypted Computation)

```
┌─────────┐
│ Creator │
└────┬────┘
     │
     │ 1. CreateJob(circuit_type=FHE_ADD, required_provers=3)
     ▼
┌──────────────┐        ┌─────────────────┐
│FHE-GENERATOR │───────►│ FheJob (Pending)│
└──────┬───────┘        └─────────────────┘
       │                ┌──────────────────────┐
       │                │FheConsensusData      │
       │                │  provers: []         │
       │                │  results: []         │
       │                └──────────────────────┘
       │
       │ 2. ClaimJob() (3 provers)
       ▼
  [CPI] verify_prover() ──► BEDROCK (check stake, registration)
       │
       ▼
┌──────────────────────┐
│FheConsensusData      │
│  provers: [P1,P2,P3] │
│  results: []         │
└──────────────────────┘
       │
       │ 3. Each prover computes FHE operation off-chain
       │    and submits SubmitResult(result_hash)
       ▼
┌──────────────────────┐
│FheConsensusData      │
│  provers: [P1,P2,P3] │
│  results: [H1,H2,H1] │ ◄─ 2 match, 1 differs
│  consensus: H1       │
└──────────────────────┘
       │
       │ 4. FinalizeJob()
       ▼
  Check consensus: 2/3 agree on H1
  ├─ YES: Pay P1 & P3 (split reward)
  │       Slash P2
  │       [CPI] update_prover_stats(P1: +1 success)
  │       [CPI] update_prover_stats(P2: +1 fail)
  │       [CPI] update_prover_stats(P3: +1 success)
  └─ NO:  Refund creator, slash all provers
```

### Flow 2: ZK Proof Verification (Optimistic)

```
┌─────────┐
│ Creator │
└────┬────┘
     │
     │ 1. CreateJob(circuit_type=MarketBet, witness_hash)
     ▼
┌──────────────┐        ┌──────────────────┐
│ZK-GENERATOR  │───────►│ ZkJob (Pending)  │
└──────┬───────┘        │ + Escrow         │
       │                └──────────────────┘
       │
       │ 2. ClaimJob()
       ▼
  [CPI] verify_prover() ──► BEDROCK
       │
       ▼
┌──────────────────┐
│ ZkJob (Claimed)  │
│ claimer: Prover1 │
└──────────────────┘
       │
       │ 3. Prover requests key shares for witness decryption
       ▼
┌───────────────┐       ┌─────────────────────┐
│  THRESHOLD    │──────►│ KeyShareRequest     │
└───────┬───────┘       │ witness_cid: "Qm..." │
        │               └─────────────────────┘
        │
        │ 4. Validators submit key shares
        ▼
┌─────────────────────┐
│ KeyShareResponses   │
│ V1, V2, V3 (3-of-5) │ ◄─ Threshold met
└─────────────────────┘
        │
        │ 5. Prover reconstructs key, decrypts witness, generates proof
        │    SubmitProof(proof_hash)
        ▼
┌──────────────────────┐
│ ZkJob (Completed)    │  ◄─ Payment released (optimistic)
│ proof_hash: 0xABC... │
│ completed_at: T      │
└──────────────────────┘
        │
        │ 6a. Dispute window (24h)
        │     DisputeProof(full_proof, public_inputs)
        ▼
   Verify proof on-chain
        │
        ├─ Valid:   No action (prover keeps payment)
        └─ Invalid: Slash prover, refund creator
```

### Flow 3: Futarchy Market (Create → Bet → Settle → Claim)

```
┌───────────┐
│ Authority │
└─────┬─────┘
      │
      │ 1. CreateMarket(question_hash, end_time, max_bet)
      ▼
┌─────────────────┐       ┌────────────────────────┐
│FUTARCHY-MARKETS │──────►│ Market (Active)        │
└────────┬────────┘       │ total_yes_bets: 0      │
         │                │ total_no_bets: 0       │
         │                │ resolution: None       │
         │                └────────────────────────┘
         │
         │ 2. Users place bets with ZK proofs
         │    PlaceBet(bet_commitment, proof, amount)
         ▼
    [CPI] verify ZK proof (circuit 30: MarketBet)
         │      ├─ Proves: user knows secret bet details
         │      └─ Public: bet_commitment
         ▼
┌────────────────────────┐
│ Market (Active)        │
│ total_yes_bets: 500    │  ◄─ Transparent pools (MVP)
│ total_no_bets: 300     │
└────────────────────────┘
┌────────────────────────┐
│ Position (User A)      │
│ bet_commitment: 0x123  │  ◄─ Bet details hidden
└────────────────────────┘
         │
         │ Time passes → end_time reached
         │
         │ 3. Oracle settles market
         │    SettleMarket(outcome=YES)
         ▼
┌────────────────────────┐
│ Market (Settled)       │
│ resolution: YES        │
│ settled_at: T          │
└────────────────────────┘
         │
         │ 4. Winners claim with ZK proof
         │    ClaimPayout(claim_nullifier, proof, payout_amount)
         ▼
    [CPI] verify ZK proof (circuit 32: MarketClaim)
         │      ├─ Proves: user has winning bet for payout amount
         │      ├─ Public: claim_nullifier
         │      └─ Prevents: double claims (nullifier tracking)
         ▼
┌────────────────────────┐
│ Market (Settled)       │
│ claim_nullifiers: [N1] │  ◄─ Nullifier recorded
└────────────────────────┘
         │
         └──► Transfer payout from escrow to user
```

### Flow 4: Governance Market Execution

```
┌───────────┐
│ Authority │
└─────┬─────┘
      │
      │ 1. CreateMarketWithGovernance(
      │      question: "Should we increase fees?",
      │      executable_action: UpdateParam(fee: 2%),
      │      execution_threshold: 66,  // 66% YES needed
      │      timelock_duration: 86400  // 24h delay
      │    )
      ▼
┌────────────────────────┐
│ Market (Active)        │
│ executable_action: ... │
│ execution_threshold: 66│
│ timelock_duration: 24h │
└────────────────────────┘
      │
      │ 2. Users vote by betting
      │    (YES = vote for, NO = vote against)
      │
      ▼
┌────────────────────────┐
│ Market (Active)        │
│ total_yes_bets: 660    │  ◄─ 66% YES
│ total_no_bets: 340     │     34% NO
└────────────────────────┘
      │
      │ 3. Oracle settles (betting closed)
      │    SettleMarket(outcome=YES)
      ▼
┌────────────────────────┐
│ Market (Settled)       │
│ resolution: YES        │
│ yes_vote_pct: 66%      │  ◄─ >= threshold
│ timelock_expires_at:   │
│   settled_at + 24h     │
└────────────────────────┘
      │
      │ 4. Wait for timelock to expire (24h)
      │
      ▼
┌────────────────────────┐
│ Market (Settled)       │
│ timelock_expires_at: T │  ◄─ current_time >= T
│ action_executed: false │
└────────────────────────┘
      │
      │ 5. Anyone executes action
      │    ExecuteGovernanceAction()
      ▼
   Execute UpdateParam(fee: 2%)
      │
      └──► Update target program config
           Mark action_executed = true
```

---

## Cross-Program Invocations (CPI)

### CPI Call Graph

```
┌────────────────┐
│   BEDROCK      │◄──────────────┐
└────────────────┘               │
        ▲                        │
        │                        │
        │ verify_prover()        │ verify_prover()
        │ update_prover_stats()  │ update_prover_stats()
        │                        │
        │                        │
┌───────┴────────┐        ┌──────┴─────────┐
│ FHE-GENERATOR  │        │ ZK-GENERATOR   │
└────────────────┘        └────────┬───────┘
                                   ▲
                                   │
                                   │ verify_proof()
                                   │ (MarketBet, MarketClaim)
                                   │
                          ┌────────┴──────────┐
                          │ FUTARCHY-MARKETS  │
                          └───────────────────┘
                                   │
                                   │ create_job() (future)
                                   ▼
                          ┌────────────────┐
                          │ FHE-GENERATOR  │
                          └────────────────┘
```

### CPI Details

**FHE-Generator → Bedrock**:
- `verify_prover(prover_account)` - Before allowing ClaimJob
- `update_prover_stats(prover, completed, failed)` - After FinalizeJob
- `slash_prover(prover, amount, reason)` - If consensus fails

**ZK-Generator → Bedrock**:
- `verify_prover(prover_account)` - Before allowing ClaimJob
- `update_prover_stats(prover, completed, failed)` - After SubmitProof or DisputeProof
- `slash_prover(prover, amount, reason)` - If proof is invalid

**Futarchy-Markets → ZK-Generator**:
- `verify_proof(circuit_type, proof, public_inputs)` - During PlaceBet (circuit 30/31)
- `verify_proof(circuit_type, proof, public_inputs)` - During ClaimPayout (circuit 32)

**Futarchy-Markets → FHE-Generator** (future):
- `create_job(circuit_type=FHE_ADD, encrypted_data)` - For private pool updates

**Threshold → Bedrock** (read-only):
- Reads `ValidatorAccount` to verify validator is registered and staked

**Threshold → ZK-Generator** (read-only):
- Reads `ZkJob` to verify prover is claimer of the job

---

## Deployment Guide

### Deployment Order

Programs must be deployed in dependency order:

```
1. bedrock          (no dependencies)
   ↓
2. threshold        (reads bedrock validators)
   ↓
3. fhe-generator    (CPI to bedrock)
   ↓
4. zk-generator     (CPI to bedrock)
   ↓
5. futarchy-markets (CPI to zk-generator, fhe-generator)
```

### Build Commands

```bash
# Build all programs
cd src/programs
anchor build

# Build individual program
cd bedrock && cargo build-sbf
cd fhe-generator && cargo build-sbf
cd zk-generator && cargo build-sbf
cd futarchy-markets && cargo build-sbf
cd threshold && cargo build-sbf
```

### Deployment Commands

```bash
# Deploy to localnet
solana program deploy target/deploy/bedrock.so
solana program deploy target/deploy/threshold.so
solana program deploy target/deploy/fhe_generator.so
solana program deploy target/deploy/zk_generator.so
solana program deploy target/deploy/futarchy_markets.so

# Verify deployment
solana program show <PROGRAM_ID>
```

### Initialization Sequence

After deployment, initialize in this order:

**1. Initialize Bedrock**
```rust
// Set up core registry with generator program IDs
bedrock::Initialize {
    zk_generator_program: <ZK_GEN_PROGRAM_ID>,
    fhe_generator_program: <FHE_GEN_PROGRAM_ID>,
}
```

**2. Register Validators**
```rust
// Register threshold encryption validators
bedrock::RegisterValidator {
    endpoint: "https://validator1.example.com",
    region: ValidatorRegion::NorthAmerica,
    stake: 100_000_000, // 0.1 SOL
}
// Repeat for 5 validators (3-of-5 threshold)
```

**3. Register Provers**
```rust
// Provers self-register with stake
bedrock::RegisterProver {
    stake_lamports: 100_000_000, // 0.1 SOL
}
```

**4. Test FHE Job**
```rust
// Create test FHE job
fhe_generator::CreateJob {
    job_id: 1,
    circuit_type: CIRCUIT_FHE_ADD,
    required_provers: 2,
    consensus_threshold: 2,
    // ... other params
}
```

**5. Test ZK Job**
```rust
// Create test ZK job
zk_generator::CreateJob {
    job_id: 2,
    circuit_type: CIRCUIT_PRIVATE_VOTE,
    // ... other params
}
```

**6. Test Market**
```rust
// Create test prediction market
futarchy_markets::CreateMarket {
    market_id: 1,
    question_hash: blake3("Will SOL hit $300 in 2025?"),
    end_time: 1735689600, // Jan 1, 2026
    max_bet: 1_000_000_000, // 1 SOL
}
```

---

## Program IDs (Localnet Placeholders)

For production deployment, generate new keypairs:

```bash
solana-keygen new -o bedrock-keypair.json
solana-keygen new -o fhe-generator-keypair.json
solana-keygen new -o zk-generator-keypair.json
solana-keygen new -o threshold-keypair.json
# futarchy-markets uses: FutMkts111111111111111111111111111111111111
```

Update `declare_id!()` macros in each program's `lib.rs` with the generated public keys.

---

## Development Tools

### Testing

```bash
# Run program tests
cd src/programs
cargo test-sbf

# Run specific program tests
cd bedrock && cargo test-sbf
cd fhe-generator && cargo test-sbf
cd zk-generator && cargo test-sbf
cd futarchy-markets && cargo test-sbf
cd threshold && cargo test-sbf

# End-to-end tests
cd e2e && cargo test
```

### SDK Usage

SDKs are available in `src/programs/sdks/`:

```bash
# Futarchy SDK (TypeScript)
cd sdks/futarchy-sdk
npm install
npm run build

# Use in your dApp
import { FutarchyClient } from '@zyberlink/futarchy-sdk';
const client = new FutarchyClient(connection, wallet);
await client.createMarket({...});
```

### Monitoring

Monitor program logs on localnet:

```bash
solana logs <PROGRAM_ID>

# Example
solana logs FutMkts111111111111111111111111111111111111
```

---

## Security Considerations

### Stake Requirements
- **Provers**: Minimum 0.1 SOL stake required
- **Validators**: Minimum 0.1 SOL stake (configurable in BedrockConfig)
- Stakes are slashed for misbehavior (invalid proofs, consensus failures)

### Slashing Conditions
- **FHE**: Prover submits result not matching consensus
- **ZK**: Prover submits invalid proof (caught in dispute)
- **Threshold**: Validator fails to respond or submits invalid share

### Timeouts
- **FHE Jobs**: Configurable timeout (typically 5-10 minutes)
- **ZK Jobs**: Configurable timeout (typically 15-30 minutes)
- **Dispute Window**: 24 hours after ZK proof submission
- **Key Share Requests**: 5 minutes

### Access Control
- **Bedrock Admin**: Can slash validators, update config
- **Market Oracle**: Can settle markets
- **Generator Programs**: Can update prover stats via CPI
- **Validators**: Must be registered in Bedrock to submit key shares

### Privacy Guarantees
- **ZK Proofs**: Hide witness data (e.g., vote choice, bet amount)
- **FHE**: Computations on encrypted data (pools, aggregations)
- **Threshold Encryption**: Witness encrypted, requires 3-of-5 validators to decrypt
- **Commitment Schemes**: Bet commitments hide amounts until claim

---

## Roadmap & Future Enhancements

### Phase 1: MVP (Current)
- ✅ Bedrock registry with prover/validator staking
- ✅ FHE-generator with multi-prover consensus
- ✅ ZK-generator with optimistic verification
- ✅ Futarchy markets with ZK bets/claims
- ✅ Threshold key share coordination

### Phase 2: Privacy Enhancements
- ⬜ Full FHE integration for market pools
- ⬜ Private odds calculation (encrypted pools)
- ⬜ Merkle tree for claim nullifiers (gas optimization)
- ⬜ TEE integration for witness handling

### Phase 3: Governance
- ✅ Governance markets with executable actions
- ⬜ DAO treasury integration
- ⬜ Multi-sig governance for admin operations
- ⬜ Automated market settlement via Switchboard oracles

### Phase 4: Scalability
- ⬜ Compressed NFTs for position tracking
- ⬜ State compression for large nullifier sets
- ⬜ Off-chain indexing for market history
- ⬜ Prover selection based on reputation

---

## References

### Documentation
- [Bedrock Architecture](./bedrock/README.md)
- [FHE-Generator Spec](./fhe-generator/README.md)
- [ZK-Generator Circuits](./zk-generator/README.md)
- [Futarchy Markets Guide](./futarchy-markets/README.md)
- [Threshold Protocol](./threshold/README.md)

### External Resources
- [Solana Program Library](https://spl.solana.com/)
- [Anchor Framework](https://www.anchor-lang.com/)
- [Groth16 Proofs](https://eprint.iacr.org/2016/260.pdf)
- [Threshold Cryptography](https://en.wikipedia.org/wiki/Threshold_cryptosystem)

---

## Contributing

When modifying programs:

1. **Update tests** - Add unit tests for new instructions
2. **Document state changes** - Update this README with new PDAs or account fields
3. **Test CPI flows** - Ensure cross-program calls work end-to-end
4. **Version compatibility** - Maintain backwards compatibility for existing jobs/markets
5. **Security review** - Flag any changes to stake/slash logic for review

---

## License

Apache 2.0
