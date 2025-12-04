import { Transaction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { createSolanaRpc } from '@solana/kit';
import { uploadServerKey } from './server_key_upload.js';

/**
 * Create an FHE job from a parsed witness.bin file
 *
 * @param {Object} options - Job configuration
 * @param {string} options.operation - FHE operation (sum, average, count_if, add, multiply, subtract)
 * @param {number} options.operationValue - Value for the operation (e.g., multiplier, threshold)
 * @param {Uint8Array} options.serverKeyBytes - Raw server key bytes from witness.bin (preferred)
 * @param {string} [options.serverKey] - Base64-encoded server key (legacy, deprecated)
 * @param {string} options.encryptedData - Base64-encoded encrypted data from witness.bin
 * @param {Object} options.wallet - Wallet store object with publicKey, signMessage, signTransaction
 * @param {string} options.apiBaseUrl - API base URL
 * @param {string} options.rpcUrl - Solana RPC URL
 * @param {number} [options.priceLamports=5000000] - Price in lamports
 * @param {number} [options.requiredProvers=3] - Number of required provers
 * @param {number} [options.consensusThreshold=2] - Consensus threshold
 * @param {string} [options.paymentMethod='SOL'] - Payment method (SOL or WZEC)
 * @param {Object} [options.predicate] - Predicate for count_if operation
 * @param {Function} [options.onProgress] - Progress callback
 * @returns {Promise<{jobId: string, signature: string}>}
 */
export async function createFheJobFromWitness(options) {
  const {
    operation,
    operationValue = 0,
    serverKeyBytes,  // New: raw bytes for pre-upload
    serverKey,       // Legacy: base64 string
    encryptedData,
    wallet,
    apiBaseUrl,
    rpcUrl = 'http://localhost:8899',
    priceLamports = 5000000,
    requiredProvers = 3,
    consensusThreshold = 2,
    paymentMethod = 'SOL',
    predicate = null,
    onProgress = () => {}
  } = options;

  if (!wallet || !wallet.publicKey) {
    throw new Error('Wallet not connected');
  }

  // Validate we have either serverKeyBytes (new) or serverKey (legacy)
  if (!serverKeyBytes && !serverKey) {
    throw new Error('Missing witness data: serverKeyBytes or serverKey required');
  }

  if (!encryptedData) {
    throw new Error('Missing encrypted data');
  }

  try {
    // Step 0: Pre-upload server key if we have raw bytes
    let serverKeyHash = null;

    if (serverKeyBytes && serverKeyBytes.length > 0) {
      onProgress({ step: 'uploading_key', message: 'Uploading server key...' });

      try {
        const uploadResult = await uploadServerKey(
          serverKeyBytes,
          apiBaseUrl,
          (progress) => {
            // Build detailed message with speed and progress
            let message = `Uploading: ${progress.loadedMB}/${progress.totalMB} MB`;
            if (progress.speedKBps) {
              message += ` @ ${progress.speedKBps} KB/s`;
            }
            if (progress.etaFormatted && progress.etaFormatted !== '--:--') {
              message += ` (ETA: ${progress.etaFormatted})`;
            }

            onProgress({
              step: 'uploading_key',
              message,
              progress: progress.percent,
              // Pass detailed upload info
              uploadDetails: {
                loadedMB: progress.loadedMB,
                totalMB: progress.totalMB,
                speedKBps: progress.speedKBps,
                speedMBps: progress.speedMBps,
                etaSeconds: progress.etaSeconds,
                etaFormatted: progress.etaFormatted
              }
            });
          }
        );
        serverKeyHash = uploadResult.server_key_hash;
        console.log('Server key uploaded, hash:', serverKeyHash);
      } catch (uploadError) {
        console.error('Server key pre-upload failed:', uploadError);
        throw new Error(`Server key upload failed: ${uploadError.message}`);
      }
    }

    // Step 1: Generate signature
    onProgress({ step: 'signing', message: 'Generating signature...' });

    const timestamp = Math.floor(Date.now() / 1000);
    const nonce = Math.random().toString(36).substring(2, 15);
    const jobId = timestamp * 1000 + Math.floor(Math.random() * 1000);
    const message = `create_job:${jobId}:${timestamp}:${nonce}`;

    const messageBytes = new TextEncoder().encode(message);

    // Use provider.signMessage for Solflare/Phantom compatibility
    const provider = wallet.provider;
    if (!provider || !provider.signMessage) {
      throw new Error('Wallet provider does not support signMessage');
    }

    const signResult = await provider.signMessage(messageBytes, 'utf8');
    // Handle both Uint8Array and {signature: Uint8Array} response formats
    const signatureBytes = signResult.signature || signResult;
    const signatureBase64 = btoa(String.fromCharCode(...new Uint8Array(signatureBytes)));

    // Step 2: Call validate-and-build endpoint
    onProgress({ step: 'validating', message: 'Validating job with backend...' });

    // Get public key as string - handle both object and string formats
    const creatorPubkey = wallet.addresses?.solana || wallet.publicKey?.toString() || wallet.publicKey;

    const requestBody = {
      creator_pubkey: creatorPubkey,
      encrypted_data: encryptedData,
      message: message,
      signature: signatureBase64,
      nonce: nonce,
      operation: operation.toLowerCase(),
      operation_value: operationValue,
      price_lamports: priceLamports,
      required_provers: requiredProvers,
      consensus_threshold: consensusThreshold,
      payment_method: paymentMethod.toUpperCase()
    };

    // Use server_key_hash if available (pre-uploaded), otherwise fall back to legacy base64
    if (serverKeyHash) {
      requestBody.server_key_hash = serverKeyHash;
      console.log('Using pre-uploaded server key hash:', serverKeyHash);
    } else if (serverKey) {
      requestBody.server_key = serverKey;
      console.log('Using legacy base64 server key');
    }

    // Add predicate for count_if operation
    if (predicate && operation.toLowerCase() === 'count_if') {
      requestBody.predicate = predicate;
    }

    const validateResponse = await fetch(`${apiBaseUrl}/api/jobs/validate-and-build`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requestBody),
    });

    if (!validateResponse.ok) {
      const errorData = await validateResponse.json();
      throw new Error(errorData.error || 'Failed to validate job');
    }

    const { job_id, transaction } = await validateResponse.json();
    console.log('Job validated with ID:', job_id);

    // Step 3: Deserialize unsigned transaction
    onProgress({ step: 'preparing', message: 'Preparing transaction...' });

    const txBytes = Uint8Array.from(atob(transaction), c => c.charCodeAt(0));
    const tx = Transaction.from(txBytes);

    // Step 4: Get recent blockhash and set fee payer
    const rpc = createSolanaRpc(rpcUrl);
    const { value: latestBlockhash } = await rpc.getLatestBlockhash({ commitment: 'confirmed' }).send();
    tx.recentBlockhash = latestBlockhash.blockhash;
    tx.feePayer = wallet.publicKey;

    // Step 5: Sign transaction with wallet
    onProgress({ step: 'wallet_sign', message: 'Waiting for wallet signature...' });

    const signedTx = await wallet.signTransaction(tx);

    // Step 6: Send transaction to Solana network
    onProgress({ step: 'sending', message: 'Sending transaction to Solana...' });

    const txBase64 = btoa(String.fromCharCode(...signedTx.serialize()));
    const sendResult = await rpc.sendTransaction(txBase64, {
      encoding: 'base64',
      skipPreflight: false,
      preflightCommitment: 'confirmed'
    }).send();
    const txSignature = sendResult;

    // Step 7: Wait for confirmation
    onProgress({ step: 'confirming', message: 'Confirming transaction...' });

    let confirmed = false;
    for (let i = 0; i < 30 && !confirmed; i++) {
      await new Promise(r => setTimeout(r, 1000));
      const statusResult = await rpc.getSignatureStatuses([txSignature]).send();
      if (statusResult.value[0]?.confirmationStatus === 'confirmed' ||
          statusResult.value[0]?.confirmationStatus === 'finalized') {
        confirmed = true;
      }
    }

    if (!confirmed) {
      throw new Error('Transaction confirmation timeout');
    }

    console.log('Transaction confirmed:', txSignature);

    // Step 8: Confirm with backend
    onProgress({ step: 'finalizing', message: 'Finalizing job creation...' });

    const confirmResponse = await fetch(`${apiBaseUrl}/api/jobs/${job_id}/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ signature: txSignature })
    });

    if (!confirmResponse.ok) {
      throw new Error('Failed to confirm job with backend');
    }

    onProgress({ step: 'complete', message: 'Job created successfully!' });

    return { jobId: job_id, signature: txSignature };

  } catch (error) {
    console.error('Error creating FHE job:', error);
    throw error;
  }
}

/**
 * Poll for job status until completion or failure
 *
 * @param {string} jobId - Job ID to poll
 * @param {string} apiBaseUrl - API base URL
 * @param {number} [maxAttempts=60] - Maximum polling attempts
 * @param {number} [intervalMs=2000] - Polling interval in ms
 * @param {Function} [onProgress] - Progress callback
 * @returns {Promise<Object>} - Final job status
 */
export async function pollJobStatus(jobId, apiBaseUrl, maxAttempts = 60, intervalMs = 2000, onProgress = () => {}) {
  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    const response = await fetch(`${apiBaseUrl}/api/jobs/${jobId}/status`);

    if (!response.ok) {
      throw new Error(`Failed to get job status: ${response.statusText}`);
    }

    const statusData = await response.json();
    const status = statusData.status;

    onProgress({
      attempt: attempt + 1,
      maxAttempts,
      status,
      data: statusData
    });

    if (status === 'completed') {
      return statusData;
    }

    if (status === 'failed' || status === 'expired') {
      throw new Error(`Job ${status}: ${statusData.error || 'Unknown error'}`);
    }

    await new Promise(r => setTimeout(r, intervalMs));
  }

  throw new Error('Job polling timeout');
}

/**
 * Get the encrypted result for a completed job
 *
 * @param {string} jobId - Job ID
 * @param {string} apiBaseUrl - API base URL
 * @returns {Promise<Object>} - Encrypted result data
 */
export async function getJobResult(jobId, apiBaseUrl) {
  const response = await fetch(`${apiBaseUrl}/api/fhe-result/${jobId}`);

  if (!response.ok) {
    throw new Error(`Failed to get job result: ${response.statusText}`);
  }

  return response.json();
}

// Legacy function - DEPRECATED, use createFheJobFromWitness instead
export async function createFheJob(operation, values, wallet, apiBaseUrl) {
  console.warn('createFheJob is deprecated, use createFheJobFromWitness instead');

  if (!wallet.connected) {
    throw new Error('Wallet not connected');
  }

  const nonce = Date.now().toString();
  const message = `create_job:${nonce}:${Date.now()}:${nonce}`;

  const signedMessage = await wallet.provider.signMessage(new TextEncoder().encode(message));
  const signatureB64 = Buffer.from(signedMessage.signature).toString('base64');

  const requestBody = {
    creator_pubkey: wallet.publicKey,
    encrypted_data: "placeholder",
    server_key: "placeholder",
    message,
    signature: signatureB64,
    nonce,
    operation,
    operation_value: 0,
    price_lamports: 10000000,
    required_provers: 1,
    consensus_threshold: 1,
    expected_count: values.length,
  };

  const validateResponse = await fetch(`${apiBaseUrl}/api/jobs/validate-and-build`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(requestBody),
  });

  if (!validateResponse.ok) {
    const error = await validateResponse.json();
    throw new Error(`Failed to validate job: ${error.error}`);
  }

  const { job_id, transaction: txB64 } = await validateResponse.json();

  const txBuf = Buffer.from(txB64, 'base64');
  const tx = Transaction.from(txBuf);

  const signedTx = await wallet.provider.signTransaction(tx);
  const signature = await wallet.provider.sendAndConfirm(signedTx);

  const confirmResponse = await fetch(`${apiBaseUrl}/api/jobs/${job_id}/confirm`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ signature }),
  });

  if (!confirmResponse.ok) {
    const error = await confirmResponse.json();
    throw new Error(`Failed to confirm job: ${error.error}`);
  }

  return { jobId: job_id, signature };
}
