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
  type IInstruction,
} from '@solana/kit';

import {
  NetworkConfig,
  NetworkName,
  NETWORKS,
  ZkCircuitType,
  ZkJob,
  ZkJobOptions,
  DisputeProofParams,
  DisputeResult,
  DISPUTE_BOND_LAMPORTS,
} from './types';

// =============================================================================
// ZK Generator Constants
// =============================================================================

const ZK_GENERATOR_PROGRAM_ID = '29dv8e1WcjL4w6a7HDaHbUfXrF12yiJiVcKQ1qgeT3rF' as Address;

const JOB_SEED = new TextEncoder().encode('job');
const ESCROW_SEED = new TextEncoder().encode('escrow');
const PROVER_STAKE_SEED = new TextEncoder().encode('prover_stake');
const DISPUTE_BOND_SEED = new TextEncoder().encode('dispute_bond');

/** Minimum stake to be a prover (0.5 SOL) */
export const MIN_PROVER_STAKE = 500_000_000n;

// =============================================================================
// ZkClient - ZK Generator SDK
// =============================================================================

export interface ZkClientOptions {
  rpcUrl?: string;
  wsUrl?: string;
  backendUrl?: string;
  zkProgramId?: Address;
}

export interface ProverStakeInfo {
  prover: Address;
  stakedAmount: bigint;
  lockedAmount: bigint;
  totalSlashed: bigint;
  jobsCompleted: bigint;
  timesSlashed: bigint;
  reputationScore: number;
}

export interface QuoteResponse {
  circuit_type: number;
  price_lamports: number;
  price_sol: number;
  expires_at: number;
  quote_id: string;
  payment_recipient: string;
}

export interface CreateZkJobParams {
  circuitType: ZkCircuitType;
  witnessData: Uint8Array;
  paymentToken?: string;
  options?: ZkJobOptions;
}

export interface SubmitProofParams {
  jobAddress: Address;
  proofHash: Uint8Array;
}

/**
 * ZkClient - Client for ZK Generator program
 *
 * Handles ZK proof generation jobs, prover staking, and disputes.
 *
 * @example
 * ```typescript
 * import { ZkClient, ZkCircuitType } from '@zyberlink/sdk';
 *
 * const zk = await ZkClient.connect('devnet', wallet);
 *
 * // Create a private vote job
 * const job = await zk.createJob({
 *   circuitType: ZkCircuitType.PrivateVote,
 *   witnessData: voteWitness,
 * });
 *
 * // Wait for proof
 * const result = await zk.waitForProof(job.id);
 * ```
 */
export class ZkClient {
  private config: NetworkConfig;
  private rpc: ReturnType<typeof createSolanaRpc>;
  private rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions>;
  private signer: TransactionSigner;
  private zkProgramId: Address;
  private defaultJobOptions: ZkJobOptions;

  private constructor(
    config: NetworkConfig,
    rpc: ReturnType<typeof createSolanaRpc>,
    rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions>,
    signer: TransactionSigner,
    zkProgramId: Address,
    defaultJobOptions: ZkJobOptions
  ) {
    this.config = config;
    this.rpc = rpc;
    this.rpcSubscriptions = rpcSubscriptions;
    this.signer = signer;
    this.zkProgramId = zkProgramId;
    this.defaultJobOptions = defaultJobOptions;
  }

  /**
   * Connect to ZK Generator
   */
  static async connect(
    network: NetworkName,
    signer: TransactionSigner,
    options?: ZkClientOptions
  ): Promise<ZkClient> {
    const config = { ...NETWORKS[network] };

    if (options?.rpcUrl) config.rpcUrl = options.rpcUrl;
    if (options?.wsUrl) config.wsUrl = options.wsUrl;
    if (options?.backendUrl) config.backendUrl = options.backendUrl;

    const rpc = createSolanaRpc(config.rpcUrl);
    const rpcSubscriptions = createSolanaRpcSubscriptions(config.wsUrl);

    const zkProgramId = options?.zkProgramId ?? ZK_GENERATOR_PROGRAM_ID;

    const defaultJobOptions: ZkJobOptions = {
      priceLamports: 50_000_000n, // 0.05 SOL default
      timeoutSeconds: 300, // 5 minutes
    };

    return new ZkClient(config, rpc, rpcSubscriptions, signer, zkProgramId, defaultJobOptions);
  }

  // ===========================================================================
  // Properties
  // ===========================================================================

  get address(): Address {
    return this.signer.address;
  }

  get network(): NetworkName {
    return this.config.name;
  }

  // ===========================================================================
  // Quote & Pricing
  // ===========================================================================

  /**
   * Get a price quote for a ZK circuit
   *
   * This should be called before creating a job to get the current price
   * and payment recipient address for Solana payment.
   *
   * @example
   * ```typescript
   * const quote = await zk.getQuote(ZkCircuitType.ProofOfInnocence);
   * console.log('Price:', quote.price_sol, 'SOL');
   * console.log('Payment recipient:', quote.payment_recipient);
   * ```
   */
  async getQuote(circuitType: ZkCircuitType): Promise<QuoteResponse> {
    const response = await fetch(`${this.config.backendUrl}/api/quote`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        circuit_type: circuitType,
        payer: this.signer.address,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to get quote: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Confirm payment and get payment token
   *
   * After paying on Solana, this method confirms the payment and returns
   * a token that can be used to create a job.
   *
   * @example
   * ```typescript
   * const quote = await zk.getQuote(ZkCircuitType.ProofOfInnocence);
   * const txSig = await payOnSolana(quote.payment_recipient, quote.price_lamports);
   * const { token_id } = await zk.confirmPayment(quote.quote_id, txSig);
   * ```
   */
  async confirmPayment(quoteId: string, signature: string): Promise<{ token_id: string }> {
    const response = await fetch(`${this.config.backendUrl}/api/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        quote_id: quoteId,
        tx_signature: signature,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to confirm payment: ${response.statusText}`);
    }

    return response.json();
  }

  // ===========================================================================
  // Job Operations
  // ===========================================================================

  /**
   * Create a new ZK proof job
   *
   * @example
   * ```typescript
   * // 1. Get quote
   * const quote = await zk.getQuote(ZkCircuitType.PrivateVote);
   *
   * // 2. Pay on Solana (user handles this)
   * const txSignature = await sendPayment(quote.payment_recipient, quote.price_lamports);
   *
   * // 3. Create job with payment proof
   * const job = await zk.createJob({
   *   circuitType: ZkCircuitType.PrivateVote,
   *   witnessData: witnessBytes,
   *   paymentToken: txSignature,
   * });
   * ```
   */
  async createJob(params: CreateZkJobParams): Promise<ZkJob> {
    const opts = { ...this.defaultJobOptions, ...params.options };

    // Hash witness data
    const witnessHash = await this.hashData(params.witnessData);

    // Upload witness to backend
    await this.uploadWitness(witnessHash, params.witnessData);

    // Build transaction
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildCreateJobInstruction(
      params.circuitType,
      witnessHash,
      params.witnessData.length,
      opts.priceLamports!,
      opts.timeoutSeconds!,
      params.paymentToken
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
    await this.waitForConfirmation(signature);

    // Get job from backend
    const jobId = await this.getJobIdFromSignature(signature);
    return this.getJob(jobId);
  }

  /**
   * Claim a pending job (for provers)
   */
  async claimJob(jobAddress: Address): Promise<void> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildClaimJobInstruction(jobAddress);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Submit proof for a claimed job (for provers)
   */
  async submitProof(params: SubmitProofParams): Promise<void> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildSubmitProofInstruction(params.jobAddress, params.proofHash);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Cancel a pending job (creator only)
   */
  async cancelJob(jobAddress: Address): Promise<void> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildCancelJobInstruction(jobAddress);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Get job details
   */
  async getJob(jobId: number): Promise<ZkJob> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/${jobId}`);
    if (!response.ok) {
      throw new Error(`Failed to get job: ${response.statusText}`);
    }
    return response.json();
  }

  /**
   * Wait for job proof to be submitted
   */
  async waitForProof(jobId: number, timeoutMs = 300_000): Promise<ZkJob> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const job = await this.getJob(jobId);

      if (job.status === 'completed') {
        return job;
      }
      if (job.status === 'failed' || job.status === 'cancelled') {
        throw new Error(`Job ${jobId} ended with status: ${job.status}`);
      }

      await this.sleep(3000);
    }

    throw new Error(`Job ${jobId} timed out after ${timeoutMs}ms`);
  }

  // ===========================================================================
  // Dispute Operations
  // ===========================================================================

  /**
   * Dispute a submitted proof
   *
   * Requires DISPUTE_BOND_LAMPORTS (0.1 SOL) bond.
   * If proof is invalid, disputor receives 50% of prover's slashed stake.
   * If proof is valid, bond is forfeited.
   *
   * @example
   * ```typescript
   * const result = await zk.disputeProof({
   *   jobAddress: jobPda,
   *   proof: fullProofBytes,
   *   publicInputs: publicInputBytes,
   * });
   *
   * if (result.proofInvalid) {
   *   console.log('Proof was invalid! Reward:', result.disputorReward);
   * }
   * ```
   */
  async disputeProof(params: DisputeProofParams): Promise<DisputeResult> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildDisputeProofInstruction(
      params.jobAddress,
      params.proof,
      params.publicInputs
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
    await this.waitForConfirmation(signature);

    // Parse result from transaction logs
    return this.getDisputeResultFromSignature(signature);
  }

  // ===========================================================================
  // Prover Staking Operations
  // ===========================================================================

  /**
   * Register as a prover with initial stake
   *
   * @example
   * ```typescript
   * await zk.registerProver(1_000_000_000n); // 1 SOL stake
   * ```
   */
  async registerProver(stakeAmount: bigint): Promise<void> {
    if (stakeAmount < MIN_PROVER_STAKE) {
      throw new Error(`Minimum stake is ${MIN_PROVER_STAKE} lamports (0.5 SOL)`);
    }

    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildRegisterProverInstruction(stakeAmount);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Deposit additional stake
   */
  async depositStake(amount: bigint): Promise<void> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildDepositStakeInstruction(amount);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Withdraw available stake (not locked in jobs)
   */
  async withdrawStake(amount: bigint): Promise<void> {
    const { value: latestBlockhash } = await this.rpc.getLatestBlockhash().send();

    const instructions = await this.buildWithdrawStakeInstruction(amount);

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
    await this.waitForConfirmation(signature);
  }

  /**
   * Get prover stake info
   */
  async getProverStake(prover?: Address): Promise<ProverStakeInfo | null> {
    const proverAddress = prover ?? this.signer.address;
    const stakePda = await this.getProverStakePda(proverAddress);

    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/provers/${stakePda}`);
    if (response.status === 404) {
      return null;
    }
    if (!response.ok) {
      throw new Error(`Failed to get prover stake: ${response.statusText}`);
    }
    return response.json();
  }

  // ===========================================================================
  // High-Level Operations
  // ===========================================================================

  /**
   * Create a private vote with ZK proof
   *
   * @example
   * ```typescript
   * const vote = await zk.privateVote({
   *   voteId: proposalId,
   *   choice: 1, // Yes
   *   voterNullifier: myNullifier,
   * });
   * ```
   */
  async privateVote(params: {
    voteId: Uint8Array;
    choice: number;
    voterNullifier: Uint8Array;
    withPoI?: boolean;
    merkleProof?: Uint8Array;
  }): Promise<ZkJob> {
    const circuitType = params.withPoI
      ? ZkCircuitType.PrivateVoteWithPoI
      : ZkCircuitType.PrivateVote;

    // Build witness data
    const witnessData = this.encodeVoteWitness(
      params.voteId,
      params.choice,
      params.voterNullifier,
      params.merkleProof
    );

    return this.createJob({ circuitType, witnessData });
  }

  /**
   * Create a market bet with ZK proof
   *
   * @example
   * ```typescript
   * const bet = await zk.marketBet({
   *   marketId: market.address,
   *   outcome: 0,
   *   amount: 1_000_000n,
   *   betterNullifier: myNullifier,
   * });
   * ```
   */
  async marketBet(params: {
    marketId: Uint8Array;
    outcome: number;
    amount: bigint;
    betterNullifier: Uint8Array;
    withPoI?: boolean;
    merkleProof?: Uint8Array;
  }): Promise<ZkJob> {
    const circuitType = params.withPoI
      ? ZkCircuitType.MarketBetWithPoI
      : ZkCircuitType.MarketBet;

    const witnessData = this.encodeMarketBetWitness(
      params.marketId,
      params.outcome,
      params.amount,
      params.betterNullifier,
      params.merkleProof
    );

    return this.createJob({ circuitType, witnessData });
  }

  /**
   * Proof of Innocence - verify wallet is not on sanctions list
   *
   * @example
   * ```typescript
   * const poi = await zk.proofOfInnocence({
   *   walletHash: myWalletHash,
   *   sanctionsRoot: currentRoot,
   *   merkleProof: proofPath,
   * });
   * ```
   */
  async proofOfInnocence(params: {
    walletHash: Uint8Array;
    sanctionsRoot: Uint8Array;
    merkleProof: Uint8Array;
  }): Promise<ZkJob> {
    const witnessData = this.encodePoIWitness(
      params.walletHash,
      params.sanctionsRoot,
      params.merkleProof
    );

    return this.createJob({
      circuitType: ZkCircuitType.ProofOfInnocence,
      witnessData,
    });
  }

  /**
   * Portfolio compliance check
   */
  async portfolioCompliance(params: {
    portfolioHash: Uint8Array;
    balances: bigint[];
    thresholds: { min?: bigint; max?: bigint }[];
    complianceRule: Uint8Array;
  }): Promise<ZkJob> {
    const witnessData = this.encodePortfolioWitness(
      params.portfolioHash,
      params.balances,
      params.thresholds,
      params.complianceRule
    );

    return this.createJob({
      circuitType: ZkCircuitType.PortfolioCompliance,
      witnessData,
    });
  }

  // ===========================================================================
  // PDA Helpers
  // ===========================================================================

  private async getJobPda(creator: Address, jobId: bigint): Promise<Address> {
    const addressEncoder = getAddressEncoder();
    const u64Encoder = getU64Encoder();

    const [pda] = await getProgramDerivedAddress({
      programAddress: this.zkProgramId,
      seeds: [JOB_SEED, addressEncoder.encode(creator), u64Encoder.encode(jobId)],
    });

    return pda;
  }

  private async getEscrowPda(jobPda: Address): Promise<Address> {
    const addressEncoder = getAddressEncoder();

    const [pda] = await getProgramDerivedAddress({
      programAddress: this.zkProgramId,
      seeds: [ESCROW_SEED, addressEncoder.encode(jobPda)],
    });

    return pda;
  }

  private async getProverStakePda(prover: Address): Promise<Address> {
    const addressEncoder = getAddressEncoder();

    const [pda] = await getProgramDerivedAddress({
      programAddress: this.zkProgramId,
      seeds: [PROVER_STAKE_SEED, addressEncoder.encode(prover)],
    });

    return pda;
  }

  private async getDisputeBondPda(jobPda: Address, disputor: Address): Promise<Address> {
    const addressEncoder = getAddressEncoder();

    const [pda] = await getProgramDerivedAddress({
      programAddress: this.zkProgramId,
      seeds: [DISPUTE_BOND_SEED, addressEncoder.encode(jobPda), addressEncoder.encode(disputor)],
    });

    return pda;
  }

  // ===========================================================================
  // Instruction Builders
  // ===========================================================================

  private async buildCreateJobInstruction(
    circuitType: ZkCircuitType,
    witnessHash: Uint8Array,
    witnessSize: number,
    priceLamports: bigint,
    timeoutSeconds: number,
    paymentToken?: string
  ): Promise<IInstruction[]> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    let endpoint: string;

    if (paymentToken) {
      headers['X-Payment-Token'] = paymentToken;
      endpoint = `${this.config.backendUrl}/gateway/zk/create`;
    } else {
      endpoint = `${this.config.backendUrl}/api/jobs/zk/validate-and-build`;
    }

    const response = await fetch(endpoint, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        creator: this.signer.address,
        circuit_type: circuitType,
        witness_commitment: Buffer.from(witnessHash).toString('hex'),
        public_inputs: [], // TODO: Extract from witness data
        timeout_seconds: timeoutSeconds,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build create job instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildClaimJobInstruction(jobAddress: Address): Promise<IInstruction[]> {
    // TODO: Backend endpoint /internal/zk/build-claim-job doesn't exist yet
    // This needs to be implemented in the backend or use a different approach
    throw new Error('buildClaimJobInstruction not implemented - backend endpoint missing');
  }

  private async buildSubmitProofInstruction(
    jobAddress: Address,
    proofHash: Uint8Array
  ): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-submit-proof`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prover: this.signer.address,
        jobAddress,
        proofHash: Buffer.from(proofHash).toString('hex'),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build submit proof instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildCancelJobInstruction(jobAddress: Address): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-cancel-job`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        creator: this.signer.address,
        jobAddress,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build cancel job instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildDisputeProofInstruction(
    jobAddress: Address,
    proof: Uint8Array,
    publicInputs: Uint8Array
  ): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-dispute-proof`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        disputor: this.signer.address,
        jobAddress,
        proof: Buffer.from(proof).toString('hex'),
        publicInputs: Buffer.from(publicInputs).toString('hex'),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build dispute proof instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildRegisterProverInstruction(stakeAmount: bigint): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-register-prover`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prover: this.signer.address,
        stakeAmount: stakeAmount.toString(),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build register prover instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildDepositStakeInstruction(amount: bigint): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-deposit-stake`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prover: this.signer.address,
        amount: amount.toString(),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build deposit stake instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  private async buildWithdrawStakeInstruction(amount: bigint): Promise<IInstruction[]> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/build-withdraw-stake`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prover: this.signer.address,
        amount: amount.toString(),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build withdraw stake instruction: ${response.statusText}`);
    }

    return (await response.json()).instructions;
  }

  // ===========================================================================
  // Witness Encoding
  // ===========================================================================

  private encodeVoteWitness(
    voteId: Uint8Array,
    choice: number,
    nullifier: Uint8Array,
    merkleProof?: Uint8Array
  ): Uint8Array {
    const parts: Uint8Array[] = [
      voteId,
      new Uint8Array([choice]),
      nullifier,
    ];

    if (merkleProof) {
      parts.push(merkleProof);
    }

    return this.concatBytes(parts);
  }

  private encodeMarketBetWitness(
    marketId: Uint8Array,
    outcome: number,
    amount: bigint,
    nullifier: Uint8Array,
    merkleProof?: Uint8Array
  ): Uint8Array {
    const amountBytes = new Uint8Array(8);
    new DataView(amountBytes.buffer).setBigUint64(0, amount, true);

    const parts: Uint8Array[] = [
      marketId,
      new Uint8Array([outcome]),
      amountBytes,
      nullifier,
    ];

    if (merkleProof) {
      parts.push(merkleProof);
    }

    return this.concatBytes(parts);
  }

  private encodePoIWitness(
    walletHash: Uint8Array,
    sanctionsRoot: Uint8Array,
    merkleProof: Uint8Array
  ): Uint8Array {
    return this.concatBytes([walletHash, sanctionsRoot, merkleProof]);
  }

  private encodePortfolioWitness(
    portfolioHash: Uint8Array,
    balances: bigint[],
    thresholds: { min?: bigint; max?: bigint }[],
    complianceRule: Uint8Array
  ): Uint8Array {
    const balanceBytes = new Uint8Array(balances.length * 8);
    const view = new DataView(balanceBytes.buffer);
    balances.forEach((b, i) => view.setBigUint64(i * 8, b, true));

    const thresholdBytes = new Uint8Array(thresholds.length * 16);
    const thresholdView = new DataView(thresholdBytes.buffer);
    thresholds.forEach((t, i) => {
      thresholdView.setBigUint64(i * 16, t.min ?? 0n, true);
      thresholdView.setBigUint64(i * 16 + 8, t.max ?? BigInt('0xFFFFFFFFFFFFFFFF'), true);
    });

    return this.concatBytes([
      portfolioHash,
      new Uint8Array([balances.length]),
      balanceBytes,
      thresholdBytes,
      complianceRule,
    ]);
  }

  // ===========================================================================
  // Utility Methods
  // ===========================================================================

  private async hashData(data: Uint8Array): Promise<Uint8Array> {
    const hashBuffer = await crypto.subtle.digest('SHA-256', data.buffer as ArrayBuffer);
    return new Uint8Array(hashBuffer);
  }

  private async uploadWitness(witnessHash: Uint8Array, witnessData: Uint8Array): Promise<void> {
    const response = await fetch(`${this.config.backendUrl}/api/jobs/zk/witness`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        hash: Buffer.from(witnessHash).toString('hex'),
        data: Buffer.from(witnessData).toString('base64'),
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to upload witness: ${response.statusText}`);
    }
  }

  private async waitForConfirmation(signature: string, timeoutMs = 30000): Promise<void> {
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

      await this.sleep(500);
    }

    throw new Error(`Transaction confirmation timed out after ${timeoutMs}ms`);
  }

  private async getJobIdFromSignature(signature: string): Promise<number> {
    const response = await fetch(
      `${this.config.backendUrl}/api/jobs/zk/job-id-from-signature/${signature}`
    );
    if (!response.ok) {
      throw new Error(`Failed to get job ID: ${response.statusText}`);
    }
    const data = await response.json();
    return data.jobId;
  }

  private async getDisputeResultFromSignature(signature: string): Promise<DisputeResult> {
    const response = await fetch(
      `${this.config.backendUrl}/api/jobs/zk/dispute-result-from-signature/${signature}`
    );
    if (!response.ok) {
      throw new Error(`Failed to get dispute result: ${response.statusText}`);
    }
    return response.json();
  }

  private concatBytes(arrays: Uint8Array[]): Uint8Array {
    const totalLength = arrays.reduce((acc, arr) => acc + arr.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of arrays) {
      result.set(arr, offset);
      offset += arr.length;
    }
    return result;
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
