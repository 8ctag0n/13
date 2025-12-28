/**
 * Token Account Manager Tests
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  checkTokenAccount,
  getTokenAddress,
  formatTokenAccountInfo,
  WZEC_MINT,
  hasWzecAccount
} from '../src/lib/utils/tokenAccountManager.js';
import { PublicKey } from '@solana/web3.js';

describe('Token Account Manager', () => {
  const mockOwner = new PublicKey('7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR');
  const mockMint = WZEC_MINT;

  describe('checkTokenAccount', () => {
    it('returns exists=true when account exists', async () => {
      const mockConnection = {
        getAccountInfo: vi.fn().mockResolvedValue({
          data: Buffer.from([]),
          executable: false,
          lamports: 1000000,
          owner: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA')
        })
      };

      const result = await checkTokenAccount(mockConnection, mockOwner, mockMint);

      expect(result.exists).toBe(true);
      expect(result.address).toBeTruthy();
      expect(mockConnection.getAccountInfo).toHaveBeenCalledOnce();
    });

    it('returns exists=false when account does not exist', async () => {
      const mockConnection = {
        getAccountInfo: vi.fn().mockResolvedValue(null)
      };

      const result = await checkTokenAccount(mockConnection, mockOwner, mockMint);

      expect(result.exists).toBe(false);
      expect(result.address).toBeNull();
    });

    it('handles errors gracefully', async () => {
      const mockConnection = {
        getAccountInfo: vi.fn().mockRejectedValue(new Error('Network error'))
      };

      const result = await checkTokenAccount(mockConnection, mockOwner, mockMint);

      expect(result.exists).toBe(false);
      expect(result.error).toBe('Network error');
    });
  });

  describe('getTokenAddress', () => {
    it('returns a valid PublicKey', async () => {
      const address = await getTokenAddress(mockOwner, mockMint);

      expect(address).toBeInstanceOf(PublicKey);
      expect(address.toString()).toMatch(/^[1-9A-HJ-NP-Za-km-z]{32,44}$/);
    });

    it('returns same address for same inputs', async () => {
      const address1 = await getTokenAddress(mockOwner, mockMint);
      const address2 = await getTokenAddress(mockOwner, mockMint);

      expect(address1.toString()).toBe(address2.toString());
    });
  });

  describe('formatTokenAccountInfo', () => {
    it('formats existing account info', () => {
      const accountInfo = {
        exists: true,
        address: new PublicKey('7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR')
      };

      const formatted = formatTokenAccountInfo(accountInfo);

      expect(formatted.status).toBe('found');
      expect(formatted.requiresCreation).toBe(false);
      expect(formatted.address).toBeTruthy();
    });

    it('formats non-existing account info', () => {
      const accountInfo = {
        exists: false
      };

      const formatted = formatTokenAccountInfo(accountInfo);

      expect(formatted.status).toBe('not_found');
      expect(formatted.requiresCreation).toBe(true);
      expect(formatted.message).toContain('does not exist');
    });
  });

  describe('hasWzecAccount', () => {
    it('returns true when wZEC account exists', async () => {
      const mockConnection = {
        getAccountInfo: vi.fn().mockResolvedValue({
          data: Buffer.from([]),
          executable: false,
          lamports: 1000000,
          owner: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA')
        })
      };

      const result = await hasWzecAccount(mockConnection, mockOwner);

      expect(result).toBe(true);
    });

    it('returns false when wZEC account does not exist', async () => {
      const mockConnection = {
        getAccountInfo: vi.fn().mockResolvedValue(null)
      };

      const result = await hasWzecAccount(mockConnection, mockOwner);

      expect(result).toBe(false);
    });
  });

  describe('Constants', () => {
    it('exports WZEC_MINT as PublicKey', () => {
      expect(WZEC_MINT).toBeInstanceOf(PublicKey);
      expect(WZEC_MINT.toString()).toBe('sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ');
    });
  });
});

describe('Token Account Manager - Mock API Integration', () => {
  it('works with mock API responses', async () => {
    const mockApiResponse = {
      exists: true,
      account_address: '7XaJXBmNqJKrZ8Lr5MqPx4fV3rTvKqF1N2LhKvwDxMYn'
    };

    // Simulate API call
    const formatted = formatTokenAccountInfo({
      exists: mockApiResponse.exists,
      address: mockApiResponse.exists
        ? new PublicKey(mockApiResponse.account_address)
        : null
    });

    expect(formatted.status).toBe('found');
    expect(formatted.requiresCreation).toBe(false);
  });
});
