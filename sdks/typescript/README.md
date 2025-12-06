# @zyberlink/sdk

TypeScript SDK for **ZyberLink** - Privacy-preserving FHE computation on Solana.

Compute on encrypted data without ever decrypting it. Built on [Fully Homomorphic Encryption (FHE)](https://en.wikipedia.org/wiki/Homomorphic_encryption).

## Installation

```bash
npm install @zyberlink/sdk @solana/kit
```

## Quick Start

```typescript
import { Zyber } from '@zyberlink/sdk';
import { generateKeyPairSigner } from '@solana/kit';

// Generate a signer (or use wallet adapter)
const wallet = await generateKeyPairSigner();

// Connect to devnet
const zyber = await Zyber.connect('devnet', wallet);

// Compute sum of private values
const sum = await zyber.sum([100, 200, 300]);
console.log('Sum:', sum); // 600

// Values are encrypted - provers never see the actual numbers!
```

## Features

### One-Liner Operations

```typescript
// Sum
const total = await zyber.sum([100, 200, 300]);

// Average
const avg = await zyber.average([10, 20, 30]);

// Count values matching a condition
const adults = await zyber.countIf([18, 25, 16, 30], '>=', 18);

// Threshold check
const hasAdult = await zyber.threshold([18, 16, 15], '>=', 18);
```

### Proof of Innocence

Verify a wallet has no sanctioned transactions - without revealing transaction history:

```typescript
const walletTxIds = [123, 456, 789];
const sanctionsList = [999, 888, 777];

const isClean = await zyber.proofOfInnocence(walletTxIds, sanctionsList);
// true = no matches found, wallet is clean
```

### Prepared Operations

Inspect costs and simulate before executing:

```typescript
// Prepare without executing
const prepared = await zyber.prepareSum([100, 200, 300]);

// Check cost
console.log('Cost:', prepared.cost.totalLamports, 'lamports');
console.log('Job ID:', prepared.jobId);
console.log('PDAs:', prepared.pdas);

// Execute when ready
const result = await zyber.execute(prepared);
```

### Batch Operations

Execute multiple operations in parallel:

```typescript
const results = await zyber.batch()
  .sum([100, 200])
  .average([10, 20, 30])
  .countIf([18, 25, 16], '>=', 18)
  .execute();

console.log('Sum:', results[0]);
console.log('Avg:', results[1]);
console.log('Count:', results[2]);
```

## Using with Wallet Adapters

```typescript
import { Zyber } from '@zyberlink/sdk';
import { useWalletAccountTransactionSendingSigner } from '@solana/react';

// In your React component
const wallet = useWalletAccountTransactionSendingSigner(account, 'devnet');
const zyber = await Zyber.connect('devnet', wallet);
```

## Configuration

```typescript
const zyber = await Zyber.connect('devnet', wallet, {
  // Custom RPC
  rpcUrl: 'https://my-rpc.example.com',

  // Custom backend
  backendUrl: 'https://my-backend.example.com',

  // Default job options
  defaultJobOptions: {
    priceLamports: 20_000_000n, // 0.02 SOL per job
    timeoutSeconds: 180,
    requiredProvers: 3,
  },
});
```

## Networks

| Network | RPC | Status |
|---------|-----|--------|
| `mainnet` | api.mainnet-beta.solana.com | Coming soon |
| `devnet` | api.devnet.solana.com | Active |
| `localnet` | localhost:8899 | For development |

## API Reference

### `Zyber.connect(network, signer, options?)`

Connect to a ZyberLink network.

### Operations

| Method | Description |
|--------|-------------|
| `sum(values)` | Sum of encrypted values |
| `average(values)` | Average of encrypted values |
| `countIf(values, op, threshold)` | Count values matching condition |
| `threshold(values, op, threshold)` | Check if any value matches |
| `proofOfInnocence(txs, sanctions)` | Verify no sanctioned transactions |

### Predicates

| Operator | Meaning |
|----------|---------|
| `>` | Greater than |
| `<` | Less than |
| `>=` | Greater than or equal |
| `<=` | Less than or equal |
| `==` | Equal |
| `!=` | Not equal |

## How It Works

1. **Client encrypts** values using FHE (tfhe-rs)
2. **Provers compute** on encrypted data (never see actual values)
3. **Multi-prover consensus** ensures correctness
4. **Client decrypts** the result

Your data stays private throughout the entire computation.

## License

MIT
