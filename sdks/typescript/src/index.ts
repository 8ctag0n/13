// ZyberLink SDK for TypeScript
// Privacy-preserving FHE computation on Solana

export { Zyber, BatchBuilder } from './client';

export {
  // Network
  type NetworkName,
  type NetworkConfig,
  NETWORKS,

  // Operations
  type FheOperationType,
  type PredicateOp,
  type FhePredicate,
  type FheOperation,

  // Jobs
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
} from './types';
