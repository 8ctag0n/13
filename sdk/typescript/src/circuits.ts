import fs from 'fs';
import fsp from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

import { ZkCircuitType } from './types';

// Support both CJS/ESM builds
const __dirname = path.dirname(fileURLToPath(import.meta.url));

export interface CircuitMetadata {
  id: number;
  name: string;
  constraints: number;
  publicInputs: string[];
  description: string;
  vkeyFilename: string;
}

const CIRCUITS: Record<number, CircuitMetadata> = {
  10: {
    id: 10,
    name: 'ProofOfInnocence',
    constraints: 4883,
    publicInputs: ['blacklist_root', 'threshold', 'timestamp'],
    description: 'Prove wallet is NOT in blacklist',
    vkeyFilename: 'circuit_10_vkey.json',
  },
  20: {
    id: 20,
    name: 'PrivateVote',
    constraints: 5675,
    publicInputs: ['poll_id', 'eligibility_root', 'nullifier', 'vote_commitment', 'min_balance'],
    description: 'Anonymous DAO voting with nullifier-based double-vote prevention',
    vkeyFilename: 'circuit_20_vkey.json',
  },
  21: {
    id: 21,
    name: 'PrivateVoteWithPoI',
    constraints: 10555,
    publicInputs: [
      'poll_id',
      'eligibility_root',
      'nullifier',
      'vote_commitment',
      'min_balance',
      'blacklist_root',
    ],
    description: 'Anonymous voting + conflict-of-interest blacklist proof',
    vkeyFilename: 'circuit_21_vkey.json',
  },
  30: {
    id: 30,
    name: 'MarketBet',
    constraints: 332,
    publicInputs: ['market_id', 'bet_commitment', 'max_bet', 'timestamp'],
    description: 'Private prediction market bet',
    vkeyFilename: 'circuit_30_vkey.json',
  },
  31: {
    id: 31,
    name: 'MarketBetWithPoI',
    constraints: 5212,
    publicInputs: ['market_id', 'bet_commitment', 'max_bet', 'timestamp', 'insider_blacklist_root'],
    description: 'Private bet with insider-trading blacklist proof',
    vkeyFilename: 'circuit_31_vkey.json',
  },
  32: {
    id: 32,
    name: 'MarketClaim',
    constraints: 510,
    publicInputs: ['market_id', 'winning_outcome', 'bet_commitment', 'claim_nullifier', 'payout_amount'],
    description: 'Claim winnings proof with double-claim prevention',
    vkeyFilename: 'circuit_32_vkey.json',
  },
  40: {
    id: 40,
    name: 'PortfolioCompliance',
    constraints: 4883,
    publicInputs: ['compliance_root', 'threshold', 'timestamp'],
    description: 'Regulatory compliance proof (no exposure to sanctions list)',
    vkeyFilename: 'circuit_40_vkey.json',
  },
  41: {
    id: 41,
    name: 'PortfolioNetWorth',
    constraints: 5189,
    publicInputs: ['min_threshold', 'price_oracle_root', 'timestamp', 'net_worth_commitment'],
    description: 'Prove net worth above threshold without revealing amount',
    vkeyFilename: 'circuit_41_vkey.json',
  },
};

export interface LoadVKeyOptions {
  /**
   * Base directory where verification keys live.
   * Defaults to repo root at circuits/verification_keys or env ZYBER_VKEY_DIR.
   */
  baseDir?: string;
}

function resolveDefaultVKeyDir(): string {
  if (process.env.ZYBER_VKEY_DIR) return process.env.ZYBER_VKEY_DIR;

  // Walk up to find circuits/verification_keys from current file location.
  for (let i = 0; i < 7; i += 1) {
    const candidate = path.resolve(__dirname, ...Array(i).fill('..'), 'circuits/verification_keys');
    if (fs.existsSync(candidate)) return candidate;
  }

  // Fallback to cwd (useful during tests).
  const cwdCandidate = path.resolve(process.cwd(), 'circuits/verification_keys');
  if (fs.existsSync(cwdCandidate)) return cwdCandidate;

  // Last resort: leave the deepest expected path (repo root relative).
  return path.resolve(__dirname, '../../../../../circuits/verification_keys');
}

export function getCircuitMetadata(id: number | ZkCircuitType): CircuitMetadata | undefined {
  return CIRCUITS[Number(id)];
}

export function listCircuits(): CircuitMetadata[] {
  return Object.values(CIRCUITS);
}

export function getVKeyPath(id: number | ZkCircuitType, opts: LoadVKeyOptions = {}): string {
  const meta = getCircuitMetadata(id);
  if (!meta) {
    throw new Error(`Unknown circuit id ${id}`);
  }
  const baseDir = opts.baseDir ?? resolveDefaultVKeyDir();
  return path.resolve(baseDir, meta.vkeyFilename);
}

export async function loadVerificationKey<T = unknown>(
  id: number | ZkCircuitType,
  opts: LoadVKeyOptions = {}
): Promise<T> {
  const vkeyPath = getVKeyPath(id, opts);
  const raw = await fsp.readFile(vkeyPath, 'utf-8');
  return JSON.parse(raw) as T;
}
