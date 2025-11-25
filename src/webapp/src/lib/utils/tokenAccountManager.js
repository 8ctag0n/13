/**
 * Token Account Manager
 * Handles SPL Token Account operations for wZEC payments
 */

import { PublicKey, Transaction, SystemProgram } from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
} from '@solana/spl-token';

/**
 * Check if a token account exists for the given owner and mint
 * @param {Connection} connection - Solana connection
 * @param {PublicKey} owner - Owner's public key
 * @param {PublicKey} mint - Token mint public key
 * @returns {Promise<{exists: boolean, address: PublicKey|null}>}
 */
export async function checkTokenAccount(connection, owner, mint) {
  try {
    const associatedTokenAddress = await getAssociatedTokenAddress(
      mint,
      owner,
      false, // allowOwnerOffCurve
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const accountInfo = await connection.getAccountInfo(associatedTokenAddress);

    return {
      exists: accountInfo !== null,
      address: accountInfo !== null ? associatedTokenAddress : null
    };
  } catch (error) {
    console.error('Error checking token account:', error);
    return {
      exists: false,
      address: null,
      error: error.message
    };
  }
}

/**
 * Create instruction to create an associated token account
 * @param {PublicKey} payer - Payer's public key (fee payer)
 * @param {PublicKey} owner - Owner's public key
 * @param {PublicKey} mint - Token mint public key
 * @returns {Promise<TransactionInstruction>}
 */
export async function createTokenAccountInstruction(payer, owner, mint) {
  const associatedTokenAddress = await getAssociatedTokenAddress(
    mint,
    owner,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  return createAssociatedTokenAccountInstruction(
    payer,
    associatedTokenAddress,
    owner,
    mint,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

/**
 * Ensure token account exists, create if not
 * @param {Connection} connection - Solana connection
 * @param {Object} wallet - Wallet adapter object
 * @param {PublicKey} mint - Token mint public key
 * @param {Function} onProgress - Progress callback
 * @returns {Promise<PublicKey>} - Associated token address
 */
export async function ensureTokenAccount(connection, wallet, mint, onProgress = null) {
  if (!wallet || !wallet.publicKey) {
    throw new Error('Wallet not connected');
  }

  const owner = wallet.publicKey;

  // Report progress
  const reportProgress = (step, message) => {
    if (onProgress) onProgress({ step, message });
    console.log(`[Token Account Manager] ${step}: ${message}`);
  };

  reportProgress('checking', 'Checking for existing token account...');

  // Check if account exists
  const { exists, address } = await checkTokenAccount(connection, owner, mint);

  if (exists) {
    reportProgress('exists', `Token account found: ${address.toString()}`);
    return address;
  }

  reportProgress('creating', 'Creating new token account...');

  try {
    // Create instruction
    const instruction = await createTokenAccountInstruction(owner, owner, mint);

    // Build transaction
    const transaction = new Transaction().add(instruction);
    transaction.feePayer = owner;

    // Get recent blockhash
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;

    reportProgress('signing', 'Waiting for wallet signature...');

    // Sign and send transaction
    const signed = await wallet.signTransaction(transaction);
    const signature = await connection.sendRawTransaction(signed.serialize());

    reportProgress('confirming', 'Confirming transaction...');

    // Confirm transaction
    await connection.confirmTransaction(signature, 'confirmed');

    // Get the created token account address
    const tokenAccountAddress = await getAssociatedTokenAddress(
      mint,
      owner,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    reportProgress('success', `Token account created: ${tokenAccountAddress.toString()}`);

    return tokenAccountAddress;
  } catch (error) {
    reportProgress('error', `Failed to create token account: ${error.message}`);
    throw error;
  }
}

/**
 * Get associated token address (without checking if it exists)
 * @param {PublicKey} owner - Owner's public key
 * @param {PublicKey} mint - Token mint public key
 * @returns {Promise<PublicKey>}
 */
export async function getTokenAddress(owner, mint) {
  return await getAssociatedTokenAddress(
    mint,
    owner,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

/**
 * Format token account info for display
 * @param {Object} accountInfo - Token account info from checkTokenAccount
 * @returns {Object}
 */
export function formatTokenAccountInfo(accountInfo) {
  if (!accountInfo.exists) {
    return {
      status: 'not_found',
      message: 'Token account does not exist',
      requiresCreation: true
    };
  }

  return {
    status: 'found',
    message: 'Token account exists',
    address: accountInfo.address?.toString(),
    requiresCreation: false
  };
}

/**
 * Constants
 */
export const WZEC_MINT = new PublicKey('sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ');

/**
 * Helper to check if wallet has wZEC token account
 * @param {Connection} connection
 * @param {PublicKey} walletPublicKey
 * @returns {Promise<boolean>}
 */
export async function hasWzecAccount(connection, walletPublicKey) {
  const { exists } = await checkTokenAccount(connection, walletPublicKey, WZEC_MINT);
  return exists;
}
