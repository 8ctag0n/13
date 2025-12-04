import { Page, expect } from '@playwright/test';
import { generateMockWalletScript, MockWalletPresets } from '../fixtures/wallets/mockWallet';

/**
 * Navigation Helpers for E2E Tests
 *
 * Reusable functions for common navigation patterns
 */

export class NavigationHelper {
  constructor(private page: Page) {}

  /**
   * Setup: Inject mock wallet before each test
   */
  async setupMockWallet(preset: keyof typeof MockWalletPresets = 'connected') {
    const config = MockWalletPresets[preset];
    const script = generateMockWalletScript(config);

    await this.page.addInitScript(script);
  }

  /**
   * Navigate to CreateJob page via hash URL
   * If firstNavigation is true, uses goto. Otherwise changes hash only.
   */
  async gotoCreateJob(firstNavigation: boolean = true) {
    if (firstNavigation) {
      // First navigation - use goto
      await this.page.goto('/#create-job');
      await this.page.waitForLoadState('domcontentloaded');
    } else {
      // Already on a page - just change the hash
      await this.page.evaluate(() => {
        window.location.hash = 'create-job';
      });
      await this.page.waitForTimeout(500); // Wait for route change
    }

    // Wait for CreateJob page to load
    await this.page.waitForSelector('.create-job', { timeout: 10000 });
  }

  /**
   * Navigate to Dashboard via hash URL
   */
  async gotoDashboard() {
    // Navigate directly via hash URL
    await this.page.goto('/#dashboard');
    await this.page.waitForLoadState('networkidle');

    // Wait for dashboard to load
    await this.page.waitForSelector('.dashboard, .dashboard-header', { timeout: 10000 });
  }

  /**
   * Connect mock wallet
   */
  async connectWallet() {
    // Try multiple selectors for connect button using data-testid first
    const connectSelectors = [
      '[data-testid="wallet-connect-button"]',
      '[data-testid="connect-wallet"]',
      'button:has-text("Connect")',
      '.wallet-button:has-text("Connect")',
      '[data-connect="wallet"]'
    ];

    let clicked = false;
    for (const selector of connectSelectors) {
      const button = this.page.locator(selector).first();
      if (await button.isVisible({ timeout: 2000 }).catch(() => false)) {
        await button.click();
        clicked = true;
        break;
      }
    }

    if (!clicked) {
      // Maybe wallet is already connected, check for connected state
      const connectedSelectors = [
        '[data-testid="wallet-connected-button"]',
        '[data-testid="wallet-connected"]',
        '[data-testid="wallet-address"]',
        '.wallet-button.connected'
      ];

      for (const selector of connectedSelectors) {
        if (await this.page.locator(selector).isVisible({ timeout: 1000 }).catch(() => false)) {
          return; // Already connected
        }
      }

      throw new Error('No connect button found and wallet not already connected');
    }

    // Wait for wallet modal (if exists)
    const modal = this.page.locator('.wallet-modal, .wallet-adapter-modal, [role="dialog"]').first();
    if (await modal.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Select Phantom or first available wallet
      const phantomButton = modal.locator('button:has-text("Phantom")').first();
      if (await phantomButton.isVisible({ timeout: 1000 }).catch(() => false)) {
        await phantomButton.click();
      } else {
        // Click first wallet option
        await modal.locator('button').first().click();
      }
    }

    // Wait for connection to complete using data-testid
    await this.page.waitForSelector(
      '[data-testid="wallet-connected-button"], [data-testid="wallet-address"], .wallet-button.connected',
      { timeout: 5000 }
    );
  }

  /**
   * Wait for API response
   */
  async waitForAPIResponse(endpoint: string, timeout: number = 5000) {
    return await this.page.waitForResponse(
      response => response.url().includes(endpoint) && response.status() === 200,
      { timeout }
    );
  }

  /**
   * Wait for toast message
   */
  async waitForToast(message: string, type: 'success' | 'error' = 'success') {
    await this.page.waitForSelector(
      `.toast.${type}:has-text("${message}"), .toast:has-text("${message}")`,
      { timeout: 10000 }
    );
  }

  /**
   * Dismiss all toasts
   */
  async dismissToasts() {
    const toasts = this.page.locator('.toast');
    const count = await toasts.count();

    for (let i = 0; i < count; i++) {
      const closeBtn = toasts.nth(i).locator('[data-testid="toast-close"], .close-button');
      if (await closeBtn.isVisible()) {
        await closeBtn.click();
      }
    }
  }

  /**
   * Check if element is visible
   */
  async isVisible(selector: string): Promise<boolean> {
    try {
      return await this.page.locator(selector).isVisible({ timeout: 1000 });
    } catch {
      return false;
    }
  }

  /**
   * Take screenshot with timestamp
   */
  async screenshot(name: string) {
    const timestamp = new Date().toISOString().replace(/:/g, '-');
    await this.page.screenshot({
      path: `test-results/screenshots/${name}-${timestamp}.png`,
      fullPage: true
    });
  }
}

/**
 * Page Object: CreateJob Wizard
 */
export class CreateJobWizardPage {
  constructor(private page: Page) {}

  // Step indicators
  get step1() {
    return this.page.locator('.stepper-step').nth(0);
  }

  get step2() {
    return this.page.locator('.stepper-step').nth(1);
  }

  get step3() {
    return this.page.locator('.stepper-step').nth(2);
  }

  get step4() {
    return this.page.locator('.stepper-step').nth(3);
  }

  get activeStep() {
    return this.page.locator('.stepper-step.active');
  }

  // Buttons - using data-testid first
  get nextButton() {
    return this.page.locator('[data-testid="wizard-next"], button:has-text("NEXT"), button:has-text("Next")');
  }

  get backButton() {
    return this.page.locator('[data-testid="wizard-back"], button:has-text("BACK"), button:has-text("Back")');
  }

  get submitButton() {
    return this.page.locator('[data-testid="wizard-submit"], button:has-text("CREATE_JOB"), button:has-text("SUBMIT")');
  }

  // Step 1: File upload
  get fileInput() {
    return this.page.locator('input[type="file"]');
  }

  get dropZone() {
    return this.page.locator('.drop-zone, .file-upload-zone');
  }

  // Step 2: Configure
  get operationSelect() {
    return this.page.locator('select[name="operation"], .operation-selector');
  }

  get operationValueInput() {
    return this.page.locator('input[name="operation_value"], input[type="number"]');
  }

  get consensusSelect() {
    return this.page.locator('select[name="consensus"], .consensus-selector');
  }

  // Step 3: Review
  get reviewBox() {
    return this.page.locator('.review-box, .job-summary').first();
  }

  get totalCost() {
    return this.page.locator('.cost-line.total, .total-cost');
  }

  // Step 4: Sign
  get signButton() {
    return this.page.locator('[data-testid="wizard-submit"], button:has-text("CREATE_JOB"), button:has-text("SIGN")');
  }

  /**
   * Helper: Fill entire wizard with defaults
   */
  async fillWizardDefaults() {
    // Step 1: Upload files
    const fixturePath = require('path').join(process.cwd(), 'e2e-tests', 'fixtures', 'fhe');
    await this.page.setInputFiles('input[type="file"]', [
      `${fixturePath}/encrypted_data.json`,
      `${fixturePath}/server_key.bin`
    ]);
    await this.nextButton.click();

    // Step 2: Configure
    await this.page.selectOption('select[name="operation"]', 'Multiply');
    await this.operationValueInput.fill('5');
    await this.nextButton.click();

    // Step 3: Review
    await this.nextButton.click();

    // Now on Step 4: Sign
  }

  /**
   * Helper: Verify current step
   */
  async expectStep(stepNumber: number) {
    // Check step indicator text which shows "STEP X OF 4"
    const stepIndicator = this.page.locator('.step-indicator');
    await expect(stepIndicator).toBeVisible({ timeout: 5000 });
    const text = await stepIndicator.textContent();
    expect(text).toContain(`STEP ${stepNumber}`);
  }
}
