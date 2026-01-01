/**
 * x402 Anti-Spam Payment Layer
 *
 * HTTP 402 Payment Required implementation for ZyberLink.
 * Prevents spam by requiring payment proof before witness upload.
 *
 * Flow:
 * 1. Client gets quote for operation
 * 2. Client builds and signs payment transaction
 * 3. Client submits signed tx and gets payment token
 * 4. Client uses payment token to upload witness and create job
 */

import {
  createSolanaRpc,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  getSignatureFromTransaction,
  getBase64EncodedWireTransaction,
  pipe,
  type Address,
  type TransactionSigner,
  type IInstruction,
} from '@solana/kit';

import { ZkCircuitType } from './types';

// =============================================================================
// Types
// =============================================================================

export interface X402Quote {
  /** Circuit type being quoted */
  circuitType: ZkCircuitType;
  /** Price in lamports */
  priceLamports: bigint;
  /** Price in SOL for display */
  priceSol: number;
  /** Quote expiration timestamp (unix seconds) */
  expiresAt: number;
  /** Quote ID for reference */
  quoteId: string;
  /** Recipient address for payment */
  paymentRecipient: Address;
}

export interface X402PaymentToken {
  /** Unique token ID */
  tokenId: string;
  /** Payer address */
  payer: Address;
  /** Amount paid in lamports */
  amountPaid: bigint;
  /** Transaction signature */
  txSignature: string;
  /** Token expiration (unix seconds) */
  expiresAt: number;
  /** Circuit type this payment is for */
  circuitType: ZkCircuitType;
  /** Whether this token has been used */
  used: boolean;
}

export interface X402Config {
  /** Backend URL for x402 endpoints */
  backendUrl: string;
  /** RPC URL for Solana */
  rpcUrl: string;
}

// =============================================================================
// X402 Client
// =============================================================================

/**
 * X402 Anti-Spam Payment Client
 *
 * @example
 * ```typescript
 * const x402 = new X402Client(config, signer);
 *
 * // Get quote
 * const quote = await x402.getQuote(ZkCircuitType.PrivateVote);
 *
 * // Pay and get token
 * const token = await x402.pay(quote);
 *
 * // Use token for witness upload
 * const commitment = await x402.uploadWitnessWithToken(witnessData, token);
 * ```
 */
export class X402Client {
  private config: X402Config;
  private signer: TransactionSigner;
  private rpc: ReturnType<typeof createSolanaRpc>;

  constructor(config: X402Config, signer: TransactionSigner) {
    this.config = config;
    this.signer = signer;
    this.rpc = createSolanaRpc(config.rpcUrl);
  }

  /**
   * Get a quote for a ZK proof operation
   *
   * @param circuitType - Type of circuit to generate proof for
   * @returns Quote with price and expiration
   */
  async getQuote(circuitType: ZkCircuitType): Promise<X402Quote> {
    const response = await fetch(`${this.config.backendUrl}/api/x402/quote`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        circuitType,
        payer: this.signer.address,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Failed to get quote: ${error.error || response.statusText}`);
    }

    const data = await response.json();
    return {
      circuitType: data.circuitType,
      priceLamports: BigInt(data.priceLamports),
      priceSol: data.priceSol,
      expiresAt: data.expiresAt,
      quoteId: data.quoteId,
      paymentRecipient: data.paymentRecipient as Address,
    };
  }

  /**
   * Pay for a quote and get a payment token
   *
   * This builds, signs, and submits a payment transaction,
   * then returns a token that can be used for witness upload.
   *
   * @param quote - Quote to pay for
   * @returns Payment token for subsequent operations
   */
  async pay(quote: X402Quote): Promise<X402PaymentToken> {
    // Check quote expiration
    if (Date.now() / 1000 > quote.expiresAt) {
      throw new Error('Quote has expired');
    }

    // Get payment instruction from backend
    const paymentInstruction = await this.getPaymentInstruction(quote);

    // Build transaction
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (tx) => setTransactionMessageFeePayerSigner(this.signer, tx),
      (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
      (tx) => appendTransactionMessageInstructions([paymentInstruction], tx)
    );

    // Sign transaction
    const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);
    const signature = getSignatureFromTransaction(signedTransaction);
    const encodedTransaction = getBase64EncodedWireTransaction(signedTransaction);

    // Submit signed transaction to backend for verification and submission
    const response = await fetch(`${this.config.backendUrl}/api/x402/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        quoteId: quote.quoteId,
        signedTransaction: encodedTransaction,
        signature,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Payment failed: ${error.error || response.statusText}`);
    }

    const data = await response.json();
    return {
      tokenId: data.tokenId,
      payer: this.signer.address,
      amountPaid: BigInt(data.amountPaid),
      txSignature: data.txSignature,
      expiresAt: data.expiresAt,
      circuitType: quote.circuitType,
      used: false,
    };
  }

  /**
   * Upload witness data using a payment token
   *
   * @param witnessData - Encrypted witness data
   * @param token - Payment token from pay()
   * @returns Witness commitment hash
   */
  async uploadWitnessWithToken(
    witnessData: Uint8Array,
    token: X402PaymentToken
  ): Promise<string> {
    if (token.used) {
      throw new Error('Payment token has already been used');
    }

    if (Date.now() / 1000 > token.expiresAt) {
      throw new Error('Payment token has expired');
    }

    const response = await fetch(`${this.config.backendUrl}/api/x402/witness`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/octet-stream',
        'X-Payment-Token': token.tokenId,
      },
      body: witnessData as unknown as BodyInit,
    });

    if (response.status === 402) {
      throw new Error('Payment required - invalid or expired token');
    }

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Failed to upload witness: ${error.error || response.statusText}`);
    }

    const data = await response.json();
    token.used = true;
    return data.commitment;
  }

  /**
   * Create a ZK job using a payment token
   *
   * @param params - Job creation parameters
   * @param token - Payment token
   * @returns Job ID and unsigned transaction
   */
  async createJobWithToken(
    params: {
      witnessCommitment: string;
      witnessSize: number;
      timeoutSeconds?: number;
    },
    token: X402PaymentToken
  ): Promise<{ jobId: number; unsignedTransaction: string }> {
    if (Date.now() / 1000 > token.expiresAt) {
      throw new Error('Payment token has expired');
    }

    const response = await fetch(`${this.config.backendUrl}/api/x402/create-job`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Payment-Token': token.tokenId,
      },
      body: JSON.stringify({
        circuitType: token.circuitType,
        witnessCommitment: params.witnessCommitment,
        witnessSize: params.witnessSize,
        timeoutSeconds: params.timeoutSeconds ?? 300,
        payer: this.signer.address,
      }),
    });

    if (response.status === 402) {
      throw new Error('Payment required - invalid or expired token');
    }

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Failed to create job: ${error.error || response.statusText}`);
    }

    return response.json();
  }

  /**
   * Complete flow: quote → pay → upload witness → create job
   *
   * @param circuitType - Circuit type
   * @param witnessData - Encrypted witness data
   * @param timeoutSeconds - Job timeout
   * @returns Job ID and witness commitment
   */
  async submitWithPayment(
    circuitType: ZkCircuitType,
    witnessData: Uint8Array,
    timeoutSeconds = 300
  ): Promise<{ jobId: number; witnessCommitment: string; paymentTx: string }> {
    // 1. Get quote
    const quote = await this.getQuote(circuitType);

    // 2. Pay
    const token = await this.pay(quote);

    // 3. Upload witness
    const witnessCommitment = await this.uploadWitnessWithToken(witnessData, token);

    // 4. Create job
    const { jobId, unsignedTransaction } = await this.createJobWithToken(
      {
        witnessCommitment,
        witnessSize: witnessData.length,
        timeoutSeconds,
      },
      token
    );

    // 5. Sign and submit job creation transaction
    const jobTxSignature = await this.signAndSubmitJobTransaction(unsignedTransaction);

    return {
      jobId,
      witnessCommitment,
      paymentTx: token.txSignature,
    };
  }

  /**
   * Check payment token status
   */
  async checkTokenStatus(tokenId: string): Promise<{
    valid: boolean;
    used: boolean;
    expiresAt: number;
  }> {
    const response = await fetch(
      `${this.config.backendUrl}/api/x402/token/${tokenId}/status`
    );

    if (!response.ok) {
      throw new Error('Failed to check token status');
    }

    return response.json();
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private async getPaymentInstruction(quote: X402Quote): Promise<IInstruction> {
    const response = await fetch(
      `${this.config.backendUrl}/api/x402/build-payment`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          quoteId: quote.quoteId,
          payer: this.signer.address,
          amount: quote.priceLamports.toString(),
          recipient: quote.paymentRecipient,
        }),
      }
    );

    if (!response.ok) {
      throw new Error('Failed to build payment instruction');
    }

    const data = await response.json();
    return data.instruction;
  }

  private async signAndSubmitJobTransaction(unsignedTxBase64: string): Promise<string> {
    // Decode and sign the transaction
    const txBytes = Uint8Array.from(atob(unsignedTxBase64), (c) => c.charCodeAt(0));

    // Get fresh blockhash
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    // For versioned transactions, we need to resign with the signer
    // This is a simplified version - in production, properly decode and resign
    const response = await fetch(`${this.config.backendUrl}/api/x402/submit-job`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        unsignedTransaction: unsignedTxBase64,
        signerAddress: this.signer.address,
      }),
    });

    if (!response.ok) {
      throw new Error('Failed to submit job transaction');
    }

    const data = await response.json();
    return data.signature;
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Create x402 client from ZkClient config
 */
export function createX402Client(
  backendUrl: string,
  rpcUrl: string,
  signer: TransactionSigner
): X402Client {
  return new X402Client({ backendUrl, rpcUrl }, signer);
}

/**
 * Get estimated price for a circuit type (no auth required)
 */
export async function getEstimatedPrice(
  backendUrl: string,
  circuitType: ZkCircuitType
): Promise<{ priceLamports: bigint; priceSol: number }> {
  const response = await fetch(`${backendUrl}/api/x402/estimate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ circuitType }),
  });

  if (!response.ok) {
    throw new Error('Failed to get price estimate');
  }

  const data = await response.json();
  return {
    priceLamports: BigInt(data.priceLamports),
    priceSol: data.priceSol,
  };
}
