import { Transaction } from '@solana/web3.js';
import { createSolanaRpc } from '@solana/kit';

const DEFAULT_RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

function decodeBase64Transaction(base64Tx) {
  if (!base64Tx) {
    throw new Error('Missing unsigned transaction');
  }

  const txBytes = Uint8Array.from(atob(base64Tx), c => c.charCodeAt(0));
  return Transaction.from(txBytes);
}

export async function signAndSendUnsignedTx({
  unsignedTransaction,
  wallet,
  rpcUrl = DEFAULT_RPC_URL,
  commitment = 'confirmed',
  maxAttempts = 30,
  pollIntervalMs = 1000,
  onStatus = () => {}
}) {
  if (!wallet?.provider || !wallet?.publicKey) {
    throw new Error('Wallet not connected');
  }

  onStatus({ step: 'preparing', message: 'Preparing transaction...' });

  const transaction = decodeBase64Transaction(unsignedTransaction);
  const rpc = createSolanaRpc(rpcUrl);
  const { value: latestBlockhash } = await rpc.getLatestBlockhash({ commitment }).send();
  transaction.recentBlockhash = latestBlockhash.blockhash;
  transaction.feePayer = wallet.publicKey;

  onStatus({ step: 'signing', message: 'Waiting for wallet signature...' });
  const signedTx = await wallet.provider.signTransaction(transaction);

  onStatus({ step: 'sending', message: 'Sending transaction to Solana...' });
  const signedTxBase64 = btoa(String.fromCharCode(...signedTx.serialize()));
  const txSignature = await rpc.sendTransaction(signedTxBase64, {
    encoding: 'base64',
    skipPreflight: false,
    preflightCommitment: commitment
  }).send();

  onStatus({
    step: 'confirming',
    message: 'Confirming on-chain...',
    signature: txSignature,
    confirmations: 0,
    signedTxBase64
  });

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
    const statusResult = await rpc.getSignatureStatuses([txSignature]).send();
    const status = statusResult.value[0];

    if (status?.err) {
      throw new Error('Transaction failed');
    }

    const confirmations = status?.confirmations ?? 0;
    onStatus({
      step: 'confirming',
      message: 'Confirming on-chain...',
      signature: txSignature,
      confirmations,
      signedTxBase64
    });

    if (status?.confirmationStatus === 'confirmed' || status?.confirmationStatus === 'finalized') {
      return { signature: txSignature, confirmationStatus: status.confirmationStatus, signedTxBase64 };
    }
  }

  throw new Error('Transaction confirmation timeout');
}
