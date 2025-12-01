import { test, expect, openPopup, TEST_WALLET, TIMEOUTS } from './fixtures';

/**
 * Testnet Integration Tests
 *
 * These tests verify actual blockchain interactions on local testnet.
 *
 * Run locally:
 *   cd docker && docker-compose up -d
 *   npm test -- testnet-integration.spec.ts
 *
 * Run in Docker:
 *   docker-compose -f docker/docker-compose.test.yml up --build
 *
 * Environment variables (for Docker):
 *   SOLANA_RPC_URL, STARKNET_RPC_URL, ZCASH_RPC_URL
 */

// RPC URLs - use env vars in Docker, localhost for local dev
const RPC_URLS = {
  solana: process.env.SOLANA_RPC_URL || 'http://localhost:8899',
  starknet: process.env.STARKNET_RPC_URL || 'http://localhost:5050',
  zcash: process.env.ZCASH_RPC_URL || 'http://localhost:18232',
};

// Check if testnet services are available
async function isTestnetAvailable(): Promise<{
  solana: boolean;
  starknet: boolean;
  zcash: boolean;
}> {
  const results = { solana: false, starknet: false, zcash: false };

  // Check Solana
  try {
    const res = await fetch(`${RPC_URLS.solana}/health`);
    results.solana = res.ok;
  } catch {}

  // Check Starknet (Katana)
  try {
    const res = await fetch(RPC_URLS.starknet);
    results.starknet = res.status !== 0;
  } catch {}

  // Check Zcash
  try {
    const res = await fetch(RPC_URLS.zcash, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Basic ' + Buffer.from('zyberlink:testpass123').toString('base64'),
      },
      body: JSON.stringify({ jsonrpc: '1.0', method: 'getblockchaininfo', params: [] }),
    });
    results.zcash = res.ok;
  } catch {}

  return results;
}

test.describe('Testnet Integration', () => {
  let testnetStatus: { solana: boolean; starknet: boolean; zcash: boolean };

  test.beforeAll(async () => {
    testnetStatus = await isTestnetAvailable();
    console.log('Testnet status:', testnetStatus);

    if (!testnetStatus.solana && !testnetStatus.starknet && !testnetStatus.zcash) {
      console.log('No testnet services available, skipping integration tests');
      test.skip();
    }
  });

  test.describe('Solana Testnet', () => {
    test.beforeEach(async () => {
      if (!testnetStatus?.solana) {
        test.skip();
      }
    });

    test('should connect to local Solana validator', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup wallet
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Go to settings
      await page.click('button[title="Settings"]');

      // Check RPC settings
      await expect(page.locator('text=localhost:8899')).toBeVisible({ timeout: TIMEOUTS.medium });
    });

    test('should fetch balance from Solana testnet', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Wait for balance to load (should show 0.0000 if no funds)
      await page.waitForTimeout(2000);

      // SOL balance should be displayed
      const solBalance = page.locator('.asset-item:has-text("SOL") .asset-amount');
      await expect(solBalance).toBeVisible();

      // Should be a valid number (even if 0)
      const balanceText = await solBalance.textContent();
      expect(parseFloat(balanceText || '0')).toBeGreaterThanOrEqual(0);
    });

    test('should request airdrop from Solana faucet', async ({ context, extensionId }) => {
      // This test actually airdrops SOL on testnet
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Get Solana address from UI
      await page.click('button:has-text("RECEIVE")');
      await expect(page.locator('text=SOLANA ADDRESS')).toBeVisible();

      const addressElement = page.locator('.receive-address');
      const address = await addressElement.textContent();

      if (address) {
        // Request airdrop via RPC
        const airdropResponse = await fetch(RPC_URLS.solana, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method: 'requestAirdrop',
            params: [address.trim(), 1000000000], // 1 SOL
          }),
        });

        const result = await airdropResponse.json();
        expect(result.result).toBeTruthy(); // Should return signature
      }
    });
  });

  test.describe('Starknet Testnet (Katana)', () => {
    test.beforeEach(async () => {
      if (!testnetStatus?.starknet) {
        test.skip();
      }
    });

    test('should connect to local Katana', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Go to settings
      await page.click('button[title="Settings"]');

      // Check RPC
      await expect(page.locator('text=localhost:5050')).toBeVisible({ timeout: TIMEOUTS.medium });
    });
  });

  test.describe('Zcash Regtest', () => {
    test.beforeEach(async () => {
      if (!testnetStatus?.zcash) {
        test.skip();
      }
    });

    test('should connect to local Zcash regtest', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Go to settings
      await page.click('button[title="Settings"]');

      // Check RPC
      await expect(page.locator('text=localhost:18232')).toBeVisible({ timeout: TIMEOUTS.medium });
    });

    test('should fetch Zcash blockchain info', async () => {
      // Direct RPC test
      const response = await fetch(RPC_URLS.zcash, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Basic ' + Buffer.from('zyberlink:testpass123').toString('base64'),
        },
        body: JSON.stringify({
          jsonrpc: '1.0',
          method: 'getblockchaininfo',
          params: [],
        }),
      });

      const data = await response.json();
      expect(data.result).toBeTruthy();
      expect(data.result.chain).toBe('regtest');
    });

    test('should generate Zcash blocks for testing', async () => {
      // Mine some blocks
      const response = await fetch(RPC_URLS.zcash, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Basic ' + Buffer.from('zyberlink:testpass123').toString('base64'),
        },
        body: JSON.stringify({
          jsonrpc: '1.0',
          method: 'generate',
          params: [5], // Generate 5 blocks
        }),
      });

      const data = await response.json();
      expect(data.result).toHaveLength(5);
    });
  });

  test.describe('Cross-chain Flow', () => {
    test('should display all chain addresses', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Check all chains have addresses
      await page.click('button:has-text("RECEIVE")');

      // Solana
      await expect(page.locator('text=SOLANA ADDRESS')).toBeVisible();
      let address = await page.locator('.receive-address').textContent();
      expect(address?.length).toBeGreaterThan(30);

      // Starknet
      await page.click('button.receive-tab:has-text("STRK")');
      await expect(page.locator('text=STARKNET ADDRESS')).toBeVisible();
      address = await page.locator('.receive-address').textContent();
      expect(address?.startsWith('0x')).toBe(true);

      // Zcash
      await page.click('button.receive-tab:has-text("ZEC")');
      await expect(page.locator('text=ZCASH ADDRESS')).toBeVisible();
      address = await page.locator('.receive-address').textContent();
      expect(address?.startsWith('t')).toBe(true); // t-address for transparent
    });
  });
});
