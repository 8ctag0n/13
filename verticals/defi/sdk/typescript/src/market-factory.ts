import { ZkClient } from '@zyber/core';

export interface Market {
  id: string;
  outcomes: string[];
  maxBet: bigint;
  closingTime: number;
  settlementTime?: number;
  status: 'open' | 'closed' | 'settled';
  requirePoI: boolean;
  createdAt: number;
  totalVolume?: bigint;
  positionCount?: number;
}

export interface CreateMarketParams {
  marketId: string;
  outcomes: string[];
  maxBet: bigint;
  closingTime: Date;
  settlementTime?: Date;
  requirePoI?: boolean;
}

export interface MarketState {
  market: Market;
  totalBets: number;
  totalVolume: bigint;
  isOpen: boolean;
  canSettle: boolean;
  outcomeVolumes: Map<number, bigint>;
}

export interface Position {
  marketId: string;
  outcome: number;
  amount: bigint;
  commitment: string;
  timestamp: number;
}

/**
 * Factory for creating and managing prediction markets.
 */
export class MarketFactory {
  private markets: Map<string, Market> = new Map();
  private positions: Map<string, Position[]> = new Map(); // marketId -> positions

  constructor(private zk: ZkClient) {}

  /**
   * Create a new prediction market.
   */
  async createMarket(params: CreateMarketParams): Promise<Market> {
    if (params.outcomes.length < 2) {
      throw new Error('Market must have at least 2 outcomes');
    }

    if (params.closingTime.getTime() <= Date.now()) {
      throw new Error('Closing time must be in the future');
    }

    const market: Market = {
      id: params.marketId,
      outcomes: params.outcomes,
      maxBet: params.maxBet,
      closingTime: params.closingTime.getTime(),
      settlementTime: params.settlementTime?.getTime(),
      status: 'open',
      requirePoI: params.requirePoI ?? false,
      createdAt: Date.now(),
      totalVolume: 0n,
      positionCount: 0,
    };

    this.markets.set(params.marketId, market);
    this.positions.set(params.marketId, []);

    return market;
  }

  /**
   * Get current market state and statistics.
   */
  async getMarketState(marketId: string): Promise<MarketState> {
    const market = this.markets.get(marketId);
    if (!market) {
      throw new Error(`Market ${marketId} not found`);
    }

    // Auto-close if past closing time
    if (market.status === 'open' && Date.now() > market.closingTime) {
      market.status = 'closed';
      this.markets.set(marketId, market);
    }

    const positions = this.positions.get(marketId) ?? [];
    const outcomeVolumes = new Map<number, bigint>();

    // Calculate volumes per outcome
    for (const position of positions) {
      const current = outcomeVolumes.get(position.outcome) ?? 0n;
      outcomeVolumes.set(position.outcome, current + position.amount);
    }

    const totalVolume = positions.reduce(
      (sum, pos) => sum + pos.amount,
      0n,
    );

    return {
      market,
      totalBets: positions.length,
      totalVolume,
      isOpen: market.status === 'open',
      canSettle: market.status === 'closed',
      outcomeVolumes,
    };
  }

  /**
   * Get market by ID.
   */
  async getMarket(marketId: string): Promise<Market> {
    const market = this.markets.get(marketId);
    if (!market) {
      throw new Error(`Market ${marketId} not found`);
    }
    return market;
  }

  /**
   * Close market for new bets.
   */
  async closeMarket(marketId: string): Promise<void> {
    const market = this.markets.get(marketId);
    if (!market) {
      throw new Error(`Market ${marketId} not found`);
    }

    if (market.status !== 'open') {
      throw new Error(`Market ${marketId} is already ${market.status}`);
    }

    market.status = 'closed';
    this.markets.set(marketId, market);
  }

  /**
   * Mark market as settled after outcome resolution.
   */
  async markAsSettled(marketId: string): Promise<void> {
    const market = this.markets.get(marketId);
    if (!market) {
      throw new Error(`Market ${marketId} not found`);
    }

    if (market.status !== 'closed') {
      throw new Error(`Cannot settle market ${marketId} in status ${market.status}`);
    }

    market.status = 'settled';
    market.settlementTime = Date.now();
    this.markets.set(marketId, market);
  }

  /**
   * Record a new position (called after successful bet placement).
   */
  async recordPosition(
    marketId: string,
    outcome: number,
    amount: bigint,
    commitment: string,
  ): Promise<void> {
    const market = this.markets.get(marketId);
    if (!market) {
      throw new Error(`Market ${marketId} not found`);
    }

    if (market.status !== 'open') {
      throw new Error(`Market ${marketId} is not open for betting`);
    }

    if (outcome >= market.outcomes.length) {
      throw new Error(`Invalid outcome ${outcome} for market ${marketId}`);
    }

    const positions = this.positions.get(marketId) ?? [];
    positions.push({
      marketId,
      outcome,
      amount,
      commitment,
      timestamp: Date.now(),
    });

    this.positions.set(marketId, positions);

    // Update market totals
    market.totalVolume = (market.totalVolume ?? 0n) + amount;
    market.positionCount = (market.positionCount ?? 0) + 1;
    this.markets.set(marketId, market);
  }

  /**
   * Get all positions for a market.
   */
  async getMarketPositions(marketId: string): Promise<Position[]> {
    return this.positions.get(marketId) ?? [];
  }

  /**
   * List all markets with optional status filter.
   */
  async listMarkets(status?: Market['status']): Promise<Market[]> {
    const allMarkets = Array.from(this.markets.values());
    return status ? allMarkets.filter((m) => m.status === status) : allMarkets;
  }

  /**
   * Get market odds based on current positions.
   */
  async getMarketOdds(marketId: string): Promise<Map<number, number>> {
    const state = await this.getMarketState(marketId);
    const odds = new Map<number, number>();

    if (state.totalVolume === 0n) {
      // Equal odds if no bets yet
      const equalOdds = 1 / state.market.outcomes.length;
      for (let i = 0; i < state.market.outcomes.length; i++) {
        odds.set(i, equalOdds);
      }
      return odds;
    }

    // Calculate implied probability from volume
    for (let i = 0; i < state.market.outcomes.length; i++) {
      const volume = state.outcomeVolumes.get(i) ?? 0n;
      const probability = Number(volume) / Number(state.totalVolume);
      odds.set(i, probability);
    }

    return odds;
  }
}
