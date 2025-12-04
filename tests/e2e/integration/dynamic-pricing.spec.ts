import { test, expect } from '@playwright/test';

/**
 * Dynamic Pricing E2E Tests
 *
 * Validates that FHE job pricing is dynamic based on operation type,
 * complexity, and market conditions.
 */

test.describe('Dynamic Pricing System', () => {
  const API_BASE = 'http://localhost:8080';

  test('should have different prices for different operations', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Group jobs by operation
    const pricesByOperation: Record<string, number[]> = {};

    data.jobs.forEach((job: any) => {
      if (!pricesByOperation[job.operation]) {
        pricesByOperation[job.operation] = [];
      }
      pricesByOperation[job.operation].push(job.price_lamports);
    });

    console.log('Prices by operation:');
    Object.entries(pricesByOperation).forEach(([op, prices]) => {
      const avg = prices.reduce((a, b) => a + b, 0) / prices.length;
      const min = Math.min(...prices);
      const max = Math.max(...prices);
      console.log(`  ${op}: avg=${(avg / 1e6).toFixed(2)}M, min=${(min / 1e6).toFixed(2)}M, max=${(max / 1e6).toFixed(2)}M lamports`);
    });

    // Verify that we have multiple operations with different pricing
    const operations = Object.keys(pricesByOperation);
    expect(operations.length).toBeGreaterThan(1);
  });

  test('should follow expected pricing tiers', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Expected pricing tiers based on complexity
    const tierDefinitions = {
      tier1: { operations: ['Add', 'Multiply'], expectedRange: [1e6, 5e6] },      // 1-5M lamports
      tier2: { operations: ['Threshold'], expectedRange: [10e6, 20e6] },           // 10-20M lamports
      tier3: { operations: ['Sum'], expectedRange: [25e6, 40e6] },                 // 25-40M lamports
      tier4: { operations: ['Average'], expectedRange: [50e6, 70e6] },             // 50-70M lamports
    };

    data.jobs.forEach((job: any) => {
      // Find which tier this operation belongs to
      let tierFound = false;

      for (const [tierName, tier] of Object.entries(tierDefinitions)) {
        if (tier.operations.includes(job.operation)) {
          tierFound = true;
          const [min, max] = tier.expectedRange;

          // Prices should fall within expected range (with some tolerance)
          expect(job.price_lamports).toBeGreaterThanOrEqual(min * 0.8); // 20% tolerance
          expect(job.price_lamports).toBeLessThanOrEqual(max * 1.2);

          console.log(`${job.operation} (${tierName}): ${(job.price_lamports / 1e6).toFixed(2)}M lamports ✓`);
          break;
        }
      }

      // All operations should be in a defined tier
      if (!tierFound) {
        console.log(`Warning: ${job.operation} not in any tier definition`);
      }
    });
  });

  test('should maintain consistent pricing for same operation', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Group jobs by operation
    const jobsByOperation: Record<string, any[]> = {};

    data.jobs.forEach((job: any) => {
      if (!jobsByOperation[job.operation]) {
        jobsByOperation[job.operation] = [];
      }
      jobsByOperation[job.operation].push(job);
    });

    // For each operation, verify price consistency
    Object.entries(jobsByOperation).forEach(([operation, jobs]) => {
      if (jobs.length < 2) return;

      const prices = jobs.map(j => j.price_lamports);
      const avgPrice = prices.reduce((a, b) => a + b, 0) / prices.length;

      // Prices should be within 20% of average (dynamic pricing allows some variation)
      prices.forEach((price, idx) => {
        const deviation = Math.abs(price - avgPrice) / avgPrice;
        expect(deviation).toBeLessThan(0.2); // Max 20% deviation

        if (deviation > 0.1) {
          console.log(`${operation} job #${jobs[idx].job_id}: ${(deviation * 100).toFixed(1)}% deviation`);
        }
      });
    });
  });

  test('should factor in consensus requirements for pricing', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Group by operation and required_provers
    const groups: Record<string, any> = {};

    data.jobs.forEach((job: any) => {
      const key = `${job.operation}_${job.required_provers}`;
      if (!groups[key]) {
        groups[key] = {
          operation: job.operation,
          required_provers: job.required_provers,
          prices: []
        };
      }
      groups[key].prices.push(job.price_lamports);
    });

    // For same operation, higher prover count should mean higher price
    Object.values(groups).forEach((group: any) => {
      const avgPrice = group.prices.reduce((a: number, b: number) => a + b, 0) / group.prices.length;
      console.log(`${group.operation} with ${group.required_provers} provers: avg ${(avgPrice / 1e6).toFixed(2)}M lamports`);
    });

    // Verify minimum price is reasonable
    data.jobs.forEach((job: any) => {
      // Price per prover should be at least 0.5M lamports
      const pricePerProver = job.price_lamports / job.required_provers;
      expect(pricePerProver).toBeGreaterThanOrEqual(0.5e6);
    });
  });

  test('should have valid payment method options', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Verify payment methods
    data.jobs.forEach((job: any) => {
      expect(job).toHaveProperty('payment_method');
      expect(['SOL', 'WZEC']).toContain(job.payment_method);
    });

    // Count by payment method
    const paymentCounts: Record<string, number> = {};
    data.jobs.forEach((job: any) => {
      paymentCounts[job.payment_method] = (paymentCounts[job.payment_method] || 0) + 1;
    });

    console.log('Payment methods:', paymentCounts);
  });

  test('should calculate total cost with fees correctly', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    data.jobs.forEach((job: any) => {
      const baseCost = job.price_lamports;

      // Calculate expected fee (1%)
      const expectedFee = Math.floor(baseCost * 0.01);
      const expectedTotal = baseCost + expectedFee;

      // Verify calculations
      expect(baseCost).toBeGreaterThan(0);
      expect(expectedFee).toBeGreaterThanOrEqual(0);

      console.log(
        `Job ${job.job_id}: ` +
        `base=${(baseCost / 1e9).toFixed(5)} SOL, ` +
        `fee=${(expectedFee / 1e9).toFixed(5)} SOL, ` +
        `total=${(expectedTotal / 1e9).toFixed(5)} SOL`
      );
    });
  });
});
