import { Page } from '@playwright/test';
import path from 'path';

/**
 * Fixture Helpers for E2E Tests
 *
 * Utilities for loading and managing test fixtures
 */

export class FixtureHelper {
  constructor(private page: Page) {}

  /**
   * Get absolute path to fixture file
   */
  getFixturePath(relativePath: string): string {
    return path.join(process.cwd(), 'e2e-tests', 'fixtures', relativePath);
  }

  /**
   * Upload FHE files using data-testid selectors
   * Uploads encrypted_data.json and server_key.bin to their respective inputs
   */
  async uploadFHEFiles() {
    // Use specific data-testid selectors for reliability
    const encryptedInput = this.page.locator('[data-testid="encrypted-data-input"]');
    const serverKeyInput = this.page.locator('[data-testid="server-key-input"]');

    // Upload encrypted data
    await encryptedInput.setInputFiles(
      this.getFixturePath('fhe/encrypted_data.json')
    );

    // Wait briefly for UI update
    await this.page.waitForTimeout(300);

    // Upload server key
    await serverKeyInput.setInputFiles(
      this.getFixturePath('fhe/server_key.bin')
    );

    // Wait for upload status indicators
    await this.page.waitForTimeout(300);
  }

  /**
   * Upload only encrypted data (for error testing)
   */
  async uploadEncryptedDataOnly(selector: string = 'input[type="file"]') {
    await this.page.setInputFiles(
      selector,
      this.getFixturePath('fhe/encrypted_data.json')
    );
  }

  /**
   * Upload invalid file (for error testing)
   */
  async uploadInvalidFile(selector: string = 'input[type="file"]') {
    await this.page.setInputFiles(
      selector,
      this.getFixturePath('fhe/invalid_data.json')
    );
  }

  /**
   * Mock API response with fixture data
   */
  async mockAPIWithFixture(endpoint: string, fixturePath: string) {
    await this.page.route(`**/${endpoint}`, async route => {
      const fs = require('fs');
      const fixtureData = fs.readFileSync(
        this.getFixturePath(fixturePath),
        'utf-8'
      );

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: fixtureData
      });
    });
  }
}

/**
 * API Mock Responses
 */
export const APIMocks = {
  validateAndBuild: {
    success: {
      job_id: 12345,
      transaction: 'base64_encoded_transaction_data',
      status: 'pending_signature',
      escrow_account: 'EscrowAccount123...'
    },
    error: {
      error: 'Validation failed',
      details: 'Insufficient balance'
    }
  },

  estimateCost: {
    tier1: {
      operation: 'Multiply',
      min_payment_lamports: 3_000_000,
      complexity_tier: 1,
      timeout_seconds: 180
    },
    tier2: {
      operation: 'Threshold',
      min_payment_lamports: 15_000_000,
      complexity_tier: 2,
      timeout_seconds: 300
    },
    tier3: {
      operation: 'Sum',
      min_payment_lamports: 33_000_000,
      complexity_tier: 3,
      timeout_seconds: 420
    },
    tier4: {
      operation: 'Average',
      min_payment_lamports: 63_000_000,
      complexity_tier: 4,
      timeout_seconds: 600
    }
  }
};
