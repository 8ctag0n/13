import { RpcProvider, stark, ec, hash, CallData, cairo } from 'starknet';
import type { ChainAdapter, TxParams, Transaction } from './types';
import type { DerivedKeypair } from '../crypto/keyring';

// ETH contract address on Starknet
const ETH_CONTRACT = '0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7';

export class StarknetAdapter implements ChainAdapter {
  chainId = 'starknet';
  name = 'Starknet';
  symbol = 'ETH';

  getAddress(keypair: DerivedKeypair): string {
    // Derive Starknet address from private key
    // Note: This is a "potential" address - it needs to be deployed as an account contract
    const privateKey = '0x' + Buffer.from(keypair.secretKey.slice(0, 32)).toString('hex');
    const starkKeyPub = ec.starkCurve.getStarkKey(privateKey);

    // Calculate address using ArgentX account class hash (for compatibility)
    const argentXaccountClassHash = '0x01a736d6ed154502257f02b1ccdf4d9d1089f80811cd6acad48e6b6a9d1f2003';
    const constructorCallData = CallData.compile({ signer: starkKeyPub, guardian: '0x0' });

    const address = hash.calculateContractAddressFromHash(
      starkKeyPub,
      argentXaccountClassHash,
      constructorCallData,
      0
    );

    return address;
  }

  async getBalance(address: string, rpcUrl: string): Promise<string> {
    try {
      const provider = new RpcProvider({ nodeUrl: rpcUrl });

      // Call ETH contract balanceOf
      const result = await provider.callContract({
        contractAddress: ETH_CONTRACT,
        entrypoint: 'balanceOf',
        calldata: [address]
      });

      // Result is uint256 (low, high)
      const low = BigInt(result[0]);
      const high = BigInt(result[1] || '0');
      const balance = low + (high << 128n);

      // Convert from wei (18 decimals)
      const balanceEth = Number(balance) / 1e18;
      return balanceEth.toFixed(6);
    } catch (error) {
      console.error('[Starknet] Failed to fetch balance:', error);
      return '0';
    }
  }

  async getTransactions(address: string, rpcUrl: string, limit = 20): Promise<Transaction[]> {
    try {
      const provider = new RpcProvider({ nodeUrl: rpcUrl });

      // Get transaction receipts for account - Starknet RPC doesn't have direct tx history
      // We use events from the ETH contract Transfer event
      const events = await provider.getEvents({
        address: ETH_CONTRACT,
        from_block: { block_number: 0 },
        to_block: 'latest',
        keys: [
          // Transfer event selector
          ['0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9']
        ],
        chunk_size: limit
      });

      const transactions: Transaction[] = [];

      for (const event of events.events) {
        try {
          const from = event.data[0];
          const to = event.data[1];
          const amountLow = BigInt(event.data[2] || '0');
          const amountHigh = BigInt(event.data[3] || '0');
          const amount = amountLow + (amountHigh << 128n);
          const amountEth = Number(amount) / 1e18;

          // Check if this address is sender or receiver
          const normalizedAddress = address.toLowerCase();
          const normalizedFrom = from?.toLowerCase();
          const normalizedTo = to?.toLowerCase();

          if (normalizedFrom !== normalizedAddress && normalizedTo !== normalizedAddress) {
            continue;
          }

          const isReceive = normalizedTo === normalizedAddress;

          transactions.push({
            id: event.transaction_hash,
            type: isReceive ? 'receive' : 'send',
            chain: 'starknet',
            amount: amountEth.toFixed(6),
            symbol: 'ETH',
            from: from || '',
            to: to || '',
            timestamp: Date.now(), // Starknet events don't include timestamp directly
            status: 'confirmed',
            signature: event.transaction_hash
          });
        } catch (parseError) {
          console.warn('[Starknet] Failed to parse event:', parseError);
        }
      }

      return transactions.slice(0, limit);
    } catch (error) {
      console.error('[Starknet] Failed to fetch transactions:', error);
      return [];
    }
  }

  async buildTransaction(params: TxParams, rpcUrl?: string): Promise<any> {
    if (!params.from) {
      throw new Error('From address is required');
    }

    const amountWei = cairo.uint256(BigInt(Math.floor(parseFloat(params.amount) * 1e18)));

    return {
      contractAddress: ETH_CONTRACT,
      entrypoint: 'transfer',
      calldata: CallData.compile({
        recipient: params.to,
        amount: amountWei
      }),
      senderAddress: params.from
    };
  }

  async signTransaction(tx: any, keypair: DerivedKeypair): Promise<any> {
    // For MVP: return transaction with signature placeholder
    // Full implementation would use Account.signTransaction
    const privateKey = '0x' + Buffer.from(keypair.secretKey.slice(0, 32)).toString('hex');

    console.warn('[Starknet] Using simplified signing - full Account abstraction not implemented');

    return {
      ...tx,
      signature: [privateKey.slice(0, 66)] // Placeholder
    };
  }

  async sendTransaction(signedTx: any, rpcUrl: string): Promise<string> {
    try {
      const provider = new RpcProvider({ nodeUrl: rpcUrl });

      // For MVP: use invoke transaction
      const response = await provider.invokeFunction({
        contractAddress: signedTx.contractAddress,
        entrypoint: signedTx.entrypoint,
        calldata: signedTx.calldata
      }, signedTx.signature);

      return response.transaction_hash;
    } catch (error) {
      console.error('[Starknet] Transaction failed:', error);
      throw new Error(`Transaction failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }
}
