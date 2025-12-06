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
