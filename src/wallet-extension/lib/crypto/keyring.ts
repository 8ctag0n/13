/**
 * ZYBERLINK Wallet - Keyring Module
 *
 * WHY EXPANDED PRIVATE KEY INSTEAD OF MNEMONIC (BIP39)?
 * =====================================================
 *
 * 1. UNIVERSAL COMPATIBILITY
 *    - A 64-byte expanded private key (32 secret + 32 public) works with ANY
 *      ed25519-based system: Solana, Starknet, Zcash, and future chains
 *    - No chain-specific derivation paths (m/44'/501'/0'/0', etc.)
 *    - Same key = same addresses everywhere
 *
 * 2. SIMPLICITY
 *    - Mnemonic requires: BIP39 → Seed → SLIP-0010 → Chain-specific derivation
 *    - Private key requires: Just ed25519 public key derivation
 *    - Less code = fewer bugs = easier to audit
 *
 * 3. FHE COMPATIBILITY
 *    - Our FHE (Fully Homomorphic Encryption) system generates keys directly
 *    - No need to convert between mnemonic and raw keys
 *    - Direct integration with witness.bin outputs
 *
 * 4. INTEROPERABILITY
 *    - Users can import/export keys from any wallet that supports raw keys
 *    - Works with hardware wallets, CLI tools, and other systems
 *    - No vendor lock-in to BIP39 word lists
 *
 * FORMAT:
 *    - 32 bytes: Secret key (random entropy)
 *    - 32 bytes: Public key (derived via ed25519)
 *    - Total: 64 bytes = 128 hex characters or ~88 base58 characters
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import bs58 from 'bs58';

// Enable sync methods for @noble/ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export interface DerivedKeypair {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}

export interface WalletKeys {
  solana: DerivedKeypair;
  starknet: DerivedKeypair;
  zcash: DerivedKeypair;
}

// Generate random 32-byte private key
export function generatePrivateKey(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(32));
}

// Derive public key from private key using ed25519
export function derivePublicKey(privateKey: Uint8Array): Uint8Array {
  return ed.getPublicKey(privateKey);
}

// Create keypair from private key
export function createKeypair(privateKey: Uint8Array): DerivedKeypair {
  if (privateKey.length !== 32) {
    throw new Error('Private key must be 32 bytes');
  }
  return {
    secretKey: privateKey,
    publicKey: derivePublicKey(privateKey)
  };
}

// Get expanded private key (64 bytes: secretKey + publicKey)
export function getExpandedPrivateKey(keypair: DerivedKeypair): Uint8Array {
  const expanded = new Uint8Array(64);
  expanded.set(keypair.secretKey, 0);
  expanded.set(keypair.publicKey, 32);
  return expanded;
}

// Create keypair from expanded private key (64 bytes)
export function createKeypairFromExpanded(expandedKey: Uint8Array): DerivedKeypair {
  if (expandedKey.length !== 64) {
    throw new Error('Expanded private key must be 64 bytes');
  }
  return {
    secretKey: expandedKey.slice(0, 32),
    publicKey: expandedKey.slice(32, 64)
  };
}

// Validate private key format
export function validatePrivateKey(key: string): boolean {
  try {
    const bytes = parsePrivateKey(key);
    return bytes.length === 32 || bytes.length === 64;
  } catch {
    return false;
  }
}

// Parse private key from hex or base58
export function parsePrivateKey(key: string): Uint8Array {
  const cleaned = key.trim();

  // Try hex format (64 or 128 chars)
  if (/^[0-9a-fA-F]+$/.test(cleaned)) {
    if (cleaned.length === 64) {
      return hexToBytes(cleaned);
    }
    if (cleaned.length === 128) {
      return hexToBytes(cleaned);
    }
  }

  // Try base58 format
  try {
    const decoded = bs58.decode(cleaned);
    if (decoded.length === 32 || decoded.length === 64) {
      return decoded;
    }
  } catch {}

  throw new Error('Invalid private key format. Expected 32-byte hex (64 chars), 64-byte hex (128 chars), or base58');
}

// Create all chain keypairs from single private key
export function deriveAllKeypairs(privateKey: Uint8Array): WalletKeys {
  const keypair = privateKey.length === 64
    ? createKeypairFromExpanded(privateKey)
    : createKeypair(privateKey);

  // Same keypair for all chains (universal key)
  return {
    solana: keypair,
    starknet: keypair,
    zcash: keypair
  };
}

// Convert bytes to hex string
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

// Convert hex string to bytes
export function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

export function keypairToBase58(keypair: DerivedKeypair): {
  publicKey: string;
  secretKey: string;
} {
  return {
    publicKey: bs58.encode(keypair.publicKey),
    secretKey: bs58.encode(keypair.secretKey)
  };
}

export function base58ToKeypair(encoded: { publicKey: string; secretKey: string }): DerivedKeypair {
  return {
    publicKey: bs58.decode(encoded.publicKey),
    secretKey: bs58.decode(encoded.secretKey)
  };
}

// Sign a message with Ed25519
export async function signMessage(message: Uint8Array, secretKey: Uint8Array): Promise<Uint8Array> {
  return ed.sign(message, secretKey);
}

// Verify a signature
export async function verifySignature(
  signature: Uint8Array,
  message: Uint8Array,
  publicKey: Uint8Array
): Promise<boolean> {
  return ed.verify(signature, message, publicKey);
}
