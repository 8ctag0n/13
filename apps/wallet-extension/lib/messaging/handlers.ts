import type { Message, MessageResponse, WalletState } from './types';
import { deriveAllKeypairs, hexToBytes, generatePrivateKey, bytesToHex, createKeypair, getExpandedPrivateKey, type WalletKeys } from '../crypto/keyring';
import { encryptVault, decryptVault } from '../crypto/encryption';
import { saveEncryptedVault, loadEncryptedVault, hasVault, updateLastUnlock, type Vault } from '../storage/vault';
import { getChainAdapter } from '../chains';
import { rpcManager } from '../rpc/manager';

class WalletStateManager {
  private isUnlocked = false;
  private keys: WalletKeys | null = null;
  private autoLockTimer: number | null = null;

  async initialize(): Promise<void> {
    await rpcManager.initialize();
    console.log('[WalletState] Initialized');
  }

  isWalletUnlocked(): boolean {
    return this.isUnlocked;
  }

  getKeys(): WalletKeys | null {
    return this.keys;
  }

  async unlock(password: string): Promise<WalletState> {
    const encryptedVault = await loadEncryptedVault();
    if (!encryptedVault) {
      throw new Error('No wallet found');
    }

    try {
      const vault: Vault = await decryptVault(encryptedVault, password);
      const privateKeyBytes = hexToBytes(vault.privateKey);
      this.keys = deriveAllKeypairs(privateKeyBytes);
      this.isUnlocked = true;

      await updateLastUnlock();
      this.startAutoLockTimer();

      console.log('[WalletState] Wallet unlocked');

      return this.getState();
    } catch (error) {
      throw new Error('Incorrect password');
    }
  }

  lock(): void {
    this.isUnlocked = false;
    this.keys = null;
    this.clearAutoLockTimer();
    console.log('[WalletState] Wallet locked');
  }

  async createWallet(password: string, privateKeyHex?: string): Promise<WalletState> {
    if (await hasVault()) {
      throw new Error('Wallet already exists');
    }

    // Generate new key if not provided, or use provided key
    let finalPrivateKeyHex: string;
    if (privateKeyHex) {
      finalPrivateKeyHex = privateKeyHex;
    } else {
      // Generate new 32-byte key and expand it
      const newKey = generatePrivateKey();
      const keypair = createKeypair(newKey);
      const expanded = getExpandedPrivateKey(keypair);
      finalPrivateKeyHex = bytesToHex(expanded);
    }

    const vault: Vault = {
      privateKey: finalPrivateKeyHex,
      createdAt: Date.now(),
      version: '0.2.0'
    };

    const encryptedVault = await encryptVault(vault, password);
    await saveEncryptedVault(encryptedVault);

    const privateKeyBytes = hexToBytes(finalPrivateKeyHex);
    this.keys = deriveAllKeypairs(privateKeyBytes);
    this.isUnlocked = true;

    await updateLastUnlock();
    this.startAutoLockTimer();

    console.log('[WalletState] Wallet created');

    return this.getState();
  }

  async getState(): Promise<WalletState> {
    const vaultExists = await hasVault();

    if (!this.isUnlocked || !this.keys) {
      return {
        isLocked: true,
        hasVault: vaultExists
      };
    }

    const solanaAdapter = getChainAdapter('solana');
    const starknetAdapter = getChainAdapter('starknet');
    const zcashAdapter = getChainAdapter('zcash');

    return {
      isLocked: false,
      hasVault: vaultExists,
      addresses: {
        solana: solanaAdapter.getAddress(this.keys.solana),
        starknet: starknetAdapter.getAddress(this.keys.starknet),
        zcash: zcashAdapter.getAddress(this.keys.zcash)
      }
    };
  }

  private startAutoLockTimer(): void {
    this.clearAutoLockTimer();

    const AUTO_LOCK_MINUTES = 15;
    this.autoLockTimer = setTimeout(() => {
      this.lock();
      console.log('[WalletState] Auto-locked after inactivity');
    }, AUTO_LOCK_MINUTES * 60 * 1000) as unknown as number;
  }

  private clearAutoLockTimer(): void {
    if (this.autoLockTimer !== null) {
      clearTimeout(this.autoLockTimer);
      this.autoLockTimer = null;
    }
  }
}

const walletState = new WalletStateManager();

walletState.initialize().catch(error => {
  console.error('[WalletState] Initialization failed:', error);
});

export async function handleMessage(message: Message): Promise<any> {
  console.log('[MessageHandler] Handling:', message.type);

  try {
    switch (message.type) {
      case 'GET_WALLET_STATE':
        return await walletState.getState();

      case 'CREATE_WALLET':
        return await walletState.createWallet(message.password, message.privateKey);

      case 'IMPORT_WALLET':
        return await walletState.createWallet(message.password, message.privateKey);

      case 'UNLOCK_WALLET':
        return await walletState.unlock(message.password);

      case 'LOCK_WALLET':
        walletState.lock();
        return { success: true };

      case 'CONNECT': {
        if (!walletState.isWalletUnlocked()) {
          throw new Error('Wallet is locked');
        }

        const keys = walletState.getKeys();
        if (!keys || !message.chain) {
          throw new Error('Invalid state');
        }

        const adapter = getChainAdapter(message.chain);
        const address = adapter.getAddress(keys[message.chain]);

        return {
          publicKey: address,
          chain: message.chain
        };
      }

      case 'DISCONNECT':
        return { success: true };

      case 'GET_ADDRESS': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const adapter = getChainAdapter(message.chain);
        return adapter.getAddress(keys[message.chain]);
      }

      case 'GET_BALANCE': {
        if (!message.chain) {
          throw new Error('Chain is required');
        }

        const adapter = getChainAdapter(message.chain);
        const rpcUrl = rpcManager.getActiveEndpoint(message.chain);

        let address = message.address;
        if (!address) {
          if (!walletState.isWalletUnlocked()) {
            throw new Error('Wallet is locked');
          }
          const keys = walletState.getKeys();
          if (!keys) {
            throw new Error('No keys available');
          }
          address = adapter.getAddress(keys[message.chain]);
        }

        const balance = await adapter.getBalance(address, rpcUrl);
        return { balance, address, chain: message.chain };
      }

      case 'SIGN_MESSAGE': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        console.warn('[MessageHandler] Message signing not fully implemented');
        return {
          signature: 'placeholder_signature',
          publicKey: keys[message.chain].publicKey
        };
      }

      case 'SIGN_TRANSACTION': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const adapter = getChainAdapter(message.chain);
        const signedTx = await adapter.signTransaction(message.transaction, keys[message.chain]);

        return signedTx;
      }

      case 'SIGN_AND_SEND_TRANSACTION': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const adapter = getChainAdapter(message.chain);
        const rpcUrl = rpcManager.getActiveEndpoint(message.chain);

        const signedTx = await adapter.signTransaction(message.transaction, keys[message.chain]);
        const signature = await adapter.sendTransaction(signedTx, rpcUrl);

        return { signature };
      }

      case 'BUILD_TRANSACTION': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const adapter = getChainAdapter(message.chain);
        const rpcUrl = rpcManager.getActiveEndpoint(message.chain);

        const tx = await adapter.buildTransaction(message.params, rpcUrl);
        return tx;
      }

      case 'GET_RPC_ENDPOINTS': {
        if (!message.chain) {
          throw new Error('Chain is required');
        }

        const endpoints = rpcManager.getEndpoints(message.chain);
        const activeEndpoint = rpcManager.getActiveEndpoint(message.chain);

        return { endpoints, activeEndpoint };
      }

      case 'SET_ACTIVE_RPC': {
        if (!message.chain || !message.url) {
          throw new Error('Chain and URL are required');
        }

        await rpcManager.setActiveEndpoint(message.chain, message.url);
        return { success: true };
      }

      case 'ADD_CUSTOM_RPC': {
        if (!message.chain || !message.endpoint) {
          throw new Error('Chain and endpoint are required');
        }

        await rpcManager.addCustomEndpoint(message.chain, message.endpoint);
        return { success: true };
      }

      case 'REMOVE_CUSTOM_RPC': {
        if (!message.chain || !message.url) {
          throw new Error('Chain and URL are required');
        }

        await rpcManager.removeCustomEndpoint(message.chain, message.url);
        return { success: true };
      }

      case 'SEND_TRANSACTION': {
        if (!walletState.isWalletUnlocked() || !message.chain) {
          throw new Error('Wallet is locked or invalid chain');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const adapter = getChainAdapter(message.chain);
        const rpcUrl = rpcManager.getActiveEndpoint(message.chain);
        const keypair = keys[message.chain];

        // Build transaction
        const tx = await adapter.buildTransaction({
          from: adapter.getAddress(keypair),
          to: message.to,
          amount: message.amount,
          memo: message.memo
        }, rpcUrl);

        // Sign transaction
        const signedTx = await adapter.signTransaction(tx, keypair);

        // Send transaction
        const signature = await adapter.sendTransaction(signedTx, rpcUrl);

        return {
          signature,
          chain: message.chain,
          to: message.to,
          amount: message.amount
        };
      }

      case 'EXPORT_PRIVATE_KEY': {
        if (!message.password) {
          throw new Error('Password is required');
        }

        // Verify password by attempting to decrypt vault
        const encryptedVault = await loadEncryptedVault();
        if (!encryptedVault) {
          throw new Error('No wallet found');
        }

        try {
          const vault: Vault = await decryptVault(encryptedVault, message.password);
          return {
            privateKey: vault.privateKey
          };
        } catch (error) {
          throw new Error('Invalid password');
        }
      }

      case 'ESTIMATE_FEE': {
        if (!message.chain) {
          throw new Error('Chain is required');
        }

        // Return estimated fees per chain (simplified)
        const fees: Record<string, { slow: string; normal: string; fast: string }> = {
          solana: { slow: '0.000005', normal: '0.00001', fast: '0.00005' },
          starknet: { slow: '0.0001', normal: '0.0005', fast: '0.001' },
          zcash: { slow: '0.0001', normal: '0.0001', fast: '0.0001' }
        };

        return fees[message.chain] || fees.solana;
      }

      case 'GET_TRANSACTIONS': {
        if (!walletState.isWalletUnlocked()) {
          throw new Error('Wallet is locked');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const allTransactions: any[] = [];

        // Fetch from all chains in parallel
        const chains: Array<'solana' | 'starknet' | 'zcash'> = ['solana', 'starknet', 'zcash'];

        await Promise.all(chains.map(async (chain) => {
          try {
            const adapter = getChainAdapter(chain);
            const rpcUrl = rpcManager.getActiveEndpoint(chain);
            const address = adapter.getAddress(keys[chain]);
            const txs = await adapter.getTransactions(address, rpcUrl, 20);
            allTransactions.push(...txs);
          } catch (error) {
            console.warn(`[Transactions] Failed to fetch ${chain}:`, error);
          }
        }));

        // Sort by timestamp descending
        allTransactions.sort((a, b) => b.timestamp - a.timestamp);

        // Also merge with locally saved transactions
        const localResult = await chrome.storage.local.get('zyberlink_transactions');
        const localTxs = localResult.zyberlink_transactions || [];

        // Merge and dedupe by id
        const txMap = new Map();
        for (const tx of [...allTransactions, ...localTxs]) {
          if (!txMap.has(tx.id)) {
            txMap.set(tx.id, tx);
          }
        }

        return Array.from(txMap.values()).sort((a, b) => b.timestamp - a.timestamp).slice(0, 100);
      }

      case 'SAVE_TRANSACTION': {
        // Save a transaction to local storage
        const { transaction } = message;
        if (!transaction) {
          throw new Error('Transaction data required');
        }

        const result = await chrome.storage.local.get('zyberlink_transactions');
        const transactions = result.zyberlink_transactions || [];

        transactions.unshift({
          ...transaction,
          id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          timestamp: Date.now()
        });

        // Keep only last 100 transactions
        const trimmed = transactions.slice(0, 100);
        await chrome.storage.local.set({ zyberlink_transactions: trimmed });

        return { success: true };
      }

      case 'GET_ZCASH_SHIELDED_ADDRESS': {
        // Get or create a shielded z-address from the Zcash node
        const zcashAdapter = getChainAdapter('zcash');
        const rpcUrl = rpcManager.getActiveEndpoint('zcash');

        // Cast to access Zcash-specific methods
        const zAddr = await (zcashAdapter as any).getShieldedAddress(rpcUrl);
        return { zAddress: zAddr };
      }

      case 'GET_ZCASH_TOTAL_BALANCE': {
        // Get both transparent and shielded Zcash balances
        const zcashAdapter = getChainAdapter('zcash');
        const rpcUrl = rpcManager.getActiveEndpoint('zcash');

        const totalBalance = await (zcashAdapter as any).getTotalBalance(rpcUrl);
        return totalBalance;
      }

      case 'SHIELD_ZCASH_FUNDS': {
        // Shield transparent funds to z-address
        if (!walletState.isWalletUnlocked()) {
          throw new Error('Wallet is locked');
        }

        const keys = walletState.getKeys();
        if (!keys) {
          throw new Error('No keys available');
        }

        const zcashAdapter = getChainAdapter('zcash');
        const rpcUrl = rpcManager.getActiveEndpoint('zcash');
        const tAddress = zcashAdapter.getAddress(keys.zcash);

        // Get shielded address
        const zAddress = await (zcashAdapter as any).getShieldedAddress(rpcUrl);

        // Shield funds
        const txid = await (zcashAdapter as any).shieldFunds(
          rpcUrl,
          tAddress,
          zAddress,
          message.amount
        );

        return { txid, from: tAddress, to: zAddress };
      }

      case 'MINE_ZCASH_BLOCKS': {
        // Mine blocks on regtest (for testing)
        const zcashAdapter = getChainAdapter('zcash');
        const rpcUrl = rpcManager.getActiveEndpoint('zcash');

        const blocks = await (zcashAdapter as any).mineBlocks(rpcUrl, message.count || 1);
        return { blocks };
      }

      default:
        throw new Error(`Unknown message type: ${message.type}`);
    }
  } catch (error) {
    console.error('[MessageHandler] Error:', error);
    throw error;
  }
}
