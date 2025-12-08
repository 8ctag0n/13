import type { Address, TransactionSigner } from '@solana/kit';

// =============================================================================
// Network Configuration
// =============================================================================

export type NetworkName = 'mainnet' | 'devnet' | 'localnet';

export interface NetworkConfig {
  name: NetworkName;
  rpcUrl: string;
  wsUrl: string;
  programId: Address;
  backendUrl: string;
}

export const NETWORKS: Record<NetworkName, NetworkConfig> = {
  mainnet: {
    name: 'mainnet',
    rpcUrl: 'https://api.mainnet-beta.solana.com',
    wsUrl: 'wss://api.mainnet-beta.solana.com',
    programId: 'ZYBRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' as Address, // TODO: Update
    backendUrl: 'https://api.zyberlink.io',
  },
  devnet: {
    name: 'devnet',
    rpcUrl: 'https://api.devnet.solana.com',
    wsUrl: 'wss://api.devnet.solana.com',
    programId: 'ZYBRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' as Address, // TODO: Update
    backendUrl: 'https://devnet-api.zyberlink.io',
  },
  localnet: {
    name: 'localnet',
    rpcUrl: 'http://localhost:8899',
    wsUrl: 'ws://localhost:8900',
    programId: 'ZYBRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' as Address, // TODO: Update
    backendUrl: 'http://localhost:3000',
  },
};

// =============================================================================
// FHE Operations
// =============================================================================

export type FheOperationType = 'Sum' | 'Average' | 'CountIf' | 'Threshold';

export type PredicateOp = '>' | '<' | '==' | '!=' | '>=' | '<=';

export interface FhePredicate {
  op: PredicateOp;
  value: number;
}

export interface FheOperation {
  type: FheOperationType;
  expectedCount?: number;
  predicate?: FhePredicate;
}

// =============================================================================
// Job Types
// =============================================================================

export type JobStatus = 'pending' | 'claimed' | 'computing' | 'completed' | 'failed' | 'cancelled';

export interface Job {
  id: number;
  creator: Address;
  operation: FheOperation;
  status: JobStatus;
  witnessCommitment: Uint8Array;
  priceLamports: bigint;
  createdAt: Date;
  completedAt?: Date;
  result?: number;
  error?: string;
}

export interface JobOptions {
  /** Price to pay provers in lamports (default: 10_000_000 = 0.01 SOL) */
  priceLamports?: bigint;
  /** Timeout in seconds (default: 120) */
  timeoutSeconds?: number;
  /** Number of required provers for consensus (default: 2) */
  requiredProvers?: number;
}

// =============================================================================
// Cost Estimation
// =============================================================================

export interface CostEstimate {
  /** Job creation cost (rent + compute) */
  jobCreationLamports: bigint;
  /** Price offered to provers */
  proverPaymentLamports: bigint;
  /** Estimated transaction fee */
  txFeeLamports: bigint;
  /** Total estimated cost */
  totalLamports: bigint;
}

export function costToSol(lamports: bigint): number {
  return Number(lamports) / 1_000_000_000;
}

// =============================================================================
// Prepared Operation
// =============================================================================

export interface PreparedOperation {
  /** Job ID */
  jobId: number;
  /** Operation type */
  operation: FheOperation;
  /** Encrypted values (base64) */
  encryptedData: string;
  /** Witness commitment */
  witnessCommitment: Uint8Array;
  /** PDAs involved */
  pdas: OperationPdas;
  /** Estimated cost */
  cost: CostEstimate;
}

export interface OperationPdas {
  job: Address;
  escrow: Address;
  fheConsensus: Address;
  config: Address;
}

// =============================================================================
// Batch Operations
// =============================================================================

export interface BatchResult {
  index: number;
  value?: number;
  error?: string;
  durationMs: number;
}

export interface BatchSummary {
  results: BatchResult[];
  totalDurationMs: number;
  successCount: number;
  failureCount: number;
}

// =============================================================================
// Client Options
// =============================================================================

export interface ZyberOptions {
  /** Network to connect to */
  network?: NetworkName;
  /** Custom RPC URL (overrides network default) */
  rpcUrl?: string;
  /** Custom WebSocket URL (overrides network default) */
  wsUrl?: string;
  /** Custom backend URL (overrides network default) */
  backendUrl?: string;
  /** Custom program ID (overrides network default) */
  programId?: Address;
  /** Default job options */
  defaultJobOptions?: JobOptions;
}

// =============================================================================
// ZK Circuit Types (v2.0 - for zk-generator program)
// =============================================================================

/**
 * ZK Circuit types supported by zk-generator program.
 *
 * IDs are grouped by category:
 * - 0-9: Legacy circuits (v1.x compatibility)
 * - 10-19: Core privacy primitives
 * - 20-29: Voting circuits
 * - 30-39: Market circuits
 * - 40-49: Portfolio circuits
 */
export enum ZkCircuitType {
  // Legacy circuits (v1.x)
  ZcashOrchard = 0,
  ZcashSapling = 1,
  AnonymousVote = 2,
  Credential = 3,

  // Core primitives (v2.0)
  ProofOfInnocence = 10,

  // Voting circuits (v2.0)
  PrivateVote = 20,
  PrivateVoteWithPoI = 21,

  // Market circuits (v2.0)
  MarketBet = 30,
  MarketBetWithPoI = 31,
  MarketClaim = 32,

  // Portfolio circuits (v2.0)
  PortfolioCompliance = 40,
  PortfolioNetWorth = 41,

  // Future
  FutarchyConditional = 50,
}

/** Check if circuit type is v2.0 */
export function isV2Circuit(circuitType: ZkCircuitType): boolean {
  return circuitType >= 10;
}

/** Check if circuit requires PoI integration */
export function requiresPoI(circuitType: ZkCircuitType): boolean {
  return [
    ZkCircuitType.PrivateVoteWithPoI,
    ZkCircuitType.MarketBetWithPoI,
    ZkCircuitType.PortfolioCompliance,
  ].includes(circuitType);
}

// =============================================================================
// ZK Job Types
// =============================================================================

export interface ZkJob {
  id: number;
  creator: Address;
  prover?: Address;
  circuitType: ZkCircuitType;
  status: JobStatus;
  witnessHash: Uint8Array;
  witnessSize: number;
  proofHash?: Uint8Array;
  priceLamports: bigint;
  createdAt: number;
  timeoutAt: number;
}

export interface ZkJobOptions {
  /** Price to pay prover in lamports */
  priceLamports?: bigint;
  /** Timeout in seconds */
  timeoutSeconds?: number;
}

// =============================================================================
// Dispute Types
// =============================================================================

/** Dispute window duration (24 hours in seconds) */
export const DISPUTE_WINDOW_SECONDS = 24 * 60 * 60;

/** Minimum bond required to dispute (0.1 SOL) */
export const DISPUTE_BOND_LAMPORTS = 100_000_000n;

/** Reward percentage for successful dispute (50%) */
export const DISPUTE_REWARD_BPS = 5000;

export interface DisputeResult {
  /** Whether the dispute was successful (proof was invalid) */
  proofInvalid: boolean;
  /** Amount slashed from prover (if proof invalid) */
  slashedAmount?: bigint;
  /** Reward given to disputor (if proof invalid) */
  disputorReward?: bigint;
  /** Bond forfeited (if proof valid) */
  bondForfeited?: bigint;
}

export interface DisputeProofParams {
  /** Job account address */
  jobAddress: Address;
  /** Full ZK proof bytes (256 bytes for Groth16) */
  proof: Uint8Array;
  /** Public inputs for circuit verification */
  publicInputs: Uint8Array;
}
