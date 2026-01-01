// ZyberLink SDK for TypeScript
// Privacy-preserving FHE computation on Solana

export { Zyber, BatchBuilder } from './client';
export { ZkClient, MIN_PROVER_STAKE, type ZkClientOptions, type ProverStakeInfo, type CreateZkJobParams, type SubmitProofParams, type QuoteResponse } from './zk-client';
export { ProverClient, type ProverClientOptions, type FheJobSubmitResult } from './prover-client';
export { X402Client, createX402Client, getEstimatedPrice, type X402Quote, type X402PaymentToken, type X402Config } from './x402';
export { verifyGroth16, type Groth16Proof, type VerifyGroth16Params } from './proving';
export {
  MerkleTree,
  sha256HashBigint,
  poseidonHash,
  createPoseidonHashFn,
  loadPoseidon,
  isPoseidonAvailable,
  generateCommitment,
  generateNullifier,
  type MerkleProof,
  type HashFn,
} from './crypto';
export {
  type CircuitMetadata,
  type LoadVKeyOptions,
  listCircuits,
  getCircuitMetadata,
  getVKeyPath,
  loadVerificationKey,
} from './circuits';
export {
  buildVoteWitness,
  buildMarketBetWitness,
  buildMarketClaimWitness,
  buildProofOfInnocenceWitness,
  buildPortfolioComplianceWitness,
  buildNetWorthWitness,
  type VoteWitnessInput,
  type MarketBetWitnessInput,
  type MarketClaimWitnessInput,
  type ProofOfInnocenceWitnessInput,
  type PortfolioComplianceWitnessInput,
  type NetWorthWitnessInput,
} from './witness';

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
