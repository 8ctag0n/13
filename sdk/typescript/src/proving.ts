import { loadVerificationKey, type LoadVKeyOptions } from './circuits';
import { ZkCircuitType } from './types';

export interface Groth16Proof {
  pi_a: [string, string, string?];
  pi_b: [[string, string], [string, string], [string, string]?];
  pi_c: [string, string, string?];
  protocol?: string;
  curve?: string;
}

export interface VerifyGroth16Params {
  /** Circuit id to select the verification key */
  circuitId: number | ZkCircuitType;
  /** Public inputs/signals for the circuit */
  publicSignals: Array<string | bigint>;
  /** Proof object in snarkjs/groth16 format */
  proof: Groth16Proof;
  /** Override verification key resolution */
  vkey?: any;
  /** Options for loading verification key */
  loadOptions?: LoadVKeyOptions;
}

/**
 * Verify a Groth16 proof using snarkjs and the circuit registry.
 *
 * Note: snarkjs is loaded dynamically; add it to your project if not already available.
 */
export async function verifyGroth16(params: VerifyGroth16Params): Promise<boolean> {
  const snarkjs = await loadSnarkjs();
  const vkey = params.vkey ?? (await loadVerificationKey(params.circuitId, params.loadOptions));
  const publicSignals = params.publicSignals.map((p) => p.toString());
  return snarkjs.groth16.verify(vkey, publicSignals, params.proof);
}

// We type this as any to avoid pulling snarkjs types into DTS. Runtime will load if present.
async function loadSnarkjs(): Promise<any> {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return require('snarkjs');
  } catch (err) {
    throw new Error(
      'snarkjs is required to verify proofs locally. Install it with `bun add snarkjs` or provide a custom verifier.'
    );
  }
}
