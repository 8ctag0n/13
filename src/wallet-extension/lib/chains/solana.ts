import {
  address,
  createSolanaRpc,
  lamports,
  createKeyPairSignerFromBytes,
  createTransactionMessage,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstruction,
  signTransaction,
  getSignatureFromTransaction,
  compileTransaction,
  pipe
} from '@solana/kit';
import type { ChainAdapter, TxParams } from './types';
import type { DerivedKeypair } from '../crypto/keyring';
import bs58 from 'bs58';

const LAMPORTS_PER_SOL = 1_000_000_000n;

// System program transfer instruction
function createTransferInstruction(
  source: ReturnType<typeof address>,
  destination: ReturnType<typeof address>,
  amount: ReturnType<typeof lamports>
) {
  return {
    programAddress: address('11111111111111111111111111111111'),
    accounts: [
      { address: source, role: 3 }, // Writable + Signer
      { address: destination, role: 1 } // Writable
    ],
    data: new Uint8Array([
      2, 0, 0, 0, // Transfer instruction (index 2)
      ...new Uint8Array(new BigUint64Array([BigInt(amount)]).buffer)
    ])
  };
}

export class SolanaAdapter implements ChainAdapter {
  chainId = 'solana';
  name = 'Solana';
  symbol = 'SOL';

  getAddress(keypair: DerivedKeypair): string {
    return bs58.encode(keypair.publicKey);
  }

  async getBalance(addressStr: string, rpcUrl: string): Promise<string> {
    try {
      const rpc = createSolanaRpc(rpcUrl);
      const addr = address(addressStr);

      const response = await rpc.getBalance(addr).send();
      const balanceLamports = response.value;

      const balanceSol = Number(balanceLamports) / Number(LAMPORTS_PER_SOL);
      return balanceSol.toFixed(6);
    } catch (error) {
      console.error('[Solana Kit] Failed to fetch balance:', error);
      return '0';
    }
  }

  async buildTransaction(params: TxParams, rpcUrl?: string): Promise<any> {
    if (!params.from || !rpcUrl) {
      throw new Error('From address and RPC URL are required');
    }

    try {
      const rpc = createSolanaRpc(rpcUrl);

      // Get latest blockhash
      const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

      const fromAddr = address(params.from);
      const toAddr = address(params.to);
      const amountLamports = lamports(
        BigInt(Math.floor(parseFloat(params.amount) * Number(LAMPORTS_PER_SOL)))
      );

      // Create transfer instruction
      const transferIx = createTransferInstruction(fromAddr, toAddr, amountLamports);

      // Build transaction message using pipe
      const txMessage = pipe(
        createTransactionMessage({ version: 0 }),
        tx => setTransactionMessageFeePayer(fromAddr, tx),
        tx => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
        tx => appendTransactionMessageInstruction(transferIx, tx)
      );

      return {
        message: txMessage,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight
      };
    } catch (error) {
      console.error('[Solana Kit] Failed to build transaction:', error);
      throw error;
    }
  }

  async signTransaction(tx: any, keypair: DerivedKeypair): Promise<any> {
    try {
      // Create 64-byte secret key (32 private + 32 public)
      const fullSecretKey = new Uint8Array(64);
      fullSecretKey.set(keypair.secretKey, 0);
      fullSecretKey.set(keypair.publicKey, 32);

      const signer = await createKeyPairSignerFromBytes(fullSecretKey);

      // Compile and sign
      const compiledTx = compileTransaction(tx.message);
      const signedTx = await signTransaction([signer], compiledTx);

      return {
        ...tx,
        signedTransaction: signedTx,
        signer
      };
    } catch (error) {
      console.error('[Solana Kit] Failed to sign transaction:', error);
      throw error;
    }
  }

  async sendTransaction(signedTx: any, rpcUrl: string): Promise<string> {
    try {
      const rpc = createSolanaRpc(rpcUrl);

      // Get signature from signed transaction
      const sig = getSignatureFromTransaction(signedTx.signedTransaction);

      // Serialize and send
      const serialized = signedTx.signedTransaction.serialize();
      const base64Tx = Buffer.from(serialized).toString('base64');

      await rpc.sendTransaction(base64Tx, { encoding: 'base64' }).send();

      // Confirm transaction
      await rpc.confirmTransaction(sig, { commitment: 'confirmed' }).send();

      return sig;
    } catch (error) {
      console.error('[Solana Kit] Transaction failed:', error);
      throw new Error(`Transaction failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }
}
