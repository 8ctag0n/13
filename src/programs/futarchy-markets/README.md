# Futarchy Markets

Native Solana program implementing futarchy-style governance through private prediction markets.

## Overview

This program enables:
- **Private Prediction Markets**: Create YES/NO markets with ZK-proof verified bets
- **Anonymous Betting**: Bet amounts and positions hidden via cryptographic commitments
- **Verifiable Claims**: Claim winnings with ZK proofs (no need to reveal original bet)
- **Oracle-based Settlement**: Markets resolved by designated oracles

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              FUTARCHY-MARKETS (this)                         │
│    Market { question, end_time, status, pools }             │
│    Position { user, market, commitment }                    │
└─────────────────────────────────────────────────────────────┘
                          │ CPI
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   ZK-GENERATOR                               │
│    Circuit 30: MarketBet (private bet verification)         │
│    Circuit 31: MarketBetWithPoI (with insider check)        │
│    Circuit 32: MarketClaim (private claim verification)     │
└─────────────────────────────────────────────────────────────┘
                          │ CPI
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                      BEDROCK                                 │
│    Prover registry and verification                         │
└─────────────────────────────────────────────────────────────┘
```

## State Accounts

### Market
- **PDA**: `["market", market_id]`
- Stores market metadata, pools, and settlement data
- Size: ~3,388 bytes (supports 100 claim nullifiers)

### Position
- **PDA**: `["position", user, market_id, bet_commitment]`
- Stores user's bet commitment (private)
- Size: 106 bytes

### Escrow
- **PDA**: `["escrow", market_id]`
- Holds all bet funds until settlement
- Simple PDA with no data, just lamports

## Instructions

### 1. CreateMarket
Creates a new prediction market.

**Accounts:**
- `[writable, signer]` Market creator/authority
- `[writable]` Market account (PDA)
- `[writable]` Escrow account (PDA)
- `[]` Oracle account
- `[]` System program
- `[]` Clock sysvar

**Parameters:**
- `market_id`: Unique ID
- `question_hash`: Hash of the question
- `end_time`: When betting closes
- `max_bet`: Maximum bet amount

### 2. PlaceBet
Place a bet with ZK proof verification.

**Accounts:**
- `[writable, signer]` User
- `[writable]` Market account
- `[writable]` Position account (PDA)
- `[writable]` Escrow account
- `[]` ZK-generator program
- `[]` System program
- `[]` Clock sysvar

**Parameters:**
- `market_id`: Market to bet on
- `bet_commitment`: Poseidon(amount, position, blinding)
- `proof`: ZK proof (circuit 30 or 31)
- `public_inputs`: Circuit public inputs
- `amount`: Bet amount in lamports
- `circuit_type`: 30 (MarketBet) or 31 (MarketBetWithPoI)

### 3. SettleMarket
Resolve market outcome (oracle only).

**Accounts:**
- `[writable, signer]` Oracle
- `[writable]` Market account
- `[]` Clock sysvar

**Parameters:**
- `market_id`: Market to settle
- `outcome`: true (YES won) or false (NO won)

### 4. ClaimPayout
Claim winnings with ZK proof.

**Accounts:**
- `[writable, signer]` User
- `[writable]` Market account
- `[writable]` Position account (optional)
- `[writable]` Escrow account
- `[]` ZK-generator program
- `[]` System program

**Parameters:**
- `market_id`: Market ID
- `claim_nullifier`: Hash(nullifier_secret, market_id)
- `proof`: ZK proof (circuit 32)
- `public_inputs`: Circuit public inputs
- `payout_amount`: Claimed amount

### 5. CancelMarket
Cancel market before settlement (authority only).

**Accounts:**
- `[writable, signer]` Market authority
- `[writable]` Market account

**Parameters:**
- `market_id`: Market to cancel

## ZK Circuits Integration

The program relies on three pre-compiled ZK circuits:

### Circuit 30: MarketBet
Proves that a bet is valid without revealing amount or position.

**Public Inputs:**
- `market_id`: Market identifier
- `bet_commitment`: Hash(amount, position, blinding)
- `max_bet`: Maximum allowed bet
- `timestamp`: Bet placement time

**Constraints:**
1. `bet_amount <= max_bet`
2. `bet_commitment == Poseidon(amount, position, blinding)`

### Circuit 31: MarketBetWithPoI
Same as Circuit 30 + insider trading prevention.

**Additional Public Input:**
- `insider_blacklist_root`: Merkle root of blacklist

**Additional Constraint:**
- Proves bettor NOT in blacklist (Merkle non-membership)

### Circuit 32: MarketClaim
Proves right to claim winnings without revealing original bet.

**Public Inputs:**
- `market_id`: Market identifier
- `winning_outcome`: Resolved outcome
- `bet_commitment`: Original bet commitment
- `claim_nullifier`: Hash(nullifier_secret, market_id)
- `payout_amount`: Calculated winnings

**Constraints:**
1. Bet commitment matches original
2. Position == winning_outcome
3. Payout calculation correct
4. Nullifier valid (prevents double claim)

## Building

```bash
# Build the program
cargo build-sbf

# Run tests
cargo test-sbf

# Check for errors
cargo check -p futarchy-markets
```

## Dependencies

- **bedrock**: Prover registry (CPI)
- **zk-generator**: ZK proof verification (CPI)
- **fhe-generator**: Future FHE integration
- **zyberlink-types**: Shared types
- **zyberlink-jobs**: Job definitions

## Security Considerations

1. **Nullifier Tracking**: Markets store claim nullifiers to prevent double claims
2. **Escrow Isolation**: Each market has its own escrow PDA
3. **Oracle Trust**: Markets rely on designated oracle for settlement
4. **ZK Proof Verification**: Currently basic validation; full Groth16 verification planned
5. **Pool Privacy**: Current MVP tracks pools transparently; FHE integration planned

## Roadmap

### Phase 1: MVP (Current)
- ✅ Basic market creation
- ✅ ZK-verified betting
- ✅ Oracle settlement
- ✅ ZK-verified claims
- ⚠️ Transparent pools (privacy limited to commitments)

### Phase 2: Privacy Enhancement
- [ ] FHE encrypted bet amounts
- [ ] Private pool calculations
- [ ] Encrypted odds display
- [ ] Full PoI integration (Circuit 31)

### Phase 3: Advanced Features
- [ ] Conditional markets
- [ ] Time-weighted markets
- [ ] Governance execution
- [ ] Quadratic voting

## License

MIT
