/**
 * Mock API responses for development
 * Used until Agent A implements real backend
 */

export const WZEC_MINT = 'sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ';

export const mockCreateJobResponse = {
  sol: {
    job_id: 1,
    transaction: 'AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQABAgMEBQ==',
    status: 'pending_signature',
    requires_token_account: false
  },
  wzec_with_account: {
    job_id: 2,
    transaction: 'AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQABAgMEBQ==',
    status: 'pending_signature',
    requires_token_account: false,
    token_mint: WZEC_MINT
  },
  wzec_needs_account: {
    job_id: 3,
    transaction: 'AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQABAgMEBQ==',
    status: 'requires_token_account',
    requires_token_account: true,
    token_mint: WZEC_MINT
  }
};

export const mockCheckTokenAccountResponse = {
  exists: {
    exists: true,
    account_address: '7XaJXBmNqJKrZ8Lr5MqPx4fV3rTvKqF1N2LhKvwDxMYn'
  },
  not_exists: {
    exists: false
  }
};

/**
 * Mock API client
 */
export class MockApiClient {
  constructor(useMockData = true) {
    this.useMockData = useMockData;
  }

  async createJob(jobData) {
    // Simulate network delay
    await this.delay(500);

    const paymentMethod = jobData.payment_method || 'sol';

    if (paymentMethod === 'sol') {
      return mockCreateJobResponse.sol;
    }

    // Randomly decide if token account exists (for testing)
    const hasTokenAccount = Math.random() > 0.5;
    return hasTokenAccount
      ? mockCreateJobResponse.wzec_with_account
      : mockCreateJobResponse.wzec_needs_account;
  }

  async checkTokenAccount(pubkey, mint) {
    await this.delay(300);

    // Randomly simulate token account existence
    const exists = Math.random() > 0.3;
    return exists
      ? mockCheckTokenAccountResponse.exists
      : mockCheckTokenAccountResponse.not_exists;
  }

  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Default mock client instance
export const mockApi = new MockApiClient();
