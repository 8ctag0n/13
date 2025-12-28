import { describe, expect, it } from 'vitest';

import { verifyGroth16, ZkCircuitType } from '@zyber/core';

describe('private-vote verification', () => {
  it('rejects invalid proof shape early', async () => {
    await expect(
      verifyGroth16({
        circuitId: ZkCircuitType.PrivateVote,
        publicSignals: ['0', '0', '0', '0', '0'],
        // malformed proof
        // @ts-expect-error testing invalid payload
        proof: { pi_a: [], pi_b: [], pi_c: [] },
      })
    ).rejects.toThrow();
  });
});
