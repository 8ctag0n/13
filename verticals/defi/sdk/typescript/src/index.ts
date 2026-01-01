import {
  buildMarketBetWitness,
  buildMarketClaimWitness,
  type Groth16Proof,
  type LoadVKeyOptions,
  type MarketBetWitnessInput,
  type MarketClaimWitnessInput,
  verifyGroth16,
  ZkClient,
  ZkCircuitType,
} from '@zyber/core';

export interface CreateMarketParams {
  marketId: string;
  outcomes: string[];
  maxBet: bigint;
  closingTime: Date;
  requirePoI?: boolean;
}

export interface PlaceBetParams {
  marketId: Uint8Array;
  amount: bigint;
  outcome: number;
  nullifier: Uint8Array;
  insiderProof?: Uint8Array;
  insiderBlacklistRoot?: string;
  witnessData?: Uint8Array;
}

export interface ClaimParams {
  marketId: Uint8Array;
  winningOutcome: number;
  betCommitment: Uint8Array;
  claimNullifier: Uint8Array;
  payoutAmount: bigint;
  witnessData?: Uint8Array;
}

export interface MarketProof {
  proof: Groth16Proof;
  publicSignals: Array<string | bigint>;
}

export interface VerifyBetParams {
  requirePoI?: boolean;
  bet: MarketProof;
  loadOptions?: LoadVKeyOptions;
}

export interface VerifyClaimParams {
  claim: MarketProof;
  loadOptions?: LoadVKeyOptions;
}

/**
 * Manager for anonymous markets. Proving is delegated to zk-generator via ZkClient,
 * and local verification uses Groth16 with bundled VKeys.
 */
export class AnonMarkets {
  constructor(private zk: ZkClient) {}

  static async connect(zk: ZkClient): Promise<AnonMarkets> {
    return new AnonMarkets(zk);
  }

  async placeBet(params: PlaceBetParams): Promise<void> {
    const circuit = params.insiderBlacklistRoot
      ? ZkCircuitType.MarketBetWithPoI
      : ZkCircuitType.MarketBet;
    const witness =
      params.witnessData ??
      buildMarketBetWitness({
        marketId: params.marketId,
        outcome: params.outcome,
        amount: params.amount,
        nullifier: params.nullifier,
        eligibilityProof: params.insiderProof,
      } satisfies MarketBetWitnessInput);
    await this.zk.createJob({ circuitType: circuit, witnessData: witness });
  }

  async claimWinnings(params: ClaimParams): Promise<void> {
    const witness =
      params.witnessData ??
      buildMarketClaimWitness({
        marketId: params.marketId,
        winningOutcome: params.winningOutcome,
        betCommitment: params.betCommitment,
        claimNullifier: params.claimNullifier,
        payoutAmount: params.payoutAmount,
      } satisfies MarketClaimWitnessInput);
    await this.zk.createJob({ circuitType: ZkCircuitType.MarketClaim, witnessData: witness });
  }

  async verifyBet(params: VerifyBetParams): Promise<boolean> {
    const circuit = params.requirePoI ? ZkCircuitType.MarketBetWithPoI : ZkCircuitType.MarketBet;
    return verifyGroth16({
      circuitId: circuit,
      proof: params.bet.proof,
      publicSignals: params.bet.publicSignals,
      loadOptions: params.loadOptions,
    });
  }

  async verifyClaim(params: VerifyClaimParams): Promise<boolean> {
    return verifyGroth16({
      circuitId: ZkCircuitType.MarketClaim,
      proof: params.claim.proof,
      publicSignals: params.claim.publicSignals,
      loadOptions: params.loadOptions,
    });
  }
}

// Export lifecycle management
export { MarketFactory, type Market, type MarketState, type Position } from './market-factory';
export {
  Settlement,
  type Payoff,
  type SettlementResult,
} from './settlement';
