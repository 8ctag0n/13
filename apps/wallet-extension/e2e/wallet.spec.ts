import { test, expect, openPopup, waitForText, TEST_WALLET, TIMEOUTS } from './fixtures';

test.describe('ZyberLink Wallet Extension', () => {
  test.describe('Onboarding Flow', () => {
    test('should show welcome screen on first open', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Should show ASCII art logo and welcome text
      await expect(page.locator('text=ZYBERLINK')).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(page.locator('text=MULTI-CHAIN')).toBeVisible();

      // Should have create and import options
      await expect(page.locator('text=CREATE')).toBeVisible();
      await expect(page.locator('text=IMPORT')).toBeVisible();
    });

    test('should create new wallet with password', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Click create wallet
      await page.click('button:has-text("CREATE")');

      // Fill password form
      await page.fill('input[type="password"]', TEST_WALLET.password);

      // Find confirm password field (second password input)
      const passwordInputs = page.locator('input[type="password"]');
      await passwordInputs.nth(1).fill(TEST_WALLET.password);

      // Submit
      await page.click('button:has-text("CREATE")');

      // Should show generated private key
      await expect(page.locator('text=PRIVATE KEY')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Should have continue button
      await expect(page.locator('button:has-text("CONTINUE")')).toBeVisible();
    });

    test('should import wallet with private key', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Click import wallet
      await page.click('button:has-text("IMPORT")');

      // Fill private key
      await page.fill('textarea, input[placeholder*="private"]', TEST_WALLET.privateKey);

      // Fill password
      const passwordInputs = page.locator('input[type="password"]');
      await passwordInputs.first().fill(TEST_WALLET.password);
      await passwordInputs.nth(1).fill(TEST_WALLET.password);

      // Submit
      await page.click('button:has-text("IMPORT")');

      // Should navigate to home with addresses
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });
    });
  });

  test.describe('Home Screen', () => {
    test.beforeEach(async ({ context, extensionId }) => {
      // Create wallet first
      const page = await openPopup(context, extensionId);

      // Quick setup: create wallet
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');

      // Wait for home
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });
    });

    test('should display all three chains', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Unlock if needed
      if (await page.locator('input[type="password"]').isVisible()) {
        await page.fill('input[type="password"]', TEST_WALLET.password);
        await page.click('button:has-text("UNLOCK")');
      }

      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Check chain tabs
      await expect(page.locator('text=ALL')).toBeVisible();
      await expect(page.locator('button:has-text("◉")')).toBeVisible(); // Solana
      await expect(page.locator('button:has-text("▲")')).toBeVisible(); // Starknet
      await expect(page.locator('button:has-text("Z")')).toBeVisible(); // Zcash
    });

    test('should show assets for each chain', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Unlock
      if (await page.locator('input[type="password"]').isVisible()) {
        await page.fill('input[type="password"]', TEST_WALLET.password);
        await page.click('button:has-text("UNLOCK")');
      }

      await expect(page.locator('text=ASSETS')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Check assets are displayed
      await expect(page.locator('text=SOL')).toBeVisible();
      await expect(page.locator('text=STRK')).toBeVisible();
      await expect(page.locator('text=ZEC')).toBeVisible();
    });

    test('should switch between chains', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Unlock
      if (await page.locator('input[type="password"]').isVisible()) {
        await page.fill('input[type="password"]', TEST_WALLET.password);
        await page.click('button:has-text("UNLOCK")');
      }

      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Click Zcash tab
      await page.click('button.chain-tab:has-text("Z")');

      // Should show only Zcash
      await expect(page.locator('.asset-item:has-text("ZEC")')).toBeVisible();
    });
  });

  test.describe('Send Flow', () => {
    test('should open send screen', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Setup wallet
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');

      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Click send button
      await page.click('button:has-text("SEND")');

      // Should show send form
      await expect(page.locator('text=RECIPIENT')).toBeVisible();
      await expect(page.locator('text=AMOUNT')).toBeVisible();
    });

    test('should validate recipient address', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      await page.click('button:has-text("SEND")');
      await expect(page.locator('text=RECIPIENT')).toBeVisible();

      // Enter invalid address (too short)
      await page.fill('input#recipient', 'invalid');

      // Review button should be disabled
      const reviewBtn = page.locator('button:has-text("REVIEW")');
      await expect(reviewBtn).toBeDisabled();
    });
  });

  test.describe('Receive Flow', () => {
    test('should show QR code for selected chain', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Click receive
      await page.click('button:has-text("RECEIVE")');

      // Should show receive view with QR
      await expect(page.locator('text=RECEIVE')).toBeVisible();
      await expect(page.locator('canvas, svg')).toBeVisible(); // QR code

      // Should show address
      await expect(page.locator('text=ADDRESS')).toBeVisible();
    });

    test('should switch between chains in receive view', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      await page.click('button:has-text("RECEIVE")');
      await expect(page.locator('text=RECEIVE')).toBeVisible();

      // Click Zcash tab
      await page.click('button.receive-tab:has-text("ZEC")');

      // Should show Zcash address label
      await expect(page.locator('text=ZCASH ADDRESS')).toBeVisible();
    });
  });

  test.describe('Settings', () => {
    test('should open settings', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Click settings button
      await page.click('button[title="Settings"]');

      // Should show settings view
      await expect(page.locator('text=SETTINGS')).toBeVisible();
      await expect(page.locator('text=RPC')).toBeVisible();
    });
  });

  test.describe('Lock/Unlock', () => {
    test('should lock wallet and require password to unlock', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Lock wallet
      await page.click('button[title="Lock Wallet"]');

      // Should show unlock screen
      await expect(page.locator('input[type="password"]')).toBeVisible();
      await expect(page.locator('button:has-text("UNLOCK")')).toBeVisible();

      // Enter password and unlock
      await page.fill('input[type="password"]', TEST_WALLET.password);
      await page.click('button:has-text("UNLOCK")');

      // Should be back to home
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.medium });
    });

    test('should reject wrong password', async ({ context, extensionId }) => {
      const page = await openPopup(context, extensionId);

      // Quick setup
      await page.click('button:has-text("CREATE")');
      await page.locator('input[type="password"]').first().fill(TEST_WALLET.password);
      await page.locator('input[type="password"]').nth(1).fill(TEST_WALLET.password);
      await page.click('button:has-text("CREATE")');
      await page.click('button:has-text("CONTINUE")');
      await expect(page.locator('text=TOTAL BALANCE')).toBeVisible({ timeout: TIMEOUTS.long });

      // Lock
      await page.click('button[title="Lock Wallet"]');

      // Try wrong password
      await page.fill('input[type="password"]', 'WrongPassword');
      await page.click('button:has-text("UNLOCK")');

      // Should show error
      await expect(page.locator('text=incorrect', { exact: false })).toBeVisible({ timeout: TIMEOUTS.short });
    });
  });
});
