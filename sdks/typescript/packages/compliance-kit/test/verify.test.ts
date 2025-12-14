import { describe, expect, it } from 'vitest';

import { verifyGroth16, ZkCircuitType } from '@zyber/core';

describe('compliance-kit verification', () => {
  it('throws when proof payload is invalid', async () => {
    await expect(
      verifyGroth16({
        circuitId: ZkCircuitType.ProofOfInnocence,
        publicSignals: ['0', '0', '0'],
        // @ts-expect-error malformed proof for test
        proof: { pi_a: [], pi_b: [], pi_c: [] },
      })
    ).rejects.toThrow();
  });
});
