import { describe, expect, it } from 'vitest';

import {
  buildVoteWitness,
  buildMarketBetWitness,
  buildMarketClaimWitness,
  buildProofOfInnocenceWitness,
  buildPortfolioComplianceWitness,
  buildNetWorthWitness,
} from '../src/witness';

const asBytes = (n: number) => new Uint8Array([n]);

describe('witness builders', () => {
  it('builds vote witness deterministically', () => {
    const a = buildVoteWitness({
      pollId: asBytes(1),
      choice: 2,
      nullifier: asBytes(3),
      eligibilityProof: asBytes(4),
    });
    const b = buildVoteWitness({
      pollId: asBytes(1),
      choice: 2,
      nullifier: asBytes(3),
      eligibilityProof: asBytes(4),
    });
    expect(Buffer.from(a).toString('hex')).toBe(Buffer.from(b).toString('hex'));
  });

  it('fuzzes vote witness determinism', () => {
    const randByte = () => new Uint8Array([Math.floor(Math.random() * 255)]);
    for (let i = 0; i < 20; i += 1) {
      const payload = {
        pollId: randByte(),
        choice: Math.floor(Math.random() * 3),
        nullifier: randByte(),
        eligibilityProof: randByte(),
        blacklistProof: randByte(),
      };
      const a = buildVoteWitness(payload);
      const b = buildVoteWitness(payload);
      expect(Buffer.from(a).toString('hex')).toBe(Buffer.from(b).toString('hex'));
    }
  });

  it('builds market bet witness with amount encoding', () => {
    const witness = buildMarketBetWitness({
      marketId: asBytes(9),
      outcome: 1,
      amount: 100n,
      nullifier: asBytes(2),
    });
    expect(witness.length).toBeGreaterThan(0);
  });

  it('builds market claim witness', () => {
    const witness = buildMarketClaimWitness({
      marketId: asBytes(9),
      winningOutcome: 0,
      betCommitment: asBytes(5),
      claimNullifier: asBytes(6),
      payoutAmount: 10n,
    });
    expect(witness.length).toBeGreaterThan(0);
  });

  it('builds PoI witness', () => {
    const w = buildProofOfInnocenceWitness({
      walletHash: asBytes(1),
      sanctionsRoot: asBytes(2),
      merkleProof: asBytes(3),
    });
    expect(w.length).toBe(3);
  });

  it('builds portfolio compliance witness with balances/thresholds', () => {
    const w = buildPortfolioComplianceWitness({
      portfolioHash: asBytes(1),
      balances: [1n, 2n],
      thresholds: [{ min: 0n, max: 10n }, { min: 1n, max: 20n }],
      complianceRule: asBytes(9),
    });
    expect(w.length).toBeGreaterThan(0);
  });

  it('builds net worth witness', () => {
    const w = buildNetWorthWitness({
      netWorth: 1_000n,
      blinding: 7n,
      oracleProof: asBytes(4),
    });
    expect(w.length).toBe(8 + 8 + 1); // two u64 + oracle proof size
  });
});
