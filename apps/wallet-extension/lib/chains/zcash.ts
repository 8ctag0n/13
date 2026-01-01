import type { ChainAdapter, TxParams, Transaction } from './types';
import type { DerivedKeypair } from '../crypto/keyring';
import bs58 from 'bs58';
import { sha256 } from '@noble/hashes/sha256';
import { ripemd160 } from '@noble/hashes/ripemd160';

// Zcash RPC configuration
const ZCASH_RPC_AUTH = 'zyberlink:testpass123';

interface ZcashRpcResponse {
  result: any;
  error: { code: number; message: string } | null;
  id: number;
}

export class ZcashAdapter implements ChainAdapter {
  chainId = 'zcash';
  name = 'Zcash';
  symbol = 'ZEC';

  // Store the z-address associated with this wallet
  private cachedZAddress: string | null = null;

  getAddress(keypair: DerivedKeypair): string {
    // Generate a proper Zcash transparent address (t-address)
    // Using RIPEMD160(SHA256(pubkey)) for the hash
    const pubkeyHash = ripemd160(sha256(keypair.publicKey));

    // Zcash regtest t-address prefix: 0x1D25 (tm...)
    // Mainnet would be 0x1CB8 (t1...)
    const versionBytes = new Uint8Array([0x1D, 0x25]); // regtest prefix
    const payload = new Uint8Array([...versionBytes, ...pubkeyHash]);

    // Calculate checksum (double SHA256, first 4 bytes)
    const checksum = sha256(sha256(payload)).slice(0, 4);
    const fullPayload = new Uint8Array([...payload, ...checksum]);

    return bs58.encode(fullPayload);
  }

  // Get or create a shielded z-address from the node
  async getShieldedAddress(rpcUrl: string): Promise<string> {
    if (this.cachedZAddress) {
      return this.cachedZAddress;
    }

    try {
      // First check if we have any existing z-addresses
      const listResult = await this.rpcCall(rpcUrl, 'z_listaddresses', []);
      if (listResult.result && listResult.result.length > 0) {
        this.cachedZAddress = listResult.result[0];
        return this.cachedZAddress;
      }

      // Generate new shielded address (Sapling)
      const newAddrResult = await this.rpcCall(rpcUrl, 'z_getnewaddress', ['sapling']);
      if (newAddrResult.result) {
        this.cachedZAddress = newAddrResult.result;
        return this.cachedZAddress;
      }

      throw new Error('Failed to generate z-address');
    } catch (error) {
      console.error('[Zcash] Failed to get shielded address:', error);
      throw error;
    }
  }

  async getBalance(address: string, rpcUrl: string): Promise<string> {
    try {
      const isShielded = address.startsWith('zs') || address.startsWith('zc') || address.startsWith('zregtestsapling');

      if (isShielded) {
        // Get shielded balance
        const result = await this.rpcCall(rpcUrl, 'z_getbalance', [address]);
        if (result.error) {
          console.error('[Zcash] z_getbalance error:', result.error);
          return '0';
        }
        return (result.result || 0).toFixed(8);
      } else {
        // For transparent addresses, use getreceivedbyaddress
        // First try to import the address if it doesn't exist
        await this.rpcCall(rpcUrl, 'importaddress', [address, '', false]).catch(() => {});

        const result = await this.rpcCall(rpcUrl, 'getreceivedbyaddress', [address, 0]);
        if (result.error) {
          // Try z_getbalance as fallback
          const zResult = await this.rpcCall(rpcUrl, 'z_gettotalbalance', []);
          if (zResult.result) {
            return zResult.result.transparent || '0';
          }
          return '0';
        }
        return (result.result || 0).toFixed(8);
      }
    } catch (error) {
      console.error('[Zcash] Failed to fetch balance:', error);
      return '0';
    }
  }

  // Get total wallet balance (transparent + shielded)
  async getTotalBalance(rpcUrl: string): Promise<{ transparent: string; shielded: string; total: string }> {
    try {
      const result = await this.rpcCall(rpcUrl, 'z_gettotalbalance', []);
      if (result.error) {
        throw new Error(result.error.message);
      }
      return {
        transparent: result.result.transparent || '0',
        shielded: result.result.private || '0',
        total: result.result.total || '0'
      };
    } catch (error) {
      console.error('[Zcash] Failed to get total balance:', error);
      return { transparent: '0', shielded: '0', total: '0' };
    }
  }

  async getTransactions(address: string, rpcUrl: string, limit = 20): Promise<Transaction[]> {
    try {
      // Get recent transactions from wallet
      const result = await this.rpcCall(rpcUrl, 'listtransactions', ['*', limit, 0]);

      if (result.error) {
        console.error('[Zcash] listtransactions error:', result.error);
        return [];
      }

      const transactions: Transaction[] = [];

      for (const tx of result.result || []) {
        const isReceive = tx.category === 'receive' || tx.category === 'generate';

        transactions.push({
          id: tx.txid,
          type: isReceive ? 'receive' : 'send',
          chain: 'zcash',
          amount: Math.abs(tx.amount || 0).toFixed(8),
          symbol: 'ZEC',
          from: isReceive ? (tx.address || '') : address,
          to: isReceive ? address : (tx.address || ''),
          timestamp: (tx.time || tx.blocktime || Date.now() / 1000) * 1000,
          status: tx.confirmations > 0 ? 'confirmed' : 'pending',
          signature: tx.txid
        });
      }

      return transactions;
    } catch (error) {
      console.error('[Zcash] Failed to fetch transactions:', error);
      return [];
    }
  }

  async buildTransaction(params: TxParams, rpcUrl?: string): Promise<any> {
    if (!params.from) {
      throw new Error('From address is required');
    }

    const isShieldedFrom = params.from.startsWith('zs') || params.from.startsWith('zc') || params.from.startsWith('zregtestsapling');
    const isShieldedTo = params.to.startsWith('zs') || params.to.startsWith('zc') || params.to.startsWith('zregtestsapling');

    return {
      from: params.from,
      to: params.to,
      amount: parseFloat(params.amount),
      memo: params.memo || '',
      shielded: isShieldedFrom || isShieldedTo,
      txType: this.getTxType(isShieldedFrom, isShieldedTo)
    };
  }

  private getTxType(fromShielded: boolean, toShielded: boolean): string {
    if (!fromShielded && !toShielded) return 't-to-t';  // Transparent to transparent
    if (!fromShielded && toShielded) return 't-to-z';   // Shielding
    if (fromShielded && !toShielded) return 'z-to-t';   // Deshielding
    return 'z-to-z';  // Fully shielded
  }

  async signTransaction(tx: any, keypair: DerivedKeypair): Promise<any> {
    // For Zcash, the node handles signing via z_sendmany
    // We just pass through the transaction data
    console.log('[Zcash] Transaction will be signed by node via z_sendmany');
    return tx;
  }

  async sendTransaction(signedTx: any, rpcUrl: string): Promise<string> {
    try {
      const recipients = [{
        address: signedTx.to,
        amount: signedTx.amount
      }];

      // Add encrypted memo for shielded transactions
      if (signedTx.memo && signedTx.shielded) {
        const memoHex = Buffer.from(signedTx.memo, 'utf8').toString('hex');
        (recipients[0] as any).memo = memoHex;
      }

      console.log('[Zcash] Sending transaction:', {
        from: signedTx.from,
        to: signedTx.to,
        amount: signedTx.amount,
        type: signedTx.txType
      });

      // Use z_sendmany for all transaction types
      const sendResult = await this.rpcCall(rpcUrl, 'z_sendmany', [
        signedTx.from,
        recipients,
        1,  // minconf
        0.0001  // fee
      ]);

      if (sendResult.error) {
        throw new Error(sendResult.error.message);
      }

      const operationId = sendResult.result;
      console.log('[Zcash] Operation started:', operationId);

      // Poll for operation status
      const txid = await this.waitForOperation(rpcUrl, operationId);
      console.log('[Zcash] Transaction confirmed:', txid);

      return txid;
    } catch (error) {
      console.error('[Zcash] Transaction failed:', error);
      throw new Error(`Transaction failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  // Wait for z_sendmany operation to complete
  private async waitForOperation(rpcUrl: string, operationId: string, maxAttempts = 60): Promise<string> {
    for (let i = 0; i < maxAttempts; i++) {
      const statusResult = await this.rpcCall(rpcUrl, 'z_getoperationstatus', [[operationId]]);

      if (statusResult.error) {
        throw new Error(statusResult.error.message);
      }

      const operations = statusResult.result;
      if (operations && operations.length > 0) {
        const op = operations[0];

        if (op.status === 'success') {
          return op.result?.txid || operationId;
        } else if (op.status === 'failed') {
          throw new Error(op.error?.message || 'Operation failed');
        }
        // Still executing, wait and retry
      }

      await new Promise(resolve => setTimeout(resolve, 1000));
    }

    throw new Error('Operation timed out');
  }

  // Mine blocks (regtest only) - useful for testing
  async mineBlocks(rpcUrl: string, count: number = 1): Promise<string[]> {
    try {
      const result = await this.rpcCall(rpcUrl, 'generate', [count]);
      if (result.error) {
        throw new Error(result.error.message);
      }
      return result.result;
    } catch (error) {
      console.error('[Zcash] Failed to mine blocks:', error);
      throw error;
    }
  }

  // Shield transparent funds to z-address
  async shieldFunds(rpcUrl: string, fromTAddress: string, toZAddress: string, amount?: number): Promise<string> {
    try {
      // If no amount specified, shield all available
      const params = amount
        ? [fromTAddress, toZAddress, 0.0001, amount]
        : [fromTAddress, toZAddress, 0.0001];

      const result = await this.rpcCall(rpcUrl, 'z_shieldcoinbase', params);
      if (result.error) {
        // Fall back to z_sendmany if z_shieldcoinbase not available
        const balance = await this.getBalance(fromTAddress, rpcUrl);
        const amountToShield = amount || (parseFloat(balance) - 0.0001);

        const sendResult = await this.rpcCall(rpcUrl, 'z_sendmany', [
          fromTAddress,
          [{ address: toZAddress, amount: amountToShield }],
          1,
          0.0001
        ]);

        if (sendResult.error) {
          throw new Error(sendResult.error.message);
        }

        return await this.waitForOperation(rpcUrl, sendResult.result);
      }
      return await this.waitForOperation(rpcUrl, result.result);
    } catch (error) {
      console.error('[Zcash] Failed to shield funds:', error);
      throw error;
    }
  }

  // RPC helper
  private async rpcCall(rpcUrl: string, method: string, params: any[] = []): Promise<ZcashRpcResponse> {
    const response = await fetch(rpcUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Basic ' + btoa(ZCASH_RPC_AUTH)
      },
      body: JSON.stringify({
        jsonrpc: '1.0',
        method,
        params,
        id: Date.now()
      })
    });

    if (!response.ok) {
      throw new Error(`RPC request failed: ${response.status}`);
    }

    return response.json();
  }
}
