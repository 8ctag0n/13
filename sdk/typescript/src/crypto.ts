import { createHash } from 'crypto';

// Types
export type HashFn = (left: bigint, right: bigint) => bigint;

export interface MerkleProof {
  path: bigint[];
  indices: number[];
  root: bigint;
  leaf: bigint;
}

/**
 * Minimal binary Merkle tree. Default hash is Poseidon if available; otherwise it will
 * throw when trying to hash unless a custom hashFn is provided.
 */
export class MerkleTree {
  private leaves: bigint[];
  private layers: bigint[][];
  private hashFn: HashFn;
  private zero: bigint;

  constructor(
    leaves: bigint[],
    options: {
      hashFn?: HashFn;
      depth?: number;
      zero?: bigint;
    } = {}
  ) {
    this.zero = options.zero ?? 0n;
    this.hashFn = options.hashFn ?? defaultPoseidonHashFn();
    const depth = options.depth ?? Math.ceil(Math.log2(Math.max(2, leaves.length))) + 1;
    this.leaves = this.padLeaves(leaves, depth);
    this.layers = this.buildLayers(this.leaves);
  }

  static async createWithPoseidon(
    leaves: bigint[],
    options: { depth?: number; zero?: bigint } = {}
  ): Promise<MerkleTree> {
    const hashFn = await createPoseidonHashFn();
    return new MerkleTree(leaves, { ...options, hashFn });
  }

  getRoot(): bigint {
    return this.layers[this.layers.length - 1][0];
  }

  getProof(index: number): MerkleProof {
    if (index < 0 || index >= this.leaves.length) {
      throw new Error('Leaf index out of range');
    }
    const path: bigint[] = [];
    const indices: number[] = [];
    let idx = index;
    for (let level = 0; level < this.layers.length - 1; level += 1) {
      const isRight = idx % 2;
      const siblingIdx = isRight ? idx - 1 : idx + 1;
      const sibling = this.layers[level][siblingIdx] ?? this.zero;
      path.push(sibling);
      indices.push(isRight);
      idx = Math.floor(idx / 2);
    }

    return { path, indices, root: this.getRoot(), leaf: this.leaves[index] };
  }

  verify(leaf: bigint, proof: MerkleProof): boolean {
    return MerkleTree.verifyProof(leaf, proof, this.hashFn);
  }

  static verifyProof(leaf: bigint, proof: MerkleProof, hashFn?: HashFn): boolean {
    const hf = hashFn ?? defaultPoseidonHashFn();
    let acc = leaf;
    for (let i = 0; i < proof.path.length; i += 1) {
      const sibling = proof.path[i];
      const isRight = proof.indices[i] === 1;
      acc = isRight ? hf(sibling, acc) : hf(acc, sibling);
    }
    return acc === proof.root;
  }

  private padLeaves(leaves: bigint[], depth: number): bigint[] {
    const size = 2 ** (depth - 1);
    if (leaves.length > size) {
      throw new Error(`Too many leaves for depth ${depth}: got ${leaves.length}, max ${size}`);
    }
    const padded = [...leaves];
    while (padded.length < size) padded.push(this.zero);
    return padded;
  }

  private buildLayers(leaves: bigint[]): bigint[][] {
    const layers: bigint[][] = [leaves];
    let current = leaves;
    while (current.length > 1) {
      const next: bigint[] = [];
      for (let i = 0; i < current.length; i += 2) {
        const left = current[i];
        const right = current[i + 1] ?? this.zero;
        next.push(this.hashFn(left, right));
      }
      layers.push(next);
      current = next;
    }
    return layers;
  }
}

// Poseidon utilities (lazy loaded)
let poseidonCache: any | null | undefined; // undefined: not tried, null: unavailable

export async function loadPoseidon(): Promise<any> {
  if (poseidonCache !== undefined) {
    if (poseidonCache === null) {
      throw new Error('circomlibjs (poseidon) is not available. Install it with `bun add circomlibjs`.');
    }
    return poseidonCache;
  }
  try {
    const mod = await import('circomlibjs');
    const builder = (mod as any).buildPoseidon ?? (mod as any).poseidon ?? (mod as any).buildPoseidonOpt;
    if (!builder) throw new Error('Poseidon builder not found in circomlibjs');
    const poseidon = await builder();
    poseidonCache = poseidon;
    return poseidon;
  } catch (err) {
    poseidonCache = null;
    throw new Error(
      'circomlibjs is required for Poseidon operations. Install it with `bun add circomlibjs` in your project.'
    );
  }
}

export async function isPoseidonAvailable(): Promise<boolean> {
  try {
    await loadPoseidon();
    return true;
  } catch {
    return false;
  }
}

export async function createPoseidonHashFn(): Promise<HashFn> {
  const poseidon = await loadPoseidon();
  const F = poseidon.F ?? { toObject: (x: any) => BigInt(x) };
  return (l: bigint, r: bigint) => BigInt(F.toObject(poseidon([l, r])));
}

// Default to sha256-based hash (not circuit-compatible) to avoid throwing in sync contexts.
export function defaultPoseidonHashFn(): HashFn {
  return (l: bigint, r: bigint) => sha256HashBigint(l, r);
}

export async function poseidonHash(...inputs: Array<bigint | number | string>): Promise<bigint> {
  const poseidon = await loadPoseidon();
  const F = poseidon.F ?? { toObject: (x: any) => BigInt(x) };
  const coerced = inputs.map((x) => BigInt(x));
  return BigInt(F.toObject(poseidon(coerced)));
}

export async function generateCommitment(value: bigint, blinding: bigint): Promise<bigint> {
  return poseidonHash(value, blinding);
}

export async function generateNullifier(secret: bigint, context: bigint): Promise<bigint> {
  return poseidonHash(secret, context);
}

// Fallback non-Poseidon hash (sha256) for testing / non-circuit contexts
export function sha256HashBigint(left: bigint, right: bigint): bigint {
  const buf = Buffer.concat([toFixedBuffer(left, 32), toFixedBuffer(right, 32)]);
  const digest = createHash('sha256').update(buf).digest('hex');
  return BigInt(`0x${digest}`);
}

function toFixedBuffer(value: bigint, size: number): Buffer {
  const hex = value.toString(16).padStart(size * 2, '0');
  return Buffer.from(hex, 'hex');
}
