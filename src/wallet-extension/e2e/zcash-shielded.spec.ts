import { test, expect, openPopup, TEST_WALLET, TIMEOUTS } from './fixtures';

/**
 * Zcash Shielded Transaction Tests
 *
 * These tests require the local testnet to be running:
 * cd docker && docker-compose up -d
 *
 * Run with: npm run test -- zcash-shielded.spec.ts
 */

// RPC URL - use env var in Docker, localhost for local dev
const ZCASH_RPC_URL = process.env.ZCASH_RPC_URL || 'http://localhost:18232';

test.describe('Zcash Shielded Transactions', () => {
  // Skip if testnet not available
  test.beforeAll(async () => {
    try {
      const response = await fetch(ZCASH_RPC_URL, {
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

      if (!response.ok) {
        test.skip();
      }
    } catch {
      console.log('Zcash testnet not available, skipping shielded tests');
      test.skip();
    }
  });

  test.describe('Zcash View with Shielded Support', () => {
    test('should show transparent and shielded addresses when Zcash selected', async ({
      context,
      extensionId,
    }) => {
      const page = await openPopup(context, extensionId);

      // Setup wallet
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Select Zcash tab
      await page.click('button.chain-tab:has-text("Z")');

      // Should show expanded Zcash view with t and z addresses
      await expect(page.locator('text=transparent')).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(page.locator('text=shielded')).toBeVisible();

      // Should show shield button
      await expect(page.locator('button:has-text("SHIELD")')).toBeVisible();

      // Should show mine button (for testnet)
      await expect(page.locator('button:has-text("MINE")')).toBeVisible();
    });

    test('should display both balances separately', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Select Zcash
      await page.click('button.chain-tab:has-text("Z")');

      // Both asset items should show balances
      const assetItems = page.locator('.asset-item');
      await expect(assetItems).toHaveCount(2); // t-address and z-address
    });
  });

  test.describe('Zcash Receive with Address Toggle', () => {
    test('should toggle between transparent and shielded address in receive', async ({
      context,
      extensionId,
    }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Go to receive
      await page.click('button:has-text("RECEIVE")');

      // Select Zcash
      await page.click('button.receive-tab:has-text("ZEC")');

      // Should show toggle buttons
      await expect(page.locator('button:has-text("Transparent")')).toBeVisible();
      await expect(page.locator('button:has-text("Shielded")')).toBeVisible();

      // Default should be transparent
      await expect(page.locator('text=TRANSPARENT ADDRESS')).toBeVisible();

      // Click shielded
      await page.click('button:has-text("Shielded")');

      // Should show shielded address
      await expect(page.locator('text=SHIELDED ADDRESS')).toBeVisible();
    });

    test('should show privacy warning for each address type', async ({
      context,
      extensionId,
    }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      await page.click('button:has-text("RECEIVE")');
      await page.click('button.receive-tab:has-text("ZEC")');

      // Transparent warning
      await expect(page.locator('text=transactions are public')).toBeVisible();

      // Switch to shielded
      await page.click('button:has-text("Shielded")');

      // Shielded warning
      await expect(page.locator('text=transactions are private')).toBeVisible();
    });
  });

  test.describe('Shield Funds Flow (requires testnet)', () => {
    test('should initiate shield operation', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Select Zcash
      await page.click('button.chain-tab:has-text("Z")');

      // Shield button should be visible (but disabled if no balance)
      const shieldBtn = page.locator('button:has-text("SHIELD")');
      await expect(shieldBtn).toBeVisible();

      // Note: Actually clicking would require funds, which we'd get from mining
    });

    test('should mine blocks on testnet', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Select Zcash
      await page.click('button.chain-tab:has-text("Z")');

      // Click mine button
      const mineBtn = page.locator('button:has-text("MINE")');
      await expect(mineBtn).toBeVisible();

      // Click and wait (this actually mines blocks on regtest)
      await mineBtn.click();

      // Should complete without error (balance might update)
      await page.waitForTimeout(2000);

      // Still on Zcash view
      await expect(page.locator('text=transparent')).toBeVisible();
    });
  });
});
