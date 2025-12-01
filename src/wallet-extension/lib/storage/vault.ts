import type { EncryptedData } from '../crypto/encryption';
import type { ChainConfig, RPCConfig } from '../chains/types';

export interface Vault {
  privateKey: string;  // hex encoded expanded private key (64 bytes = 128 hex chars)
  createdAt: number;
  version: string;
}

export interface StorageData {
  vault?: EncryptedData;
  rpcConfig?: ChainConfig;
  settings?: WalletSettings;
  lastUnlock?: number;
}

export interface WalletSettings {
  autoLockMinutes: number;
  showTestnets: boolean;
  preferredCurrency: 'USD' | 'EUR' | 'BTC';
}

const STORAGE_KEYS = {
  VAULT: 'zyberlink_vault',
  RPC_CONFIG: 'zyberlink_rpc_config',
  SETTINGS: 'zyberlink_settings',
  LAST_UNLOCK: 'zyberlink_last_unlock'
} as const;

export async function saveEncryptedVault(vault: EncryptedData): Promise<void> {
  await chrome.storage.local.set({
    [STORAGE_KEYS.VAULT]: vault
  });
  console.log('[Storage] Vault saved');
}

export async function loadEncryptedVault(): Promise<EncryptedData | null> {
  const result = await chrome.storage.local.get(STORAGE_KEYS.VAULT);
  return result[STORAGE_KEYS.VAULT] || null;
}

export async function removeVault(): Promise<void> {
  await chrome.storage.local.remove(STORAGE_KEYS.VAULT);
  console.log('[Storage] Vault removed');
}

export async function hasVault(): Promise<boolean> {
  const vault = await loadEncryptedVault();
  return vault !== null;
}

export async function saveRPCConfig(config: ChainConfig): Promise<void> {
  await chrome.storage.local.set({
    [STORAGE_KEYS.RPC_CONFIG]: config
  });
  console.log('[Storage] RPC config saved');
}

export async function loadRPCConfig(): Promise<ChainConfig | null> {
  const result = await chrome.storage.local.get(STORAGE_KEYS.RPC_CONFIG);
  return result[STORAGE_KEYS.RPC_CONFIG] || null;
}

export async function getDefaultRPCConfig(): Promise<ChainConfig> {
  return {
    solana: {
      chainId: 'solana',
      endpoints: [
        { url: 'http://localhost:8899', name: 'Local', custom: false },
        { url: 'https://api.devnet.solana.com', name: 'Devnet', custom: false },
        { url: 'https://api.mainnet-beta.solana.com', name: 'Mainnet', custom: false }
      ],
      activeEndpoint: 'http://localhost:8899'
    },
    starknet: {
      chainId: 'starknet',
      endpoints: [
        { url: 'http://localhost:5050', name: 'Local (Katana)', custom: false },
        { url: 'https://starknet-sepolia.public.blastapi.io', name: 'Sepolia', custom: false },
        { url: 'https://starknet-mainnet.public.blastapi.io', name: 'Mainnet', custom: false }
      ],
      activeEndpoint: 'http://localhost:5050'
    },
    zcash: {
      chainId: 'zcash',
      endpoints: [
        { url: 'http://localhost:18232', name: 'Local (Regtest)', custom: false },
        { url: 'https://testnet.zcash.example.com', name: 'Testnet', custom: false }
      ],
      activeEndpoint: 'http://localhost:18232'
    }
  };
}

export async function saveSettings(settings: WalletSettings): Promise<void> {
  await chrome.storage.local.set({
    [STORAGE_KEYS.SETTINGS]: settings
  });
  console.log('[Storage] Settings saved');
}

export async function loadSettings(): Promise<WalletSettings> {
  const result = await chrome.storage.local.get(STORAGE_KEYS.SETTINGS);
  return result[STORAGE_KEYS.SETTINGS] || {
    autoLockMinutes: 15,
    showTestnets: false,
    preferredCurrency: 'USD'
  };
}

export async function updateLastUnlock(): Promise<void> {
  await chrome.storage.local.set({
    [STORAGE_KEYS.LAST_UNLOCK]: Date.now()
  });
}

export async function getLastUnlock(): Promise<number | null> {
  const result = await chrome.storage.local.get(STORAGE_KEYS.LAST_UNLOCK);
  return result[STORAGE_KEYS.LAST_UNLOCK] || null;
}

export async function clearAllData(): Promise<void> {
  await chrome.storage.local.clear();
  console.log('[Storage] All data cleared');
}
