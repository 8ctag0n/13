export * from './types';
export { SolanaAdapter } from './solana';
export { StarknetAdapter } from './starknet';
export { ZcashAdapter } from './zcash';

import { SolanaAdapter } from './solana';
import { StarknetAdapter } from './starknet';
import { ZcashAdapter } from './zcash';
import type { ChainAdapter, SupportedChain } from './types';

export const CHAIN_ADAPTERS: Record<SupportedChain, ChainAdapter> = {
  solana: new SolanaAdapter(),
  starknet: new StarknetAdapter(),
  zcash: new ZcashAdapter()
};

export function getChainAdapter(chain: SupportedChain): ChainAdapter {
  const adapter = CHAIN_ADAPTERS[chain];
  if (!adapter) {
    throw new Error(`Unsupported chain: ${chain}`);
  }
  return adapter;
}
