/**
 * Token Account Manager
 * Handles SPL Token Account operations for wZEC payments
 * Using @solana/kit + @solana-program/token
 */

import { createSolanaRpc, address } from '@solana/kit';
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  TOKEN_PROGRAM_ADDRESS,
} from '@solana-program/token';

// wZEC mint address
export const WZEC_MINT = address('sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ');

/**
 * Check if a token account exists for the given owner and mint
 * @param {string} rpcUrl - RPC URL
 * @param {string} ownerAddress - Owner's public key string
 * @param {Address} mint - Token mint address
 * @returns {Promise<{exists: boolean, address: string|null}>}
 */
export async function checkTokenAccount(rpcUrl, ownerAddress, mint) {
  try {
    const rpc = createSolanaRpc(rpcUrl);
    const owner = address(ownerAddress);

    // Find the associated token address using PDA
    const [ataAddress] = await findAssociatedTokenPda({
      mint,
      owner,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    // Check if account exists
    const accountInfo = await rpc.getAccountInfo(ataAddress, { encoding: 'base64' }).send();

    return {
      exists: accountInfo.value !== null,
      address: accountInfo.value !== null ? ataAddress : null
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
 * Get the associated token address for an owner and mint
 * @param {string} ownerAddress - Owner's public key string
 * @param {Address} mint - Token mint address
 * @returns {Promise<Address>}
 */
export async function getTokenAddress(ownerAddress, mint) {
  const owner = address(ownerAddress);
  const [ataAddress] = await findAssociatedTokenPda({
    mint,
    owner,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  return ataAddress;
}

/**
 * Ensure token account exists, create if not
 * Note: This returns the instruction to create, actual signing happens in the caller
 * @param {string} rpcUrl - RPC URL
 * @param {Object} wallet - Wallet object with publicKey
 * @param {Address} mint - Token mint address
 * @param {Function} onProgress - Progress callback
 * @returns {Promise<{address: Address, needsCreation: boolean, instruction?: any}>}
 */
export async function ensureTokenAccount(rpcUrl, wallet, mint, onProgress = null) {
  if (!wallet || !wallet.publicKey) {
    throw new Error('Wallet not connected');
  }

  const ownerAddress = wallet.publicKey.toString();

  const reportProgress = (step, message) => {
    if (onProgress) onProgress({ step, message });
    console.log(`[Token Account Manager] ${step}: ${message}`);
  };

  reportProgress('checking', 'Checking for existing token account...');

  // Check if account exists
  const { exists, address: existingAddress } = await checkTokenAccount(rpcUrl, ownerAddress, mint);

  if (exists) {
    reportProgress('exists', `Token account found: ${existingAddress}`);
    return { address: existingAddress, needsCreation: false };
  }

  reportProgress('creating', 'Preparing token account creation...');

  // Get the ATA address
  const ataAddress = await getTokenAddress(ownerAddress, mint);

  // Create instruction for creating the ATA (idempotent - safe if already exists)
  const owner = address(ownerAddress);
  const instruction = await getCreateAssociatedTokenIdempotentInstructionAsync({
    mint,
    owner,
    payer: owner,
  });

  reportProgress('ready', `Token account instruction ready: ${ataAddress}`);

  return {
    address: ataAddress,
    needsCreation: true,
    instruction
  };
}

/**
 * Helper to check if wallet has wZEC token account
 * @param {string} rpcUrl - RPC URL
 * @param {string} walletPublicKey - Wallet public key string
 * @returns {Promise<boolean>}
 */
export async function hasWzecAccount(rpcUrl, walletPublicKey) {
  const { exists } = await checkTokenAccount(rpcUrl, walletPublicKey, WZEC_MINT);
  return exists;
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
