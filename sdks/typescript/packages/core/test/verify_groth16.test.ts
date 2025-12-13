import fs from 'fs/promises';
import path from 'path';
import { describe, expect, it } from 'vitest';

import { loadVerificationKey } from '../src/circuits';
import { verifyGroth16 } from '../src/proving';
import { ZkCircuitType } from '../src/types';

// Uses real proof/vkey fixtures from circuits/poi
const FIXTURE_DIR = path.resolve(__dirname, '../../../../../circuits/poi');
const RUN_PROOF_TESTS = process.env.RUN_PROOF_TESTS === '1';
const PROOF_TIMEOUT = Number(process.env.PROOF_TIMEOUT_MS ?? 30000);

async function loadJson(name: string) {
  const raw = await fs.readFile(path.join(FIXTURE_DIR, name), 'utf-8');
  return JSON.parse(raw);
}

describe('verifyGroth16 with real fixtures', () => {
  it('accepts a valid PoI proof', async () => {
    if (!RUN_PROOF_TESTS) return;
    const proof = await loadJson('proof.json');
    const pub = await loadJson('public.json');
    const ok = await verifyGroth16({
      circuitId: ZkCircuitType.ProofOfInnocence,
      publicSignals: pub,
      proof,
    });
    expect(ok).toBe(true);
  }, PROOF_TIMEOUT);

  it('rejects tampered public input', async () => {
    if (!RUN_PROOF_TESTS) return;
    const proof = await loadJson('proof.json');
    const pub = await loadJson('public.json');
    pub[0] = '0'; // tamper
    const ok = await verifyGroth16({
      circuitId: ZkCircuitType.ProofOfInnocence,
      publicSignals: pub,
      proof,
    });
    expect(ok).toBe(false);
  }, PROOF_TIMEOUT);

  it('rejects wrong circuit vkey', async () => {
    if (!RUN_PROOF_TESTS) return;
    const proof = await loadJson('proof.json');
    const pub = await loadJson('public.json');
    const goodVk = await loadVerificationKey(ZkCircuitType.ProofOfInnocence);
    const wrongVk = { ...goodVk, vk_alpha_1: ['0', ...goodVk.vk_alpha_1.slice(1)] };
    const ok = await verifyGroth16({
      circuitId: ZkCircuitType.ProofOfInnocence,
      publicSignals: pub,
      proof,
      vkey: wrongVk,
    });
    expect(ok).toBe(false);
  }, PROOF_TIMEOUT);
});
