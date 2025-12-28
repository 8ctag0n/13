# Futarchy Markets - Usage Examples

## Overview

Dos modos de operación:

1. **Transparent Mode** (MVP simple)
2. **FHE Mode** (Encrypted pools con multi-prover consensus)

---

## 1. Transparent Mode (Sin FHE)

### Create Market

```typescript
import { PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY } from '@solana/web3.js';
import { FutarchyMarketsProgram } from './sdk';

const program = new FutarchyMarketsProgram(connection, wallet);

// Derive market PDA
const [marketPda] = await PublicKey.findProgramAddress(
  [Buffer.from("market"), Buffer.from([1])], // market_id = 1
  program.programId
);

// Derive escrow PDA
const [escrowPda] = await PublicKey.findProgramAddress(
  [Buffer.from("escrow"), Buffer.from([1])],
  program.programId
);

// Create market
await program.methods
  .createMarket({
    marketId: 1,
    questionHash: Buffer.from(sha256("Will ETH reach $5000 by EOY?")),
    endTime: Date.now() / 1000 + 30 * 24 * 60 * 60, // 30 days
    maxBet: 10_000_000_000, // 10 SOL
  })
  .accounts({
    authority: wallet.publicKey,
    market: marketPda,
    escrow: escrowPda,
    oracle: oraclePublicKey,
    systemProgram: SystemProgram.programId,
    clock: SYSVAR_CLOCK_PUBKEY,
  })
  .rpc();

console.log("Market created:", marketPda.toString());
```

### Place Bet (Transparent)

```typescript
// Generate bet commitment
const betAmount = 1_000_000_000; // 1 SOL
const position = 1; // 1 = YES, 0 = NO
const blinding = randomBytes(32);

const betCommitment = poseidon([betAmount, position, blinding]);

// Generate ZK proof (Circuit 30)
const { proof, publicInputs } = await generateMarketBetProof({
  // Private inputs
  bettorWallet: wallet.publicKey.toBuffer(),
  betAmount,
  position,
  blinding,

  // Public inputs
  marketId: 1,
  betCommitment,
  maxBet: 10_000_000_000,
  timestamp: Date.now() / 1000,
});

// Derive position PDA
const [positionPda] = await PublicKey.findProgramAddress(
  [
    Buffer.from("position"),
    wallet.publicKey.toBuffer(),
    Buffer.from([1]), // market_id
    betCommitment,
  ],
  program.programId
);

// Place bet (NO encrypted_bet_amount = transparent mode)
await program.methods
  .placeBet({
    marketId: 1,
    betCommitment,
    proof,
    publicInputs,
    amount: betAmount,
    circuitType: 30, // MarketBet
    encryptedBetAmount: null, // ← Transparent mode
    side: null,
  })
  .accounts({
    user: wallet.publicKey,
    market: marketPda,
    position: positionPda,
    escrow: escrowPda,
    zkGeneratorProgram: ZK_GENERATOR_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    clock: SYSVAR_CLOCK_PUBKEY,
    // NO FHE accounts needed
  })
  .rpc();

console.log("Bet placed (transparent)");
```

---

## 2. FHE Mode (Encrypted Pools)

### Place Bet (FHE)

```typescript
import { TfheClientKey, TfheCompactPublicKey } from 'tfhe';

// 1. Get market's TFHE public key
const market = await program.account.market.fetch(marketPda);
const tfhePublicKey = TfheCompactPublicKey.deserialize(market.tfhePublicKey);

// 2. Encrypt bet amount client-side
const betAmount = 1_000_000_000; // 1 SOL
const encryptedBetAmount = tfhePublicKey.encrypt_u64(betAmount);

// 3. Generate bet commitment (same as before)
const position = 1; // YES
const blinding = randomBytes(32);
const betCommitment = poseidon([betAmount, position, blinding]);

// 4. Generate ZK proof
const { proof, publicInputs } = await generateMarketBetProof({
  bettorWallet: wallet.publicKey.toBuffer(),
  betAmount,
  position,
  blinding,
  marketId: 1,
  betCommitment,
  maxBet: 10_000_000_000,
  timestamp: Date.now() / 1000,
});

// 5. Derive FHE job PDAs
const fheJobId = await getNextJobId(); // Get from on-chain counter

const [fheJobPda] = await PublicKey.findProgramAddress(
  [
    Buffer.from("fhe_job"),
    marketPda.toBuffer(),
    Buffer.from(fheJobId.toArray("le", 8)),
  ],
  FHE_GENERATOR_PROGRAM_ID
);

const [fheConsensusPda] = await PublicKey.findProgramAddress(
  [
    Buffer.from("fhe_consensus"),
    Buffer.from(fheJobId.toArray("le", 8)),
  ],
  FHE_GENERATOR_PROGRAM_ID
);

const [fheEscrowPda] = await PublicKey.findProgramAddress(
  [Buffer.from("fhe_escrow"), Buffer.from(fheJobId.toArray("le", 8))],
  FHE_GENERATOR_PROGRAM_ID
);

// 6. Place bet with FHE
await program.methods
  .placeBet({
    marketId: 1,
    betCommitment,
    proof,
    publicInputs,
    amount: betAmount,
    circuitType: 30,
    encryptedBetAmount: encryptedBetAmount.serialize(), // ← FHE mode!
    side: true, // YES
  })
  .accounts({
    user: wallet.publicKey,
    market: marketPda,
    position: positionPda,
    escrow: escrowPda,
    zkGeneratorProgram: ZK_GENERATOR_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    clock: SYSVAR_CLOCK_PUBKEY,

    // Additional FHE accounts
    fheJob: fheJobPda,
    fheConsensus: fheConsensusPda,
    fheEscrow: fheEscrowPda,
    fheGeneratorProgram: FHE_GENERATOR_PROGRAM_ID,
  })
  .rpc();

console.log("Bet placed (FHE encrypted)");
console.log("FHE Job ID:", fheJobId);
console.log("Waiting for prover consensus...");
```

### Monitor FHE Job

```typescript
// Poll for consensus
const fheJob = await fheProgram.account.fheJob.fetch(fheJobPda);
const fheConsensus = await fheProgram.account.fheConsensusData.fetch(fheConsensusPda);

console.log("Provers claimed:", fheConsensus.claimedCount, "/", fheConsensus.requiredProvers);
console.log("Results submitted:", fheConsensus.resultsCount);

if (fheConsensus.consensusHash) {
  console.log("✓ Consensus reached!");
  console.log("Consensus hash:", Buffer.from(fheConsensus.consensusHash).toString('hex'));
} else {
  console.log("Waiting for consensus...");
}
```

### Update Pool (After Consensus)

```typescript
// Anyone can call this after consensus is reached
const fheConsensus = await fheProgram.account.fheConsensusData.fetch(fheConsensusPda);

if (!fheConsensus.consensusHash) {
  throw new Error("Consensus not reached yet");
}

// Get the encrypted result from one of the winning provers
const winningProvers = getWinningProvers(fheConsensus);
const encryptedResult = await fetchEncryptedResultFromProver(winningProvers[0]);

// Update pool on-chain
await program.methods
  .updatePool({
    marketId: 1,
    fheJobId,
    encryptedResult,
    side: true, // YES pool
  })
  .accounts({
    updater: wallet.publicKey, // Anyone can call
    market: marketPda,
    fheJob: fheJobPda,
    fheConsensus: fheConsensusPda,
  })
  .rpc();

console.log("✓ Encrypted pool updated!");

// Verify
const updatedMarket = await program.account.market.fetch(marketPda);
console.log("Encrypted YES pool size:", updatedMarket.encryptedPoolYes.length, "bytes");
console.log("Pending job cleared:", updatedMarket.pendingPoolUpdateJob === null);
```

---

## 3. Settle Market

```typescript
// Oracle settles (after end_time)
await program.methods
  .settleMarket({
    marketId: 1,
    outcome: true, // YES won
  })
  .accounts({
    oracle: oracleWallet.publicKey,
    market: marketPda,
    clock: SYSVAR_CLOCK_PUBKEY,
  })
  .signers([oracleWallet])
  .rpc();

console.log("Market settled: YES won");
```

---

## 4. Claim Payout

```typescript
// User retrieves their original bet data (stored locally)
const localBetData = {
  amount: 1_000_000_000,
  position: 1, // YES
  blinding,
};

// Calculate payout (proportional distribution)
const market = await program.account.market.fetch(marketPda);
const totalWinningPool = market.totalYesBets; // In transparent mode
const totalLosingPool = market.totalNoBets;
const payoutAmount = localBetData.amount +
  Math.floor((localBetData.amount / totalWinningPool) * totalLosingPool);

// Generate claim proof (Circuit 32)
const nullifierSecret = randomBytes(32);
const claimNullifier = poseidon([nullifierSecret, betCommitment]);
const totalPool = totalWinningPool + totalLosingPool;
const marketIdBytes = Buffer.alloc(8);
marketIdBytes.writeUInt32LE(1, 0);

const [nullifierPda] = await PublicKey.findProgramAddress(
  [Buffer.from("nullifier"), marketIdBytes, claimNullifier],
  program.programId
);

const { proof: claimProof, publicInputs: claimPublicInputs } = await generateMarketClaimProof({
  // Private inputs
  betAmount: localBetData.amount,
  betSide: localBetData.position,
  blinding: localBetData.blinding,
  secret: nullifierSecret,

  // Public inputs
  marketId: 1,
  resolution: 1, // YES
  totalPool,
  winningPool: totalWinningPool,
  betCommitment,
  nullifier: claimNullifier,
  payoutAmount,
  timestamp: Math.floor(Date.now() / 1000),
});

// Claim payout
await program.methods
  .claimPayout({
    marketId: 1,
    claimNullifier,
    proof: claimProof,
    publicInputs: claimPublicInputs,
    payoutAmount,
  })
  .accounts({
    user: wallet.publicKey,
    market: marketPda,
    nullifier: nullifierPda,
    escrow: escrowPda,
    zkGeneratorProgram: ZK_GENERATOR_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

console.log("✓ Payout claimed:", payoutAmount / 1e9, "SOL");
```

---

## Comparison: Transparent vs FHE

| Feature | Transparent Mode | FHE Mode |
|---------|------------------|----------|
| **Pool Visibility** | 👁️ Anyone can see YES/NO pools | 🔒 Pools encrypted, nobody can see |
| **Gas Cost** | ~0.00005 SOL per bet | ~0.00018 SOL per bet (3.6x) |
| **Latency** | Instant | ~30s (prover consensus) |
| **Privacy** | Bet commitments private | Full privacy (amounts + pools) |
| **Complexity** | Simple | Requires 3+ provers |
| **Front-running Risk** | High (pools visible) | None (pools encrypted) |
| **Competitive Advantage** | Standard | 🚀 Unique in market |

---

## Running Provers (For FHE Mode)

### Setup Prover Node

```bash
# Clone prover-node
git clone https://github.com/your-org/zyberlink-provers
cd zyberlink-provers

# Install dependencies
cargo build --release

# Configure
cp .env.example .env
# Edit .env with your prover wallet

# Run
./target/release/prover-node \
  --rpc-url https://api.devnet.solana.com \
  --program-id <FHE_GENERATOR_PROGRAM_ID> \
  --prover-wallet ~/.config/solana/prover-keypair.json
```

### Prover Flow

```
1. Poll for FHE jobs (circuit_type = 4)
2. Download encrypted witness from IPFS
3. Compute: new_pool = tfhe::add(current_pool, new_bet)
4. Submit result_hash on-chain
5. Get paid when consensus reached
```

---

## Next Steps

1. **Frontend Integration**
   - Build React components for market creation/betting
   - TFHE.js integration for client-side encryption
   - Real-time prover status monitoring

2. **Prover Network**
   - Deploy 3+ prover nodes
   - Monitor consensus success rate
   - Implement slashing for dishonest provers

3. **Testing**
   - E2E tests with real TFHE computation
   - Load testing (multiple concurrent bets)
   - Consensus failure scenarios

4. **Production Deployment**
   - Deploy to mainnet
   - Set up monitoring/alerts
   - Document for users
