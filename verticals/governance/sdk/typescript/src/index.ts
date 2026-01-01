import {
  buildVoteWitness,
  type Groth16Proof,
  type LoadVKeyOptions,
  type VoteWitnessInput,
  verifyGroth16,
  ZkClient,
  ZkCircuitType,
} from '@zyber/core';

export interface CreatePollParams {
  pollId: string;
  eligibleVotersRoot: string;
  minBalance: bigint;
  requirePoI?: boolean;
}

export interface CastVoteParams {
  pollId: Uint8Array;
  voteChoice: number;
  nullifier: Uint8Array;
  eligibilityProof?: Uint8Array;
  // Optional PoI blacklist proof
  blacklistProof?: Uint8Array;
  witnessData?: Uint8Array;
}

export interface VoteProof {
  proof: Groth16Proof;
  publicSignals: Array<string | bigint>;
}

export interface VerifyVoteParams {
  pollId: string;
  vote: VoteProof;
  requirePoI?: boolean;
  loadOptions?: LoadVKeyOptions;
}

/**
 * Lightweight manager that delegates proving to the backend via ZkClient jobs
 * and uses local Groth16 verify for client-side validation when needed.
 */
export class PrivateVote {
  constructor(private zk: ZkClient) {}

  static async connect(zk: ZkClient): Promise<PrivateVote> {
    return new PrivateVote(zk);
  }

  /**
   * Submit a vote by creating a ZK job for circuit 20/21.
   * The caller must provide serialized witness data. This SDK does not yet build witnesses.
   */
  async castVote(params: CastVoteParams): Promise<void> {
    const circuit = params.blacklistProof ? ZkCircuitType.PrivateVoteWithPoI : ZkCircuitType.PrivateVote;
    const witness =
      params.witnessData ??
      buildVoteWitness({
        pollId: params.pollId,
        choice: params.voteChoice,
        nullifier: params.nullifier,
        eligibilityProof: params.eligibilityProof,
        blacklistProof: params.blacklistProof,
      } satisfies VoteWitnessInput);

    await this.zk.createJob({ circuitType: circuit, witnessData: witness });
  }

  /**
   * Client-side verification using snarkjs and the bundled verification key.
   */
  async verifyVote(params: VerifyVoteParams): Promise<boolean> {
    const circuit = params.requirePoI ? ZkCircuitType.PrivateVoteWithPoI : ZkCircuitType.PrivateVote;
    return verifyGroth16({
      circuitId: circuit,
      proof: params.vote.proof,
      publicSignals: params.vote.publicSignals,
      loadOptions: params.loadOptions,
    });
  }
}

// Export lifecycle management
export { PollManager, type Poll, type PollState } from './poll-manager';
export {
  VoteAggregator,
  type AggregatedResult,
  type TallyProof,
} from './vote-aggregator';
