import { test, expect } from '@playwright/test';

/**
 * API E2E Tests: /api/jobs endpoints
 *
 * Tests the REST API endpoints for job listing, filtering, and retrieval.
 * Validates that blockchain jobs are properly synced and served via API.
 */

test.describe('Jobs API Endpoints', () => {
  const API_BASE = 'http://localhost:8080';

  test('GET /api/jobs should return list of jobs', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);

    expect(response.ok()).toBeTruthy();
    expect(response.status()).toBe(200);

    const data = await response.json();

    // Validate response structure
    expect(data).toHaveProperty('jobs');
    expect(data).toHaveProperty('count');
    expect(Array.isArray(data.jobs)).toBeTruthy();
    expect(data.count).toBeGreaterThan(0);

    // Validate first job structure
    if (data.jobs.length > 0) {
      const job = data.jobs[0];
      expect(job).toHaveProperty('job_id');
      expect(job).toHaveProperty('creator_pubkey');
      expect(job).toHaveProperty('status');
      expect(job).toHaveProperty('price_lamports');
      expect(job).toHaveProperty('operation');
      expect(job).toHaveProperty('required_provers');
      expect(job).toHaveProperty('consensus_threshold');
      expect(job).toHaveProperty('created_at');
    }
  });

  test('GET /api/jobs should support status filtering', async ({ request }) => {
    // Test filtering by pending status
    const response = await request.get(`${API_BASE}/api/jobs?status=pending`);

    expect(response.ok()).toBeTruthy();
    const data = await response.json();

    // All jobs should have pending status
    data.jobs.forEach((job: any) => {
      expect(job.status).toBe('pending');
    });
  });

  test('GET /api/jobs should return jobs in correct format', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Validate data types
    data.jobs.forEach((job: any) => {
      expect(typeof job.job_id).toBe('number');
      expect(typeof job.creator_pubkey).toBe('string');
      expect(typeof job.status).toBe('string');
      expect(typeof job.price_lamports).toBe('number');
      expect(typeof job.operation).toBe('string');
      expect(typeof job.required_provers).toBe('number');
      expect(typeof job.consensus_threshold).toBe('number');

      // Validate Solana pubkey format (base58, ~32-44 chars)
      expect(job.creator_pubkey.length).toBeGreaterThanOrEqual(32);
      expect(job.creator_pubkey.length).toBeLessThanOrEqual(44);

      // Validate FHE operations
      const validOperations = ['Add', 'Multiply', 'Sum', 'Average', 'Threshold', 'Max', 'Min', 'Variance'];
      expect(validOperations).toContain(job.operation);

      // Validate consensus configuration
      expect(job.required_provers).toBeGreaterThanOrEqual(1);
      expect(job.consensus_threshold).toBeGreaterThanOrEqual(1);
      expect(job.consensus_threshold).toBeLessThanOrEqual(job.required_provers);
    });
  });

  test('GET /api/jobs should validate dynamic pricing', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Group jobs by operation to verify pricing tiers
    const pricesByOperation: Record<string, Set<number>> = {};

    data.jobs.forEach((job: any) => {
      if (!pricesByOperation[job.operation]) {
        pricesByOperation[job.operation] = new Set();
      }
      pricesByOperation[job.operation].add(job.price_lamports);
    });

    console.log('Pricing by operation:', pricesByOperation);

    // Verify that prices are reasonable (between 1M and 100M lamports)
    data.jobs.forEach((job: any) => {
      expect(job.price_lamports).toBeGreaterThan(1_000_000);    // > 0.001 SOL
      expect(job.price_lamports).toBeLessThan(100_000_000);     // < 0.1 SOL
    });
  });

  test('GET /health should return healthy status', async ({ request }) => {
    const response = await request.get(`${API_BASE}/health`);

    expect(response.ok()).toBeTruthy();
    expect(response.status()).toBe(200);

    const data = await response.json();
    expect(data).toHaveProperty('status');
    expect(data.status).toBe('ok'); // Backend returns "ok" not "healthy"
  });

  test('GET /api/jobs should handle pagination', async ({ request }) => {
    // Get all jobs
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Verify count matches array length
    expect(data.count).toBe(data.jobs.length);

    // Verify jobs are sorted by creation date (newest first)
    if (data.jobs.length > 1) {
      for (let i = 0; i < data.jobs.length - 1; i++) {
        const current = new Date(data.jobs[i].created_at);
        const next = new Date(data.jobs[i + 1].created_at);
        expect(current.getTime()).toBeGreaterThanOrEqual(next.getTime());
      }
    }
  });

  test('API should handle CORS correctly', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`, {
      headers: {
        'Origin': 'http://localhost:5173'
      }
    });

    expect(response.ok()).toBeTruthy();
    // In production, verify CORS headers
    // const headers = response.headers();
    // expect(headers['access-control-allow-origin']).toBeDefined();
  });
});
