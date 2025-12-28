/**
 * Helpers to build witness byte payloads for zk-generator circuits.
 * These helpers do not perform cryptographic hashing; callers must precompute
 * commitments/nullifiers/paths as required by the circuits.
 */

export interface VoteWitnessInput {
  pollId: Uint8Array;
  choice: number;
  nullifier: Uint8Array;
  eligibilityProof?: Uint8Array;
  blacklistProof?: Uint8Array;
}

export interface MarketBetWitnessInput {
  marketId: Uint8Array;
  outcome: number;
  amount: bigint;
  nullifier: Uint8Array;
  eligibilityProof?: Uint8Array;
}

export interface MarketClaimWitnessInput {
  marketId: Uint8Array;
  winningOutcome: number;
  betCommitment: Uint8Array;
  claimNullifier: Uint8Array;
  payoutAmount: bigint;
}

export interface ProofOfInnocenceWitnessInput {
  walletHash: Uint8Array;
  sanctionsRoot: Uint8Array;
  merkleProof: Uint8Array;
}

export interface PortfolioComplianceWitnessInput {
  portfolioHash: Uint8Array;
  balances: bigint[];
  thresholds: { min?: bigint; max?: bigint }[];
  complianceRule: Uint8Array;
}

export interface NetWorthWitnessInput {
  netWorth: bigint;
  blinding: bigint;
  oracleProof: Uint8Array;
}

export function buildVoteWitness(input: VoteWitnessInput): Uint8Array {
  const parts: Uint8Array[] = [input.pollId, new Uint8Array([input.choice]), input.nullifier];
  if (input.eligibilityProof) parts.push(input.eligibilityProof);
  if (input.blacklistProof) parts.push(input.blacklistProof);
  return concatBytes(parts);
}

export function buildMarketBetWitness(input: MarketBetWitnessInput): Uint8Array {
  const amountBytes = u64Le(input.amount);
  const parts: Uint8Array[] = [
    input.marketId,
    new Uint8Array([input.outcome]),
    amountBytes,
    input.nullifier,
  ];
  if (input.eligibilityProof) parts.push(input.eligibilityProof);
  return concatBytes(parts);
}

export function buildMarketClaimWitness(input: MarketClaimWitnessInput): Uint8Array {
  const payoutBytes = u64Le(input.payoutAmount);
  return concatBytes([
    input.marketId,
    new Uint8Array([input.winningOutcome]),
    input.betCommitment,
    input.claimNullifier,
    payoutBytes,
  ]);
}

export function buildProofOfInnocenceWitness(input: ProofOfInnocenceWitnessInput): Uint8Array {
  return concatBytes([input.walletHash, input.sanctionsRoot, input.merkleProof]);
}

export function buildPortfolioComplianceWitness(
  input: PortfolioComplianceWitnessInput
): Uint8Array {
  const balanceBytes = new Uint8Array(input.balances.length * 8);
  const view = new DataView(balanceBytes.buffer);
  input.balances.forEach((b, i) => view.setBigUint64(i * 8, b, true));

  const thresholdBytes = new Uint8Array(input.thresholds.length * 16);
  const thresholdView = new DataView(thresholdBytes.buffer);
  input.thresholds.forEach((t, i) => {
    thresholdView.setBigUint64(i * 16, t.min ?? 0n, true);
    thresholdView.setBigUint64(i * 16 + 8, t.max ?? BigInt('0xFFFFFFFFFFFFFFFF'), true);
  });

  return concatBytes([
    input.portfolioHash,
    new Uint8Array([input.balances.length]),
    balanceBytes,
    thresholdBytes,
    input.complianceRule,
  ]);
}

/**
 * Net worth proof witness (circuit 41) builder.
 * Assumes private inputs: net_worth, blinding, oracle path blob (pre-serialized).
 */
export function buildNetWorthWitness(input: NetWorthWitnessInput): Uint8Array {
  const netWorthBytes = u64Le(input.netWorth);
  const blindingBytes = u64Le(input.blinding);
  return concatBytes([netWorthBytes, blindingBytes, input.oracleProof]);
}

function u64Le(value: bigint): Uint8Array {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, value, true);
  return bytes;
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, p) => sum + p.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
