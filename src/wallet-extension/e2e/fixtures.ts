import { test as base, chromium, type BrowserContext } from '@playwright/test';
import path from 'path';

const EXTENSION_PATH = path.join(__dirname, '..', 'dist');

// Test fixtures for extension testing
export const test = base.extend<{
  context: BrowserContext;
  extensionId: string;
}>({
  // Create browser context with extension loaded
  context: async ({}, use) => {
    const context = await chromium.launchPersistentContext('', {
      headless: false,
      args: [
        `--disable-extensions-except=${EXTENSION_PATH}`,
        `--load-extension=${EXTENSION_PATH}`,
        '--no-sandbox',
        '--disable-gpu',
      ],
      viewport: { width: 400, height: 600 },
    });

    await use(context);
    await context.close();
  },

  // Get extension ID from service worker
  extensionId: async ({ context }, use) => {
    // Wait for service worker to be registered
    let [background] = context.serviceWorkers();
    if (!background) {
      background = await context.waitForEvent('serviceworker');
    }

    const extensionId = background.url().split('/')[2];
    await use(extensionId);
  },
});

export { expect } from '@playwright/test';

// Helper to open extension popup
export async function openPopup(context: BrowserContext, extensionId: string) {
  const popupUrl = `chrome-extension://${extensionId}/popup/index.html`;
  const page = await context.newPage();
  await page.goto(popupUrl);
  await page.waitForLoadState('domcontentloaded');
  return page;
}

// Helper to wait for element with text
export async function waitForText(page: any, text: string, timeout = 10000) {
  await page.waitForFunction(
    (t: string) => document.body.innerText.includes(t),
    text,
    { timeout }
  );
}

// Deterministic test wallet (DO NOT use in production)
export const TEST_WALLET = {
  // 64-byte expanded private key (hex)
  privateKey: 'a'.repeat(128), // Deterministic for testing
  password: 'TestPassword123!',

  // Pre-computed addresses from this key
  expectedAddresses: {
    solana: '', // Will be computed in tests
    starknet: '',
    zcash: '',
  },
};

// Test constants
export const TIMEOUTS = {
  short: 2000,
  medium: 5000,
  long: 10000,
  transaction: 30000,
};
