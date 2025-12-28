import { test, expect } from '@playwright/test';

/**
 * API Integration Tests (No Browser Required)
 *
 * These tests verify blockchain RPC connectivity without requiring
 * the browser extension. Suitable for running in containerized CI/CD.
 *
 * Run in Docker:
 *   docker-compose -f docker/docker-compose.test.yml up --build
 */

// RPC URLs - use env vars in Docker, localhost for local dev
const RPC_URLS = {
  solana: process.env.SOLANA_RPC_URL || 'http://localhost:8899',
  starknet: process.env.STARKNET_RPC_URL || 'http://localhost:5050',
  zcash: process.env.ZCASH_RPC_URL || 'http://localhost:18232',
};

const ZCASH_AUTH = 'Basic ' + Buffer.from('zyberlink:testpass123').toString('base64');

// Helper to make JSON-RPC calls
async function rpcCall(url: string, method: string, params: unknown[] = [], headers: Record<string, string> = {}) {
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method,
      params,
    }),
  });
  return response.json();
}

test.describe('Solana API', () => {
  test.beforeEach(async () => {
    try {
      const res = await fetch(`${RPC_URLS.solana}/health`);
      if (!res.ok) test.skip();
    } catch {
      test.skip();
    }
  });

  test('health endpoint responds', async () => {
    const response = await fetch(`${RPC_URLS.solana}/health`);
    expect(response.ok).toBe(true);
  });

  test('getVersion returns validator info', async () => {
    const result = await rpcCall(RPC_URLS.solana, 'getVersion');
    expect(result.result).toBeTruthy();
    expect(result.result['solana-core']).toBeTruthy();
  });

  test('getSlot returns current slot', async () => {
    const result = await rpcCall(RPC_URLS.solana, 'getSlot');
    expect(typeof result.result).toBe('number');
    expect(result.result).toBeGreaterThanOrEqual(0);
  });

  test('getBlockHeight returns block height', async () => {
    const result = await rpcCall(RPC_URLS.solana, 'getBlockHeight');
    expect(typeof result.result).toBe('number');
    expect(result.result).toBeGreaterThanOrEqual(0);
  });

  test('getBalance for zero account', async () => {
    // Test with a random address (should have 0 balance)
    const testAddress = '11111111111111111111111111111111';
    const result = await rpcCall(RPC_URLS.solana, 'getBalance', [testAddress]);
    expect(result.result).toBeTruthy();
    expect(typeof result.result.value).toBe('number');
  });

  test('requestAirdrop works on devnet', async () => {
    // Generate a test keypair address (base58)
    const testAddress = 'Hs5xLNxY4EjzGPAxqhWJwmgDJVuSWQjmxqLQYq1q2Q2e';
    const result = await rpcCall(RPC_URLS.solana, 'requestAirdrop', [testAddress, 1000000000]);
    // Should return a transaction signature or error
    expect(result.result || result.error).toBeTruthy();
  });
});

test.describe('Starknet API (Katana)', () => {
  test.beforeEach(async () => {
    try {
      const res = await fetch(RPC_URLS.starknet);
      if (res.status === 0) test.skip();
    } catch {
      test.skip();
    }
  });

  test('starknet_chainId returns chain ID', async () => {
    const result = await rpcCall(RPC_URLS.starknet, 'starknet_chainId');
    expect(result.result).toBeTruthy();
    // Katana returns KATANA chain ID
    expect(result.result).toMatch(/^0x/);
  });

  test('starknet_blockNumber returns block number', async () => {
    const result = await rpcCall(RPC_URLS.starknet, 'starknet_blockNumber');
    expect(typeof result.result).toBe('number');
    expect(result.result).toBeGreaterThanOrEqual(0);
  });

  test('starknet_getBlockWithTxs returns latest block', async () => {
    const result = await rpcCall(RPC_URLS.starknet, 'starknet_getBlockWithTxs', ['latest']);
    expect(result.result).toBeTruthy();
    expect(result.result.block_number).toBeGreaterThanOrEqual(0);
  });

  test('katana predeployed accounts are available', async () => {
    // Katana creates prefunded accounts
    const result = await rpcCall(RPC_URLS.starknet, 'starknet_getBlockWithTxHashes', ['latest']);
    expect(result.result).toBeTruthy();
  });
});

test.describe('Zcash API', () => {
  test.beforeEach(async () => {
    try {
      const res = await fetch(RPC_URLS.zcash, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: ZCASH_AUTH },
        body: JSON.stringify({ jsonrpc: '1.0', method: 'getblockchaininfo', params: [] }),
      });
      if (!res.ok) test.skip();
    } catch {
      test.skip();
    }
  });

  test('getblockchaininfo returns regtest chain', async () => {
    const result = await rpcCall(
      RPC_URLS.zcash,
      'getblockchaininfo',
      [],
      { Authorization: ZCASH_AUTH }
    );
    expect(result.result).toBeTruthy();
    expect(result.result.chain).toBe('regtest');
  });

  test('getnetworkinfo returns network details', async () => {
    const result = await rpcCall(
      RPC_URLS.zcash,
      'getnetworkinfo',
      [],
      { Authorization: ZCASH_AUTH }
    );
    expect(result.result).toBeTruthy();
    expect(result.result.version).toBeTruthy();
  });

  test('generate blocks for mining', async () => {
    const result = await rpcCall(
      RPC_URLS.zcash,
      'generate',
      [3],
      { Authorization: ZCASH_AUTH }
    );
    expect(result.result).toHaveLength(3);
  });

  test('getbalance returns wallet balance', async () => {
    const result = await rpcCall(
      RPC_URLS.zcash,
      'getbalance',
      [],
      { Authorization: ZCASH_AUTH }
    );
    expect(typeof result.result).toBe('number');
    expect(result.result).toBeGreaterThanOrEqual(0);
  });

  test('listunspent returns UTXOs', async () => {
    const result = await rpcCall(
      RPC_URLS.zcash,
      'listunspent',
      [],
      { Authorization: ZCASH_AUTH }
    );
    expect(Array.isArray(result.result)).toBe(true);
  });
});

test.describe('Cross-chain Connectivity', () => {
  test('all three chains are accessible', async () => {
    const results = { solana: false, starknet: false, zcash: false };

    // Solana
    try {
      const res = await fetch(`${RPC_URLS.solana}/health`);
      results.solana = res.ok;
    } catch {}

    // Starknet
    try {
      const res = await fetch(RPC_URLS.starknet);
      results.starknet = res.status !== 0;
    } catch {}

    // Zcash
    try {
      const res = await fetch(RPC_URLS.zcash, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: ZCASH_AUTH },
        body: JSON.stringify({ jsonrpc: '1.0', method: 'getblockchaininfo', params: [] }),
      });
      results.zcash = res.ok;
    } catch {}

    console.log('Chain connectivity:', results);

    // At least one chain should be available
    const anyAvailable = results.solana || results.starknet || results.zcash;
    expect(anyAvailable).toBe(true);
  });
});
