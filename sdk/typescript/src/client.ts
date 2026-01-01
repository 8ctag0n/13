import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  getSignatureFromTransaction,
  getBase64EncodedWireTransaction,
  pipe,
  getProgramDerivedAddress,
  getAddressEncoder,
  getU64Encoder,
  type Address,
  type TransactionSigner,
} from '@solana/kit';

import {
  NetworkConfig,
  NetworkName,
  NETWORKS,
  ZyberOptions,
  JobOptions,
  FheOperation,
  PredicateOp,
  CostEstimate,
  PreparedOperation,
  OperationPdas,
  costToSol,
} from './types';

// =============================================================================
// Zyber - Main SDK Entry Point
// =============================================================================

/**
 * ZyberLink SDK Client
 *
 * High-level client for privacy-preserving FHE computation on Solana.
 *
 * @example
 * ```typescript
 * import { Zyber } from '@zyberlink/sdk';
 *
 * // Connect with signer
 * const zyber = await Zyber.connect('devnet', wallet);
 *
 * // Compute sum of private values
 * const sum = await zyber.sum([100, 200, 300]);
 * console.log('Sum:', sum);
 * ```
 */
export class Zyber {
  private config: NetworkConfig;
  private rpc: ReturnType<typeof createSolanaRpc>;
  private rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions>;
  private signer: TransactionSigner;
  private defaultJobOptions: JobOptions;

  private constructor(
    config: NetworkConfig,
    rpc: ReturnType<typeof createSolanaRpc>,
    rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions>,
    signer: TransactionSigner,
    defaultJobOptions: JobOptions
  ) {
    this.config = config;
    this.rpc = rpc;
    this.rpcSubscriptions = rpcSubscriptions;
    this.signer = signer;
    this.defaultJobOptions = defaultJobOptions;
  }

  /**
   * Connect to a network with a signer
   *
   * @example
   * ```typescript
   * const zyber = await Zyber.connect('devnet', wallet);
   * ```
   */
  static async connect(
    network: NetworkName,
    signer: TransactionSigner,
    options?: Partial<ZyberOptions>
  ): Promise<Zyber> {
    const config = { ...NETWORKS[network] };

    if (options?.rpcUrl) config.rpcUrl = options.rpcUrl;
    if (options?.wsUrl) config.wsUrl = options.wsUrl;
    if (options?.backendUrl) config.backendUrl = options.backendUrl;
    if (options?.programId) config.programId = options.programId;

    const rpc = createSolanaRpc(config.rpcUrl);
    const rpcSubscriptions = createSolanaRpcSubscriptions(config.wsUrl);

    const defaultJobOptions: JobOptions = {
      priceLamports: 10_000_000n, // 0.01 SOL
      timeoutSeconds: 120,
      requiredProvers: 2,
      ...options?.defaultJobOptions,
    };

    return new Zyber(config, rpc, rpcSubscriptions, signer, defaultJobOptions);
  }

  // ===========================================================================
  // Properties
  // ===========================================================================

  /** Get wallet address */
  get address(): Address {
    return this.signer.address;
  }

  /** Get network name */
  get network(): NetworkName {
    return this.config.name;
  }

  // ===========================================================================
  // One-Liner Operations (Layer 0)
  // ===========================================================================

  /**
   * Compute sum of values
   *
   * @example
   * ```typescript
   * const total = await zyber.sum([100, 200, 300]);
   * // total = 600
   * ```
   */
  async sum(values: number[], options?: JobOptions): Promise<number> {
    return this.compute({ type: 'Sum', expectedCount: values.length }, values, options);
  }

  /**
   * Compute average of values
   *
   * @example
   * ```typescript
   * const avg = await zyber.average([10, 20, 30]);
   * // avg = 20
   * ```
   */
  async average(values: number[], options?: JobOptions): Promise<number> {
    const sum = await this.compute({ type: 'Sum', expectedCount: values.length }, values, options);
    return Math.floor(sum / values.length);
  }

  /**
   * Count values matching a condition
   *
   * @example
   * ```typescript
   * const adultCount = await zyber.countIf([18, 25, 16, 30, 15], '>=', 18);
   * // adultCount = 3
   * ```
   */
  async countIf(
    values: number[],
    op: PredicateOp,
    threshold: number,
    options?: JobOptions
  ): Promise<number> {
    return this.compute(
      {
        type: 'CountIf',
        expectedCount: values.length,
        predicate: { op, value: threshold },
      },
      values,
      options
    );
  }

  /**
   * Check if any value meets a threshold
   *
   * @example
   * ```typescript
   * const hasAdult = await zyber.threshold([18, 16, 15], '>=', 18);
   * // hasAdult = true
   * ```
   */
  async threshold(
    values: number[],
    op: PredicateOp,
    threshold: number,
    options?: JobOptions
  ): Promise<boolean> {
    const count = await this.countIf(values, op, threshold, options);
    return count > 0;
  }

  /**
   * Proof of Innocence - verify wallet has no sanctioned transactions
   *
   * @example
   * ```typescript
   * const isClean = await zyber.proofOfInnocence(walletTxIds, sanctionsList);
   * // isClean = true (no matches found)
   * ```
   */
  async proofOfInnocence(
    walletTransactions: number[],
    sanctionsList: number[],
    options?: JobOptions
  ): Promise<boolean> {
    for (const txId of walletTransactions) {
      const matches = await this.countIf(sanctionsList, '==', txId, options);
      if (matches > 0) {
        return false;
      }
    }
    return true;
  }

  // ===========================================================================
  // Prepared Operations (Layer 2)
  // ===========================================================================

  /**
   * Prepare a sum operation without executing
   */
  async prepareSum(values: number[], options?: JobOptions): Promise<PreparedOperation> {
    return this.prepare({ type: 'Sum', expectedCount: values.length }, values, options);
  }

  /**
   * Prepare an average operation without executing
   */
  async prepareAverage(values: number[], options?: JobOptions): Promise<PreparedOperation> {
    return this.prepare({ type: 'Sum', expectedCount: values.length }, values, options);
  }

  /**
   * Prepare a countIf operation without executing
   */
  async prepareCountIf(
    values: number[],
    op: PredicateOp,
    threshold: number,
    options?: JobOptions
  ): Promise<PreparedOperation> {
    return this.prepare(
      {
        type: 'CountIf',
        expectedCount: values.length,
        predicate: { op, value: threshold },
      },
      values,
      options
    );
  }

  /**
   * Execute a prepared operation
   */
  async execute(prepared: PreparedOperation): Promise<number> {
    // Upload witness
    await this.uploadWitness(prepared.encryptedData, prepared.witnessCommitment);

    // Build and send transaction
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildCreateJobInstructions(
      prepared.jobId,
      prepared.operation,
      prepared.witnessCommitment,
      prepared.encryptedData.length,
      this.defaultJobOptions
    );

    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (tx) => setTransactionMessageFeePayerSigner(this.signer, tx),
      (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
      (tx) => appendTransactionMessageInstructions(instructions, tx)
    );

    const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);
    const signature = getSignatureFromTransaction(signedTransaction);
    const encodedTransaction = getBase64EncodedWireTransaction(signedTransaction);
    await this.rpc.sendTransaction(encodedTransaction).send();

    // Wait for confirmation
    await this.waitForConfirmation(signature);

    // Poll for result
    return this.pollResult(prepared.jobId);
  }

  // ===========================================================================
  // Batch Operations
  // ===========================================================================

  /**
   * Create a batch builder for parallel operations
   *
   * @example
   * ```typescript
   * const results = await zyber.batch()
   *   .sum([100, 200])
   *   .average([10, 20, 30])
   *   .countIf([18, 25, 16], '>=', 18)
   *   .execute();
   * ```
   */
  batch(): BatchBuilder {
    return new BatchBuilder(this);
  }

  // ===========================================================================
  // Cost Estimation
  // ===========================================================================

  /**
   * Estimate cost for an operation
   */
  estimateCost(options?: JobOptions): CostEstimate {
    const opts = { ...this.defaultJobOptions, ...options };
    const jobCreation = 15_000_000n;
    const txFee = 10_000n;

    return {
      jobCreationLamports: jobCreation,
      proverPaymentLamports: opts.priceLamports!,
      txFeeLamports: txFee,
      totalLamports: jobCreation + opts.priceLamports! + txFee,
    };
  }

  // ===========================================================================
  // Internal Methods
  // ===========================================================================

  private async compute(
    operation: FheOperation,
    values: number[],
    options?: JobOptions
  ): Promise<number> {
    const prepared = await this.prepare(operation, values, options);
    return this.execute(prepared);
  }

  private async prepare(
    operation: FheOperation,
    values: number[],
    options?: JobOptions
  ): Promise<PreparedOperation> {
    const opts = { ...this.defaultJobOptions, ...options };

    // Get next job ID from backend
    const jobId = await this.fetchNextJobId();

    // Encrypt values (via backend)
    const { encryptedData, witnessCommitment } = await this.encryptValues(values);

    // Get PDAs
    const pdas = await this.getPdas(jobId);

    return {
      jobId,
      operation,
      encryptedData,
      witnessCommitment,
      pdas,
      cost: this.estimateCost(opts),
    };
  }

  private async getPdas(jobId: number): Promise<OperationPdas> {
    const addressEncoder = getAddressEncoder();
    const u64Encoder = getU64Encoder();

    const jobIdBytes = u64Encoder.encode(BigInt(jobId));

    const [job] = await getProgramDerivedAddress({
      programAddress: this.config.programId,
      seeds: [
        new TextEncoder().encode('job'),
        addressEncoder.encode(this.signer.address),
        jobIdBytes,
      ],
    });

    const [escrow] = await getProgramDerivedAddress({
      programAddress: this.config.programId,
      seeds: [new TextEncoder().encode('escrow'), addressEncoder.encode(job)],
    });

    const [fheConsensus] = await getProgramDerivedAddress({
      programAddress: this.config.programId,
      seeds: [new TextEncoder().encode('fhe_consensus'), jobIdBytes],
    });

    const [config] = await getProgramDerivedAddress({
      programAddress: this.config.programId,
      seeds: [new TextEncoder().encode('config')],
    });

    return { job, escrow, fheConsensus, config };
  }

  private async fetchNextJobId(): Promise<number> {
    const response = await fetch(`${this.config.backendUrl}/api/next-job-id`);
    if (!response.ok) {
      throw new Error(`Failed to fetch next job ID: ${response.statusText}`);
    }
    const data = await response.json();
    return data.jobId;
  }

  private async encryptValues(
    values: number[]
  ): Promise<{ encryptedData: string; witnessCommitment: Uint8Array }> {
    const response = await fetch(`${this.config.backendUrl}/api/encrypt`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ values }),
    });

    if (!response.ok) {
      throw new Error(`Failed to encrypt values: ${response.statusText}`);
    }

    const data = await response.json();
    return {
      encryptedData: data.encryptedData,
      witnessCommitment: new Uint8Array(Buffer.from(data.witnessCommitment, 'hex')),
    };
  }

  private async uploadWitness(encryptedData: string, commitment: Uint8Array): Promise<void> {
    const response = await fetch(`${this.config.backendUrl}/api/witness`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        data: encryptedData,
        commitment: Buffer.from(commitment).toString('hex'),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to upload witness: ${response.statusText}`);
    }
  }

  private async buildCreateJobInstructions(
    jobId: number,
    operation: FheOperation,
    witnessCommitment: Uint8Array,
    witnessSize: number,
    options: JobOptions
  ): Promise<any[]> {
    // For now, use backend to build instructions
    const response = await fetch(`${this.config.backendUrl}/api/build-instructions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        creator: this.signer.address,
        jobId,
        operation,
        witnessCommitment: Buffer.from(witnessCommitment).toString('hex'),
        witnessSize,
        priceLamports: options.priceLamports!.toString(),
        timeoutSeconds: options.timeoutSeconds,
        requiredProvers: options.requiredProvers,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build instructions: ${response.statusText}`);
    }

    const data = await response.json();
    return data.instructions;
  }

  private async waitForConfirmation(signature: string, timeoutMs: number = 30000): Promise<void> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const { value } = await this.rpc.getSignatureStatuses([signature as any]).send();
      const status = value[0];

      if (status?.confirmationStatus === 'confirmed' || status?.confirmationStatus === 'finalized') {
        if (status.err) {
          throw new Error(`Transaction failed: ${JSON.stringify(status.err)}`);
        }
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, 500));
    }

    throw new Error(`Transaction confirmation timed out after ${timeoutMs}ms`);
  }

  private async pollResult(jobId: number, timeoutMs: number = 120000): Promise<number> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const response = await fetch(`${this.config.backendUrl}/api/jobs/${jobId}/result`);

      if (response.ok) {
        const data = (await response.json()) as { status: string; result?: number; error?: string };

        if (data.status === 'completed') {
          return data.result!;
        } else if (data.status === 'failed') {
          throw new Error(`Job failed: ${data.error}`);
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 2000));
    }

    throw new Error(`Job ${jobId} timed out after ${timeoutMs}ms`);
  }
}

// =============================================================================
// BatchBuilder
// =============================================================================

interface BatchOp {
  type: 'sum' | 'average' | 'countIf';
  values: number[];
  op?: PredicateOp;
  threshold?: number;
}

export class BatchBuilder {
  private client: Zyber;
  private operations: BatchOp[] = [];

  constructor(client: Zyber) {
    this.client = client;
  }

  sum(values: number[]): this {
    this.operations.push({ type: 'sum', values });
    return this;
  }

  average(values: number[]): this {
    this.operations.push({ type: 'average', values });
    return this;
  }

  countIf(values: number[], op: PredicateOp, threshold: number): this {
    this.operations.push({ type: 'countIf', values, op, threshold });
    return this;
  }

  /**
   * Execute all operations in parallel
   */
  async execute(): Promise<number[]> {
    const promises = this.operations.map(async (op) => {
      switch (op.type) {
        case 'sum':
          return this.client.sum(op.values);
        case 'average':
          return this.client.average(op.values);
        case 'countIf':
          return this.client.countIf(op.values, op.op!, op.threshold!);
      }
    });

    return Promise.all(promises);
  }

  /** Get number of pending operations */
  get length(): number {
    return this.operations.length;
  }
}
