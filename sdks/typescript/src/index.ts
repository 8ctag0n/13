// ZyberLink SDK for TypeScript
// Privacy-preserving FHE computation on Solana

export { Zyber, BatchBuilder } from './client';

export {
  // Network
  type NetworkName,
  type NetworkConfig,
  NETWORKS,

  // FHE Operations
  type FheOperationType,
  type PredicateOp,
  type FhePredicate,
  type FheOperation,

  // FHE Jobs
  type JobStatus,
  type Job,
  type JobOptions,

  // Cost
  type CostEstimate,
  costToSol,

  // Prepared Operations
  type PreparedOperation,
  type OperationPdas,

  // Batch
  type BatchResult,
  type BatchSummary,

  // Options
  type ZyberOptions,

  // ZK Circuit Types (v2.0)
  ZkCircuitType,
  isV2Circuit,
  requiresPoI,

  // ZK Jobs
  type ZkJob,
  type ZkJobOptions,

  // Dispute System
  DISPUTE_WINDOW_SECONDS,
  DISPUTE_BOND_LAMPORTS,
  DISPUTE_REWARD_BPS,
  type DisputeResult,
  type DisputeProofParams,
} from './types';
