import type { RPCConfig, RPCEndpoint, SupportedChain } from '../chains/types';
import { loadRPCConfig, saveRPCConfig, getDefaultRPCConfig } from '../storage/vault';

export class RPCManager {
  private config: Map<SupportedChain, RPCConfig> = new Map();

  async initialize(): Promise<void> {
    let config = await loadRPCConfig();

    if (!config) {
      config = await getDefaultRPCConfig();
      await saveRPCConfig(config);
    }

    this.config.set('solana', config.solana);
    this.config.set('starknet', config.starknet);
    this.config.set('zcash', config.zcash);

    console.log('[RPCManager] Initialized with config:', config);
  }

  getActiveEndpoint(chain: SupportedChain): string {
    const chainConfig = this.config.get(chain);
    if (!chainConfig) {
      throw new Error(`No RPC config for chain: ${chain}`);
    }
    return chainConfig.activeEndpoint;
  }

  getEndpoints(chain: SupportedChain): RPCEndpoint[] {
    const chainConfig = this.config.get(chain);
    if (!chainConfig) {
      throw new Error(`No RPC config for chain: ${chain}`);
    }
    return chainConfig.endpoints;
  }

  async setActiveEndpoint(chain: SupportedChain, url: string): Promise<void> {
    const chainConfig = this.config.get(chain);
    if (!chainConfig) {
      throw new Error(`No RPC config for chain: ${chain}`);
    }

    const endpoint = chainConfig.endpoints.find(e => e.url === url);
    if (!endpoint) {
      throw new Error(`Endpoint not found: ${url}`);
    }

    chainConfig.activeEndpoint = url;
    this.config.set(chain, chainConfig);

    await this.saveConfig();
    console.log(`[RPCManager] Active endpoint for ${chain} set to ${url}`);
  }

  async addCustomEndpoint(chain: SupportedChain, endpoint: RPCEndpoint): Promise<void> {
    const chainConfig = this.config.get(chain);
    if (!chainConfig) {
      throw new Error(`No RPC config for chain: ${chain}`);
    }

    const exists = chainConfig.endpoints.some(e => e.url === endpoint.url);
    if (exists) {
      throw new Error(`Endpoint already exists: ${endpoint.url}`);
    }

    chainConfig.endpoints.push({ ...endpoint, custom: true });
    this.config.set(chain, chainConfig);

    await this.saveConfig();
    console.log(`[RPCManager] Custom endpoint added for ${chain}:`, endpoint);
  }

  async removeCustomEndpoint(chain: SupportedChain, url: string): Promise<void> {
    const chainConfig = this.config.get(chain);
    if (!chainConfig) {
      throw new Error(`No RPC config for chain: ${chain}`);
    }

    const endpoint = chainConfig.endpoints.find(e => e.url === url);
    if (!endpoint || !endpoint.custom) {
      throw new Error('Can only remove custom endpoints');
    }

    chainConfig.endpoints = chainConfig.endpoints.filter(e => e.url !== url);

    if (chainConfig.activeEndpoint === url) {
      chainConfig.activeEndpoint = chainConfig.endpoints[0]?.url || '';
    }

    this.config.set(chain, chainConfig);
    await this.saveConfig();
    console.log(`[RPCManager] Custom endpoint removed for ${chain}: ${url}`);
  }

  async checkHealth(url: string): Promise<boolean> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'eth_blockNumber',
          params: [],
          id: 1
        }),
        signal: controller.signal
      });

      clearTimeout(timeoutId);

      return response.ok;
    } catch (error) {
      console.error(`[RPCManager] Health check failed for ${url}:`, error);
      return false;
    }
  }

  private async saveConfig(): Promise<void> {
    const config = {
      solana: this.config.get('solana')!,
      starknet: this.config.get('starknet')!,
      zcash: this.config.get('zcash')!
    };
    await saveRPCConfig(config);
  }
}

export const rpcManager = new RPCManager();
