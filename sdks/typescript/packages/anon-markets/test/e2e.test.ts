import { describe, it, expect, beforeEach } from 'vitest';
import { ZkClient } from '@zyber/core';
import { MarketFactory, Settlement } from '../src';

describe('AnonMarkets E2E', () => {
  let zkClient: ZkClient;
  let marketFactory: MarketFactory;
  let settlement: Settlement;

  beforeEach(() => {
    zkClient = {
      createJob: async () => ({ jobId: 'test-job-id' }),
      getJobStatus: async () => ({ status: 'completed', proof: null }),
    } as unknown as ZkClient;

    marketFactory = new MarketFactory(zkClient);
    settlement = new Settlement(zkClient);
  });

  describe('Market Creation', () => {
    it('should create a market', async () => {
      const market = await marketFactory.createMarket({
        marketId: 'market-001',
        outcomes: ['Team A', 'Team B'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      expect(market.id).toBe('market-001');
      expect(market.outcomes).toHaveLength(2);
      expect(market.status).toBe('open');
    });

    it('should reject market with less than 2 outcomes', async () => {
      await expect(
        marketFactory.createMarket({
          marketId: 'market-invalid',
          outcomes: ['Only One'],
          maxBet: 1000n,
          closingTime: new Date(Date.now() + 86400000),
        }),
      ).rejects.toThrow('at least 2 outcomes');
    });

    it('should reject market with past closing time', async () => {
      await expect(
        marketFactory.createMarket({
          marketId: 'market-past',
          outcomes: ['A', 'B'],
          maxBet: 1000n,
          closingTime: new Date(Date.now() - 1000),
        }),
      ).rejects.toThrow('must be in the future');
    });
  });

  describe('Position Management', () => {
    it('should record positions', async () => {
      await marketFactory.createMarket({
        marketId: 'market-002',
        outcomes: ['Yes', 'No'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      await marketFactory.recordPosition('market-002', 0, 100n, 'commitment-1');
      await marketFactory.recordPosition('market-002', 1, 200n, 'commitment-2');

      const positions = await marketFactory.getMarketPositions('market-002');
      expect(positions).toHaveLength(2);
      expect(positions[0].amount).toBe(100n);
      expect(positions[1].amount).toBe(200n);
    });

    it('should update market totals on position recording', async () => {
      await marketFactory.createMarket({
        marketId: 'market-003',
        outcomes: ['A', 'B'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      await marketFactory.recordPosition('market-003', 0, 150n, 'c1');
      await marketFactory.recordPosition('market-003', 1, 250n, 'c2');

      const state = await marketFactory.getMarketState('market-003');
      expect(state.totalVolume).toBe(400n);
      expect(state.totalBets).toBe(2);
    });

    it('should reject positions on closed markets', async () => {
      await marketFactory.createMarket({
        marketId: 'market-004',
        outcomes: ['A', 'B'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      await marketFactory.closeMarket('market-004');

      await expect(
        marketFactory.recordPosition('market-004', 0, 100n, 'c1'),
      ).rejects.toThrow('not open for betting');
    });
  });

  describe('Market State', () => {
    it('should get market state with volumes', async () => {
      await marketFactory.createMarket({
        marketId: 'market-005',
        outcomes: ['X', 'Y', 'Z'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      await marketFactory.recordPosition('market-005', 0, 100n, 'c1');
      await marketFactory.recordPosition('market-005', 0, 150n, 'c2');
      await marketFactory.recordPosition('market-005', 1, 200n, 'c3');

      const state = await marketFactory.getMarketState('market-005');

      expect(state.outcomeVolumes.get(0)).toBe(250n);
      expect(state.outcomeVolumes.get(1)).toBe(200n);
      expect(state.outcomeVolumes.get(2)).toBeUndefined();
    });

    it('should calculate market odds', async () => {
      await marketFactory.createMarket({
        marketId: 'market-006',
        outcomes: ['A', 'B'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      await marketFactory.recordPosition('market-006', 0, 300n, 'c1');
      await marketFactory.recordPosition('market-006', 1, 700n, 'c2');

      const odds = await marketFactory.getMarketOdds('market-006');

      expect(odds.get(0)).toBe(0.3); // 30% probability
      expect(odds.get(1)).toBe(0.7); // 70% probability
    });

    it('should return equal odds for market with no bets', async () => {
      await marketFactory.createMarket({
        marketId: 'market-007',
        outcomes: ['A', 'B'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      const odds = await marketFactory.getMarketOdds('market-007');

      expect(odds.get(0)).toBe(0.5);
      expect(odds.get(1)).toBe(0.5);
    });
  });

  describe('Settlement', () => {
    it('should settle market and calculate payoffs', async () => {
      const positions = [
        { marketId: 'm1', outcome: 0, amount: 100n, commitment: 'winner1', timestamp: Date.now() },
        { marketId: 'm1', outcome: 0, amount: 200n, commitment: 'winner2', timestamp: Date.now() },
        { marketId: 'm1', outcome: 1, amount: 300n, commitment: 'loser1', timestamp: Date.now() },
      ];

      const result = await settlement.settleMarket('m1', 0, positions);

      expect(result.winningOutcome).toBe(0);
      expect(result.winnerCount).toBe(2);
      expect(result.payoffs).toHaveLength(3);

      const winner1 = result.payoffs.find((p) => p.commitment === 'winner1');
      const winner2 = result.payoffs.find((p) => p.commitment === 'winner2');
      const loser = result.payoffs.find((p) => p.commitment === 'loser1');

      expect(winner1?.isWinner).toBe(true);
      expect(winner2?.isWinner).toBe(true);
      expect(loser?.isWinner).toBe(false);
      expect(loser?.amount).toBe(0n);
    });

    it('should distribute loser pool proportionally to winners', async () => {
      const positions = [
        { marketId: 'm2', outcome: 0, amount: 100n, commitment: 'w1', timestamp: Date.now() },
        { marketId: 'm2', outcome: 0, amount: 100n, commitment: 'w2', timestamp: Date.now() },
        { marketId: 'm2', outcome: 1, amount: 200n, commitment: 'l1', timestamp: Date.now() },
      ];

      const payoffs = await settlement.calculatePayoffs(positions, 0);

      const w1 = payoffs.find((p) => p.commitment === 'w1');
      const w2 = payoffs.find((p) => p.commitment === 'w2');

      // Each winner: 100 stake + (100/200 * 200) = 100 + 100 = 200
      expect(w1?.amount).toBe(200n);
      expect(w2?.amount).toBe(200n);
    });

    it('should verify claim eligibility', async () => {
      const positions = [
        { marketId: 'm3', outcome: 0, amount: 100n, commitment: 'c1', timestamp: Date.now() },
        { marketId: 'm3', outcome: 1, amount: 200n, commitment: 'c2', timestamp: Date.now() },
      ];

      await settlement.settleMarket('m3', 0, positions);

      const eligible = await settlement.verifyClaimEligibility('m3', 'c1');
      const notEligible = await settlement.verifyClaimEligibility('m3', 'c2');

      expect(eligible.eligible).toBe(true);
      expect(eligible.amount).toBeGreaterThan(0n);
      expect(notEligible.eligible).toBe(false);
      expect(notEligible.amount).toBe(0n);
    });

    it('should calculate payout statistics', async () => {
      const positions = [
        { marketId: 'm4', outcome: 0, amount: 100n, commitment: 'w1', timestamp: Date.now() },
        { marketId: 'm4', outcome: 0, amount: 200n, commitment: 'w2', timestamp: Date.now() },
        { marketId: 'm4', outcome: 1, amount: 150n, commitment: 'l1', timestamp: Date.now() },
      ];

      await settlement.settleMarket('m4', 0, positions);
      const stats = await settlement.getPayoutStats('m4');

      expect(stats).not.toBeNull();
      expect(stats!.winRate).toBe(2 / 3); // 2 winners out of 3 total
      expect(stats!.totalPaid).toBeGreaterThan(0n);
    });

    it('should prevent double settlement', async () => {
      const positions = [
        { marketId: 'm5', outcome: 0, amount: 100n, commitment: 'c1', timestamp: Date.now() },
      ];

      await settlement.settleMarket('m5', 0, positions);

      await expect(
        settlement.settleMarket('m5', 0, positions),
      ).rejects.toThrow('already settled');
    });
  });

  describe('Complete Market Workflow', () => {
    it('should handle full market lifecycle', async () => {
      // Create market
      const market = await marketFactory.createMarket({
        marketId: 'market-full',
        outcomes: ['Winner', 'Loser'],
        maxBet: 1000n,
        closingTime: new Date(Date.now() + 86400000),
      });

      expect(market.status).toBe('open');

      // Record bets
      await marketFactory.recordPosition('market-full', 0, 100n, 'bet1');
      await marketFactory.recordPosition('market-full', 0, 150n, 'bet2');
      await marketFactory.recordPosition('market-full', 1, 200n, 'bet3');

      // Check state
      const state = await marketFactory.getMarketState('market-full');
      expect(state.totalBets).toBe(3);
      expect(state.totalVolume).toBe(450n);

      // Close market
      await marketFactory.closeMarket('market-full');
      const closedState = await marketFactory.getMarketState('market-full');
      expect(closedState.isOpen).toBe(false);
      expect(closedState.canSettle).toBe(true);

      // Settle market
      const positions = await marketFactory.getMarketPositions('market-full');
      const settlementResult = await settlement.settleMarket('market-full', 0, positions);

      expect(settlementResult.winningOutcome).toBe(0);
      expect(settlementResult.winnerCount).toBe(2);

      // Mark as settled
      await marketFactory.markAsSettled('market-full');
      const finalMarket = await marketFactory.getMarket('market-full');
      expect(finalMarket.status).toBe('settled');

      // Verify claims
      const claim1 = await settlement.verifyClaimEligibility('market-full', 'bet1');
      const claim3 = await settlement.verifyClaimEligibility('market-full', 'bet3');

      expect(claim1.eligible).toBe(true);
      expect(claim3.eligible).toBe(false);
    });
  });
});
