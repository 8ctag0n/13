import { ZkClient, ZkCircuitType } from '@zyber/core';

export interface Position {
  marketId: string;
  outcome: number;
  amount: bigint;
  commitment: string;
  timestamp: number;
}

export interface Payoff {
  commitment: string;
  amount: bigint;
  outcome: number;
  isWinner: boolean;
}

export interface SettlementResult {
  marketId: string;
  winningOutcome: number;
  totalPayout: bigint;
  winnerCount: number;
  payoffs: Payoff[];
  timestamp: number;
}

/**
 * Handles market settlement and payout calculation.
 */
export class Settlement {
  private settlements: Map<string, SettlementResult> = new Map();

  constructor(private zk: ZkClient) {}

  /**
   * Settle a market with the winning outcome.
   */
  async settleMarket(
    marketId: string,
    outcome: number,
    positions: Position[],
  ): Promise<SettlementResult> {
    // Check if already settled
    if (this.settlements.has(marketId)) {
      throw new Error(`Market ${marketId} is already settled`);
    }

    // Calculate payoffs
    const payoffs = await this.calculatePayoffs(positions, outcome);

    const result: SettlementResult = {
      marketId,
      winningOutcome: outcome,
      totalPayout: payoffs.reduce((sum, p) => sum + p.amount, 0n),
      winnerCount: payoffs.filter((p) => p.isWinner).length,
      payoffs,
      timestamp: Date.now(),
    };

    this.settlements.set(marketId, result);
    return result;
  }

  /**
   * Calculate payoffs for all positions based on winning outcome.
   */
  async calculatePayoffs(
    positions: Position[],
    outcome: number,
  ): Promise<Payoff[]> {
    const payoffs: Payoff[] = [];

    // Separate winners and losers
    const winners = positions.filter((p) => p.outcome === outcome);
    const losers = positions.filter((p) => p.outcome !== outcome);

    // Calculate total pool
    const winnerPool = winners.reduce((sum, p) => sum + p.amount, 0n);
    const loserPool = losers.reduce((sum, p) => sum + p.amount, 0n);
    const totalPool = winnerPool + loserPool;

    if (totalPool === 0n) {
      return payoffs;
    }

    // Winners get back their stake + proportional share of loser pool
    for (const position of winners) {
      let payout = position.amount; // Return stake

      if (winnerPool > 0n) {
        // Add proportional winnings
        const share = (position.amount * loserPool) / winnerPool;
        payout += share;
      }

      payoffs.push({
        commitment: position.commitment,
        amount: payout,
        outcome: position.outcome,
        isWinner: true,
      });
    }

    // Losers get nothing (but we still record them)
    for (const position of losers) {
      payoffs.push({
        commitment: position.commitment,
        amount: 0n,
        outcome: position.outcome,
        isWinner: false,
      });
    }

    return payoffs;
  }

  /**
   * Get settlement result for a market.
   */
  async getSettlement(marketId: string): Promise<SettlementResult | null> {
    return this.settlements.get(marketId) ?? null;
  }

  /**
   * Verify a claim proof and determine payout eligibility.
   */
  async verifyClaimEligibility(
    marketId: string,
    commitment: string,
  ): Promise<{ eligible: boolean; amount: bigint }> {
    const settlement = this.settlements.get(marketId);
    if (!settlement) {
      return { eligible: false, amount: 0n };
    }

    const payoff = settlement.payoffs.find((p) => p.commitment === commitment);
    if (!payoff || !payoff.isWinner) {
      return { eligible: false, amount: 0n };
    }

    return {
      eligible: true,
      amount: payoff.amount,
    };
  }

  /**
   * Generate a settlement proof for audit purposes.
   * This would create a ZK proof that settlement was calculated correctly.
   */
  async generateSettlementProof(
    marketId: string,
  ): Promise<{ proof: Uint8Array; publicInputs: string[] }> {
    const settlement = this.settlements.get(marketId);
    if (!settlement) {
      throw new Error(`No settlement found for market ${marketId}`);
    }

    // Build witness for settlement proof
    const witnessData = this.buildSettlementWitness(settlement);

    // Submit to zk-generator (would need a SettlementVerification circuit)
    await this.zk.createJob({
      circuitType: ZkCircuitType.MarketClaim, // Placeholder
      witnessData,
    });

    // TODO: Poll for job completion and retrieve proof
    return {
      proof: new Uint8Array(0),
      publicInputs: [
        marketId,
        settlement.winningOutcome.toString(),
        settlement.totalPayout.toString(),
      ],
    };
  }

  /**
   * Build witness data for settlement proof.
   */
  private buildSettlementWitness(result: SettlementResult): Uint8Array {
    const witnessObject = {
      marketId: result.marketId,
      winningOutcome: result.winningOutcome,
      totalPayout: result.totalPayout.toString(),
      winnerCount: result.winnerCount,
      timestamp: result.timestamp,
    };

    return new TextEncoder().encode(JSON.stringify(witnessObject));
  }

  /**
   * Calculate market house edge or fees.
   */
  async calculateFees(
    totalVolume: bigint,
    feePercentage: number,
  ): Promise<bigint> {
    if (feePercentage < 0 || feePercentage > 100) {
      throw new Error('Fee percentage must be between 0 and 100');
    }

    return (totalVolume * BigInt(Math.floor(feePercentage * 100))) / 10000n;
  }

  /**
   * Get payout statistics for a settled market.
   */
  async getPayoutStats(marketId: string): Promise<{
    totalPaid: bigint;
    averagePayout: bigint;
    maxPayout: bigint;
    winRate: number;
  } | null> {
    const settlement = this.settlements.get(marketId);
    if (!settlement) {
      return null;
    }

    const winningPayoffs = settlement.payoffs.filter((p) => p.isWinner);
    if (winningPayoffs.length === 0) {
      return {
        totalPaid: 0n,
        averagePayout: 0n,
        maxPayout: 0n,
        winRate: 0,
      };
    }

    const totalPaid = winningPayoffs.reduce((sum, p) => sum + p.amount, 0n);
    const maxPayout = winningPayoffs.reduce(
      (max, p) => (p.amount > max ? p.amount : max),
      0n,
    );

    return {
      totalPaid,
      averagePayout: totalPaid / BigInt(winningPayoffs.length),
      maxPayout,
      winRate: winningPayoffs.length / settlement.payoffs.length,
    };
  }
}
