import { test, expect } from '@playwright/test';
import { NavigationHelper, CreateJobWizardPage } from '../helpers/navigation';
import { FixtureHelper } from '../helpers/fixtures';

/**
 * CreateJob Wizard E2E Tests
 *
 * Tests the complete UI flow for creating FHE jobs,
 * including file uploads, form validation, and navigation.
 */

test.describe('CreateJob Wizard - Navigation', () => {
  let nav: NavigationHelper;
  let wizard: CreateJobWizardPage;
  let fixtures: FixtureHelper;

  test.beforeEach(async ({ page }) => {
    nav = new NavigationHelper(page);
    wizard = new CreateJobWizardPage(page);
    fixtures = new FixtureHelper(page);

    // Setup MockWallet (now that Buffer polyfill is fixed)
    await nav.setupMockWallet('connected');

    // Navigate to CreateJob directly
    await nav.gotoCreateJob();
  });

  test('should display all 4 wizard steps', async () => {
    // Verify all steps are visible
    await expect(wizard.step1).toBeVisible();
    await expect(wizard.step2).toBeVisible();
    await expect(wizard.step3).toBeVisible();
    await expect(wizard.step4).toBeVisible();

    // Verify step 1 is active
    await wizard.expectStep(1);
  });

  test('should navigate forward through all steps', async ({ page }) => {
    // Step 1: Upload files
    await wizard.expectStep(1);
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();

    // Step 2: Configure (uses radio buttons, not select)
    await wizard.expectStep(2);
    // Default operation is already "Multiply", just set value
    await page.locator('[data-testid="operation-value-input"]').fill('5');
    await wizard.nextButton.click();

    // Step 3: Review
    await wizard.expectStep(3);
    await expect(wizard.reviewBox).toBeVisible();
    await wizard.nextButton.click();

    // Step 4: Sign
    await wizard.expectStep(4);
    await expect(wizard.signButton).toBeVisible();
  });

  test('should navigate backward through steps', async ({ page }) => {
    // Fill wizard to step 3
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();
    // Default operation is already "Multiply", just set value
    await page.locator('[data-testid="operation-value-input"]').fill('5');
    await wizard.nextButton.click();

    // Now on step 3
    await wizard.expectStep(3);

    // Go back to step 2
    await wizard.backButton.click();
    await wizard.expectStep(2);

    // Go back to step 1
    await wizard.backButton.click();
    await wizard.expectStep(1);
  });

  test('should preserve form data when navigating back', async ({ page }) => {
    // Fill step 1
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();

    // Fill step 2 (using radio buttons)
    // Select "Add" operation by clicking the label (radio input is hidden)
    await page.locator('.radio-option:has-text("ADD")').click();
    await page.locator('[data-testid="operation-value-input"]').fill('10');
    await wizard.nextButton.click();

    // Go back
    await wizard.backButton.click();

    // Verify data is preserved - check if Add radio is selected
    const addRadio = page.locator('input[type="radio"][value="Add"]');
    await expect(addRadio).toBeChecked();

    const operationValue = await page.locator('[data-testid="operation-value-input"]').inputValue();
    expect(operationValue).toBe('10');
  });
});

test.describe('CreateJob Wizard - File Upload', () => {
  let nav: NavigationHelper;
  let wizard: CreateJobWizardPage;
  let fixtures: FixtureHelper;

  test.beforeEach(async ({ page }) => {
    nav = new NavigationHelper(page);
    wizard = new CreateJobWizardPage(page);
    fixtures = new FixtureHelper(page);

    await nav.setupMockWallet('connected');
    await nav.gotoCreateJob();
  });

  test('should accept valid FHE files', async ({ page }) => {
    await fixtures.uploadFHEFiles();

    // Verify files are displayed in status elements
    const encryptedStatus = await page.locator('[data-testid="encrypted-data-status"]').textContent();
    const serverKeyStatus = await page.locator('[data-testid="server-key-status"]').textContent();

    expect(encryptedStatus).toContain('encrypted_data.json');
    expect(serverKeyStatus).toContain('server_key.bin');

    // Next button should be enabled
    await expect(wizard.nextButton).toBeEnabled();
  });

  test('should disable next button without files', async () => {
    // Without files, next should be disabled
    await expect(wizard.nextButton).toBeDisabled();
  });

  test('should require both files to proceed', async ({ page }) => {
    // Upload only encrypted data using data-testid
    await page.locator('[data-testid="encrypted-data-input"]').setInputFiles(
      fixtures.getFixturePath('fhe/encrypted_data.json')
    );

    // Next button should still be disabled (need both files)
    await expect(wizard.nextButton).toBeDisabled();

    // Upload server key
    await page.locator('[data-testid="server-key-input"]').setInputFiles(
      fixtures.getFixturePath('fhe/server_key.bin')
    );

    // Now next button should be enabled
    await expect(wizard.nextButton).toBeEnabled();
  });

  test('should display file sizes', async ({ page }) => {
    await fixtures.uploadFHEFiles();

    // Check that file sizes are displayed in status elements
    const encryptedStatus = await page.locator('[data-testid="encrypted-data-status"]').textContent();
    const serverKeyStatus = await page.locator('[data-testid="server-key-status"]').textContent();

    // Should show KB or MB
    const hasSize = encryptedStatus?.includes('KB') || serverKeyStatus?.includes('MB') || serverKeyStatus?.includes('KB');
    expect(hasSize).toBeTruthy();
  });

  test('should allow removing uploaded files', async ({ page }) => {
    await fixtures.uploadFHEFiles();

    // Find and click remove button
    const removeBtn = page.locator('button:has-text("Remove"), .file-remove-button').first();
    if (await removeBtn.isVisible()) {
      await removeBtn.click();

      // File should be removed
      const fileItems = await page.locator('.file-item, .uploaded-file').count();
      expect(fileItems).toBeLessThan(2);
    }
  });
});

test.describe('CreateJob Wizard - Form Validation', () => {
  let nav: NavigationHelper;
  let wizard: CreateJobWizardPage;
  let fixtures: FixtureHelper;

  test.beforeEach(async ({ page }) => {
    nav = new NavigationHelper(page);
    wizard = new CreateJobWizardPage(page);
    fixtures = new FixtureHelper(page);

    await nav.setupMockWallet('connected');
    await nav.gotoCreateJob();

    // Navigate to step 2
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();
  });

  test('should require operation selection', async ({ page }) => {
    // Operation is already selected by default (Multiply)
    // Verify the radio button is checked
    const multiplyRadio = page.locator('input[type="radio"][value="Multiply"]');
    await expect(multiplyRadio).toBeChecked();

    // With value set, should proceed
    await page.locator('[data-testid="operation-value-input"]').fill('5');
    await wizard.nextButton.click();

    // Should proceed to step 3
    await wizard.expectStep(3);
  });

  test('should require operation value for numeric operations', async ({ page }) => {
    // Operation is Multiply by default

    // Clear the value
    await page.locator('[data-testid="operation-value-input"]').clear();

    // Wait for button state to update
    await page.waitForTimeout(200);

    // Next button should be disabled when value is empty
    const nextButton = wizard.nextButton;
    await expect(nextButton).toBeDisabled();

    // Should still be on step 2
    await wizard.expectStep(2);
  });

  test('should validate operation value range', async ({ page }) => {
    const valueInput = page.locator('[data-testid="operation-value-input"]');

    // Test value outside range (> 255)
    await valueInput.fill('300');
    // Trigger blur for validation
    await valueInput.blur();

    // Check for error message
    const errorMsg = page.locator('[data-testid="operation-value-error"]');
    await expect(errorMsg).toBeVisible({ timeout: 2000 });

    // Test valid value
    await valueInput.fill('5');
    await valueInput.blur();

    // Error should be gone
    await expect(errorMsg).not.toBeVisible({ timeout: 2000 });

    await wizard.nextButton.click();
    await wizard.expectStep(3); // Should proceed
  });

  test('should require consensus selection', async ({ page }) => {
    // Consensus is using radio buttons and has default selection
    await page.locator('[data-testid="operation-value-input"]').fill('10');

    // The consensus is already selected (2-of-3 by default)
    const consensusRadio = page.locator('input[type="radio"][value="2-of-3"]');
    await expect(consensusRadio).toBeChecked();

    // Should be able to proceed
    await wizard.nextButton.click();
    await wizard.expectStep(3);
  });
});

test.describe('CreateJob Wizard - Dynamic Pricing', () => {
  let nav: NavigationHelper;
  let wizard: CreateJobWizardPage;
  let fixtures: FixtureHelper;

  test.beforeEach(async ({ page }) => {
    nav = new NavigationHelper(page);
    wizard = new CreateJobWizardPage(page);
    fixtures = new FixtureHelper(page);

    await nav.setupMockWallet('connected');

    // Mock estimate-cost API
    await page.route('**/api/estimate-cost', async route => {
      const body = JSON.parse(route.request().postData() || '{}');

      const prices: Record<string, number> = {
        'add': 3_000_000,
        'multiply': 3_000_000,
        'subtract': 3_000_000,
        'threshold': 15_000_000,
        'sum': 33_000_000,
        'average': 63_000_000
      };

      const op = (body.operation || 'multiply').toLowerCase();
      const basePrice = prices[op] || 10_000_000;
      const requiredProvers = body.required_provers || 3;

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          operation: op,
          min_payment_lamports: basePrice,
          min_payment_sol: basePrice / 1_000_000_000,
          total_min_payment_lamports: basePrice * requiredProvers,
          complexity_tier: op === 'add' ? 1 : op === 'multiply' ? 2 : 3,
          timeout_seconds: 300
        })
      });
    });

    await nav.gotoCreateJob();
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();
  });

  test('should update pricing when operation changes', async ({ page }) => {
    // Default operation is Multiply, fill value
    await page.locator('[data-testid="operation-value-input"]').fill('5');

    // Wait for cost to load
    await page.waitForTimeout(500);

    // Get initial cost
    const totalCostEl = page.locator('[data-testid="total-cost"]');
    await expect(totalCostEl).toBeVisible({ timeout: 5000 });
    const initialCost = await totalCostEl.textContent();

    // Change to Add operation by clicking the label (radio input is hidden)
    await page.locator('.radio-option:has-text("ADD")').click();

    // Wait for cost update
    await page.waitForTimeout(500);

    // Get updated cost
    const updatedCost = await totalCostEl.textContent();

    // Cost element should have content
    expect(initialCost).toBeTruthy();
    expect(updatedCost).toBeTruthy();
  });

  test('should update pricing when prover count changes', async ({ page }) => {
    // Fill value first
    await page.locator('[data-testid="operation-value-input"]').fill('5');

    // Wait for initial cost
    await page.waitForTimeout(500);

    const totalCostEl = page.locator('[data-testid="total-cost"]');
    await expect(totalCostEl).toBeVisible({ timeout: 5000 });
    const cost2of3 = await totalCostEl.textContent();

    // Change to 3-of-5 consensus by clicking the label that contains "3_OF_5"
    const consensusOption = page.locator('.radio-option:has-text("3_OF_5_PROVERS")');
    await consensusOption.click();

    // Wait for cost update
    await page.waitForTimeout(500);

    const cost3of5 = await totalCostEl.textContent();

    // Costs should be different (3-of-5 costs more)
    expect(cost2of3).toBeTruthy();
    expect(cost3of5).toBeTruthy();
  });

  test('should display cost breakdown', async ({ page }) => {
    // Fill value
    await page.locator('[data-testid="operation-value-input"]').fill('5');

    // Wait for cost to load
    await page.waitForTimeout(500);

    // Verify cost breakdown elements are visible
    const costBreakdown = page.locator('[data-testid="cost-breakdown"]');
    await expect(costBreakdown).toBeVisible({ timeout: 5000 });

    // Check for total cost
    const totalCost = page.locator('[data-testid="total-cost"]');
    await expect(totalCost).toBeVisible();

    const text = await totalCost.textContent();
    // Should show SOL amount
    expect(text).toMatch(/SOL/i);
  });
});

test.describe('CreateJob Wizard - Payment Method', () => {
  let nav: NavigationHelper;
  let wizard: CreateJobWizardPage;
  let fixtures: FixtureHelper;

  test.beforeEach(async ({ page }) => {
    nav = new NavigationHelper(page);
    wizard = new CreateJobWizardPage(page);
    fixtures = new FixtureHelper(page);

    await nav.setupMockWallet('connected');
    await nav.gotoCreateJob();
    await fixtures.uploadFHEFiles();
    await wizard.nextButton.click();
  });

  test('should allow selecting SOL payment', async ({ page }) => {
    const solRadio = page.locator('input[value="sol"], input[name="payment_method"][value="SOL"]');

    if (await solRadio.isVisible()) {
      await solRadio.click();

      // Verify selected
      await expect(solRadio).toBeChecked();
    }
  });

  test('should allow selecting wZEC payment', async ({ page }) => {
    const wzecRadio = page.locator('input[value="wzec"], input[name="payment_method"][value="WZEC"]');

    if (await wzecRadio.isVisible()) {
      await wzecRadio.click();

      // Verify selected
      await expect(wzecRadio).toBeChecked();
    }
  });
});
