import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for ZyberLink E2E tests
 * Tests both API endpoints and full user flows
 */
export default defineConfig({
  testDir: './e2e-tests',

  // Run tests in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Opt out of parallel tests on CI
  workers: process.env.CI ? 1 : undefined,

  // Reporter to use
  reporter: [
    ['html'],
    ['list'],
    ['json', { outputFile: 'test-results/results.json' }]
  ],

  // Shared settings for all the projects below
  use: {
    // Base URL - point to webapp for UI tests, API for API tests
    baseURL: process.env.BASE_URL || 'http://localhost:5173',

    // Collect trace when retrying the failed test
    trace: 'on-first-retry',

    // Screenshot on failure
    screenshot: 'only-on-failure',

    // Video on failure
    video: 'retain-on-failure',
  },

  // Test timeout
  timeout: 30000,

  // Configure projects for major browsers (headless for server)
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Run headless on server
        headless: true,
      },
    },
  ],

  // Run your local dev server before starting the tests
  webServer: [
    // Backend API server
    {
      command: 'make start-backend',
      url: 'http://localhost:8080/health',
      timeout: 30 * 1000,
      reuseExistingServer: true,
    },
    // Frontend webapp (Svelte)
    {
      command: 'make start-frontend',
      url: 'http://localhost:5173',
      timeout: 30 * 1000,
      reuseExistingServer: true,
    },
  ],
});
