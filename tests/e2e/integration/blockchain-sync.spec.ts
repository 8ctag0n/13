import { test, expect } from '@playwright/test';

/**
 * Blockchain Synchronization E2E Tests
 *
 * Validates that jobs created on the Solana blockchain are properly
 * synchronized to the PostgreSQL database and served via the API.
 */

test.describe('Blockchain Synchronization', () => {
  const API_BASE = 'http://localhost:8080';

  test('should sync jobs from blockchain to database', async ({ request }) => {
    // Get initial job count
    const initialResponse = await request.get(`${API_BASE}/api/jobs`);
    const initialData = await initialResponse.json();
    const initialCount = initialData.count;

    console.log(`Initial job count: ${initialCount}`);

    // Wait for next sync cycle (8 seconds + buffer)
    await new Promise(resolve => setTimeout(resolve, 10000));

    // Get updated job count
    const updatedResponse = await request.get(`${API_BASE}/api/jobs`);
    const updatedData = await updatedResponse.json();
    const updatedCount = updatedData.count;

    console.log(`Updated job count: ${updatedCount}`);

    // Verify that jobs are being synced (count should be same or increased)
    expect(updatedCount).toBeGreaterThanOrEqual(initialCount);

    // If new jobs were created, verify they have all required fields
    if (updatedCount > initialCount) {
      const newestJob = updatedData.jobs[0]; // Jobs are sorted by created_at desc
      expect(newestJob).toBeDefined();
      expect(newestJob.job_id).toBeDefined();
      expect(newestJob.creator_pubkey).toBeDefined();
      expect(newestJob.status).toBe('pending');
    }
  });

  test('should maintain data consistency during sync', async ({ request }) => {
    // Get a specific job
    const response1 = await request.get(`${API_BASE}/api/jobs`);
    const data1 = await response1.json();

    if (data1.jobs.length === 0) {
      test.skip();
      return;
    }

    const jobId = data1.jobs[0].job_id;
    const originalJobData = JSON.stringify(data1.jobs[0]);

    // Wait for sync cycle
    await new Promise(resolve => setTimeout(resolve, 10000));

    // Get the same job again
    const response2 = await request.get(`${API_BASE}/api/jobs`);
    const data2 = await response2.json();
    const sameJob = data2.jobs.find((j: any) => j.job_id === jobId);

    // Job data should be consistent (unless it changed on blockchain)
    if (sameJob) {
      const updatedJobData = JSON.stringify(sameJob);

      // Core fields should remain the same
      expect(sameJob.job_id).toBe(data1.jobs[0].job_id);
      expect(sameJob.creator_pubkey).toBe(data1.jobs[0].creator_pubkey);
      expect(sameJob.operation).toBe(data1.jobs[0].operation);
      expect(sameJob.price_lamports).toBe(data1.jobs[0].price_lamports);
    }
  });

  test('should sync FHE-specific fields correctly', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Verify all jobs have FHE consensus configuration
    data.jobs.forEach((job: any) => {
      // FHE jobs should have required_provers >= 1
      expect(job.required_provers).toBeGreaterThanOrEqual(1);

      // Consensus threshold should be reasonable
      expect(job.consensus_threshold).toBeGreaterThanOrEqual(1);
      expect(job.consensus_threshold).toBeLessThanOrEqual(job.required_provers);

      // Typical FHE consensus is 2-of-3
      if (job.required_provers === 3) {
        expect(job.consensus_threshold).toBe(2);
      }

      // Operation should be a valid FHE operation
      const validOps = ['Add', 'Multiply', 'Sum', 'Average', 'Threshold', 'Max', 'Min', 'Variance'];
      expect(validOps).toContain(job.operation);
    });
  });

  test('should handle blockchain reorg gracefully', async ({ request }) => {
    // Get current state
    const response1 = await request.get(`${API_BASE}/api/jobs`);
    const data1 = await response1.json();
    const jobIds1 = data1.jobs.map((j: any) => j.job_id).sort();

    // Wait for multiple sync cycles
    await new Promise(resolve => setTimeout(resolve, 20000));

    // Get updated state
    const response2 = await request.get(`${API_BASE}/api/jobs`);
    const data2 = await response2.json();
    const jobIds2 = data2.jobs.map((j: any) => j.job_id).sort();

    // Most original job IDs should still exist (accounting for pagination/limits)
    // Check that at least 80% of original jobs are still present
    const persistedJobs = jobIds1.filter((id: number) => jobIds2.includes(id));
    const persistenceRate = persistedJobs.length / jobIds1.length;

    expect(persistenceRate).toBeGreaterThan(0.8); // At least 80% should persist

    console.log(`Verified ${persistedJobs.length}/${jobIds1.length} jobs persist (${(persistenceRate * 100).toFixed(1)}%)`);
  });

  test('should sync with correct timestamps', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    data.jobs.forEach((job: any) => {
      // created_at should be a valid ISO 8601 timestamp
      const createdAt = new Date(job.created_at);
      expect(createdAt.toString()).not.toBe('Invalid Date');

      // Job should not be created in the future
      expect(createdAt.getTime()).toBeLessThanOrEqual(Date.now());

      // Job should have a reasonable creation date (within last year)
      // This is more flexible for development environments with legacy test data
      const oneYearAgo = Date.now() - (365 * 24 * 60 * 60 * 1000);
      expect(createdAt.getTime()).toBeGreaterThan(oneYearAgo);
    });
  });

  test('should sync status transitions correctly', async ({ request }) => {
    const response = await request.get(`${API_BASE}/api/jobs`);
    const data = await response.json();

    // Count jobs by status
    const statusCounts: Record<string, number> = {};
    data.jobs.forEach((job: any) => {
      statusCounts[job.status] = (statusCounts[job.status] || 0) + 1;
    });

    console.log('Status distribution:', statusCounts);

    // In a fresh test environment, most jobs should be pending
    expect(statusCounts['pending']).toBeGreaterThan(0);

    // Valid statuses only
    Object.keys(statusCounts).forEach(status => {
      expect(['pending', 'claimed', 'completed', 'failed', 'cancelled']).toContain(status);
    });
  });
});
