import { defineConfig } from '@playwright/test';

/**
 * Playwright config for API-only tests (no browser extension required)
 * Use this for containerized CI/CD environments
 *
 * Run: npx playwright test --config=playwright.api.config.ts
 */
export default defineConfig({
  testDir: './e2e',
  testMatch: 'api-integration.spec.ts',
  timeout: 30000,
  retries: 1,
  workers: 4, // Can run in parallel since no browser state

  use: {
    trace: 'on-first-retry',
  },

  reporter: [
    ['list'],
    ['json', { outputFile: 'test-results/api-results.json' }],
  ],
});
