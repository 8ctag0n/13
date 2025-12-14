import fs from 'fs/promises';
import { describe, expect, it } from 'vitest';

import { getCircuitMetadata, getVKeyPath, listCircuits, loadVerificationKey } from '../src/circuits';
import { ZkCircuitType } from '../src/types';

describe('circuits registry', () => {
  it('lists all expected circuits', () => {
    const ids = listCircuits().map((c) => c.id).sort();
    expect(ids).toEqual([
      ZkCircuitType.ProofOfInnocence,
      ZkCircuitType.PrivateVote,
      ZkCircuitType.PrivateVoteWithPoI,
      ZkCircuitType.MarketBet,
      ZkCircuitType.MarketBetWithPoI,
      ZkCircuitType.MarketClaim,
      ZkCircuitType.PortfolioCompliance,
      ZkCircuitType.PortfolioNetWorth,
    ]);
  });

  it('resolves vkey path on disk', async () => {
    const circuitId = ZkCircuitType.ProofOfInnocence;
    const meta = getCircuitMetadata(circuitId);
    expect(meta?.vkeyFilename).toBeDefined();

    const vkeyPath = getVKeyPath(circuitId);
    await fs.access(vkeyPath);
  });

  it('loads a verification key JSON', async () => {
    const vkey = await loadVerificationKey(ZkCircuitType.ProofOfInnocence);
    expect(typeof vkey).toBe('object');
    // Common Groth16 fields sanity check
    expect(vkey).toHaveProperty('protocol');
    expect(vkey).toHaveProperty('vk_alpha_1');
    expect(vkey).toHaveProperty('vk_beta_2');
  });
});
