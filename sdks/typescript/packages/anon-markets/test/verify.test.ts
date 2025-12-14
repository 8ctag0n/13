import { describe, expect, it } from 'vitest';

import { verifyGroth16, ZkCircuitType } from '@zyber/core';

describe('anon-markets verification', () => {
  it('fails on malformed proof payload', async () => {
    await expect(
      verifyGroth16({
        circuitId: ZkCircuitType.MarketBet,
        publicSignals: ['0', '0', '0', '0'],
        // @ts-expect-error malformed proof for test
        proof: { pi_a: [], pi_b: [], pi_c: [] },
      })
    ).rejects.toThrow();
  });
});
