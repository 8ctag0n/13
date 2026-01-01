import { defineConfig } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const EXTENSION_PATH = path.join(__dirname, 'dist');

export default defineConfig({
  testDir: './e2e',
  timeout: 60000,
  retries: 0,
  workers: 1, // Extensions require serial execution

  use: {
    headless: false, // Extensions don't work in headless mode
    viewport: { width: 400, height: 600 },
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: {
        channel: 'chromium',
        // Extension context will be created in tests
        launchOptions: {
          args: [
            `--disable-extensions-except=${EXTENSION_PATH}`,
            `--load-extension=${EXTENSION_PATH}`,
            '--no-sandbox',
          ],
        },
      },
    },
  ],

  // Build extension before running tests
  webServer: {
    command: 'npm run build',
    reuseExistingServer: true,
    timeout: 30000,
  },
});
