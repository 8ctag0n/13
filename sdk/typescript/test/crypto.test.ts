import { describe, expect, it } from 'vitest';

import {
  MerkleTree,
  sha256HashBigint,
  poseidonHash,
  isPoseidonAvailable,
  generateNullifier,
} from '../src/crypto';

describe('crypto helpers', () => {
  it('builds and verifies a merkle proof with sha256 fallback', () => {
    const leaves = [1n, 2n, 3n];
    const tree = new MerkleTree(leaves, { hashFn: sha256HashBigint });
    const proof = tree.getProof(1);
    const ok = MerkleTree.verifyProof(leaves[1], proof, sha256HashBigint);
    expect(ok).toBe(true);
  });

  it('detects merkle tampering', () => {
    const leaves = [1n, 2n, 3n];
    const tree = new MerkleTree(leaves, { hashFn: sha256HashBigint });
    const proof = tree.getProof(1);
    // Flip a bit in the leaf and ensure verification fails
    const tamperedLeaf = leaves[1] + 1n;
    const ok = MerkleTree.verifyProof(tamperedLeaf, proof, sha256HashBigint);
    expect(ok).toBe(false);
  });

  it('fuzzes merkle proofs over random leaves', () => {
    const rand = () => BigInt(Math.floor(Math.random() * 1_000_000));
    const leaves = Array.from({ length: 64 }, rand);
    const tree = new MerkleTree(leaves, { hashFn: sha256HashBigint });
    for (let i = 0; i < 10; i += 1) {
      const idx = Math.floor(Math.random() * leaves.length);
      const proof = tree.getProof(idx);
      expect(MerkleTree.verifyProof(leaves[idx], proof, sha256HashBigint)).toBe(true);
      // tamper
      expect(MerkleTree.verifyProof(leaves[idx] + 1n, proof, sha256HashBigint)).toBe(false);
    }
  });

  it('poseidon hash works when available', async () => {
    if (!(await isPoseidonAvailable())) {
      return;
    }
    const h = await poseidonHash(1n, 2n, 3n);
    expect(typeof h).toBe('bigint');
  });

  it('generates a nullifier when poseidon is available', async () => {
    if (!(await isPoseidonAvailable())) {
      return;
    }
    const n = await generateNullifier(42n, 7n);
    expect(typeof n).toBe('bigint');
  });
});
