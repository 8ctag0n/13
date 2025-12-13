import { type TransactionSigner } from '@solana/kit';
import { NetworkConfig, NetworkName, NETWORKS } from './types';

export interface ProverClientOptions {
  gatewayUrl?: string;
}

export interface FheJobSubmitResult {
  commitment: string;
}

/**
 * ProverClient - Client for provers to interact with the gateway
 *
 * Handles authenticated requests to the prover gateway endpoints.
 *
 * @example
 * ```typescript
 * import { ProverClient } from '@zyberlink/sdk';
 *
 * const prover = await ProverClient.connect('devnet', proverKeypair);
 *
 * // Get witness data for a job
 * const witness = await prover.getWitness(witnessHash);
 *
 * // Submit ZK proof
 * await prover.submitZkProof(jobId, proof, publicInputs);
 *
 * // Submit FHE result
 * const result = await prover.submitFheResult(jobId, encryptedResult);
 * ```
 */
export class ProverClient {
  private config: NetworkConfig;
  private signer: TransactionSigner;

  private constructor(config: NetworkConfig, signer: TransactionSigner) {
    this.config = config;
    this.signer = signer;
  }

  /**
   * Connect to gateway as a prover
   */
  static async connect(
    network: NetworkName,
    signer: TransactionSigner,
    options?: ProverClientOptions
  ): Promise<ProverClient> {
    const config = { ...NETWORKS[network] };

    if (options?.gatewayUrl) {
      config.backendUrl = options.gatewayUrl;
    }

    return new ProverClient(config, signer);
  }

  // ===========================================================================
  // Properties
  // ===========================================================================

  get address(): string {
    return this.signer.address;
  }

  get network(): NetworkName {
    return this.config.name;
  }

  // ===========================================================================
  // Witness Operations
  // ===========================================================================

  /**
   * Get witness data for a job
   *
   * @param hash - Witness hash (hex string)
   * @returns Witness data as Uint8Array
   *
   * @example
   * ```typescript
   * const witness = await prover.getWitness(witnessHash);
   * ```
   */
  async getWitness(hash: string): Promise<Uint8Array> {
    const headers = await this.signRequest('GET', `/gateway/prover/witness/${hash}`);

    const response = await fetch(`${this.config.backendUrl}/gateway/prover/witness/${hash}`, {
      method: 'GET',
      headers,
    });

    if (!response.ok) {
      throw new Error(`Failed to get witness: ${response.statusText}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    return new Uint8Array(arrayBuffer);
  }

  // ===========================================================================
  // ZK Proof Submission
  // ===========================================================================

  /**
   * Submit ZK proof for a job
   *
   * @param jobId - Job ID
   * @param proof - Proof string (hex)
   * @param publicInputs - Public inputs array
   *
   * @example
   * ```typescript
   * await prover.submitZkProof(123, proofHex, [input1, input2]);
   * ```
   */
  async submitZkProof(jobId: number, proof: string, publicInputs: string[]): Promise<void> {
    const path = `/gateway/prover/zk/${jobId}/submit`;
    const body = JSON.stringify({
      proof,
      public_inputs: publicInputs,
    });

    const headers = await this.signRequest('POST', path, Buffer.from(body));

    const response = await fetch(`${this.config.backendUrl}${path}`, {
      method: 'POST',
      headers,
      body,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to submit ZK proof: ${response.statusText} - ${error}`);
    }
  }

  // ===========================================================================
  // FHE Result Submission
  // ===========================================================================

  /**
   * Submit FHE computation result
   *
   * @param jobId - Job ID
   * @param result - Encrypted result as Uint8Array
   * @returns Commitment to the result
   *
   * @example
   * ```typescript
   * const { commitment } = await prover.submitFheResult(456, encryptedResult);
   * console.log('Result commitment:', commitment);
   * ```
   */
  async submitFheResult(jobId: number, result: Uint8Array): Promise<FheJobSubmitResult> {
    const path = `/gateway/prover/fhe/${jobId}/submit`;

    // Create a copy to ensure proper ArrayBuffer type
    const resultCopy = new Uint8Array(result);
    const headers = await this.signRequest('POST', path, resultCopy);

    const response = await fetch(`${this.config.backendUrl}${path}`, {
      method: 'POST',
      headers,
      body: resultCopy.buffer as ArrayBuffer,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to submit FHE result: ${response.statusText} - ${error}`);
    }

    return response.json();
  }

  // ===========================================================================
  // Authentication
  // ===========================================================================

  /**
   * Sign a request for authentication
   *
   * Generates authentication headers for the gateway.
   *
   * @param method - HTTP method
   * @param path - Request path
   * @param body - Request body (optional)
   */
  private async signRequest(
    method: string,
    path: string,
    body?: Uint8Array
  ): Promise<Record<string, string>> {
    const timestamp = Math.floor(Date.now() / 1000);

    let bodyHash = '';
    if (body) {
      const hashBuffer = await crypto.subtle.digest('SHA-256', body.buffer as ArrayBuffer);
      bodyHash = Buffer.from(hashBuffer).toString('hex');
    }

    const message = `${method}:${path}:${timestamp}:${bodyHash}`;
    const messageBytes = new TextEncoder().encode(message);

    // Sign message using TransactionSigner
    // The signer should implement signMessages for arbitrary message signing
    if ('signMessages' in this.signer && typeof this.signer.signMessages === 'function') {
      const signatures = await this.signer.signMessages([messageBytes]);
      const signature = signatures[0];

      return {
        'Content-Type': 'application/json',
        'X-Prover-Pubkey': this.signer.address,
        'X-Prover-Signature': Buffer.from(signature).toString('base64'),
        'X-Timestamp': timestamp.toString(),
      };
    }

    throw new Error(
      'ProverClient requires a signer that implements signMessages() for authentication. ' +
        'Please use a Keypair or compatible signer.'
    );
  }
}
