export interface EncryptedData {
  ciphertext: string;
  iv: string;
  salt: string;
}

const PBKDF2_ITERATIONS = 100000;
const KEY_LENGTH = 256;
const SALT_LENGTH = 16;
const IV_LENGTH = 12;

async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const encoder = new TextEncoder();
  const passwordKey = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  );

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: PBKDF2_ITERATIONS,
      hash: 'SHA-256'
    },
    passwordKey,
    { name: 'AES-GCM', length: KEY_LENGTH },
    false,
    ['encrypt', 'decrypt']
  );
}

export async function encryptVault(data: any, password: string): Promise<EncryptedData> {
  const salt = crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));

  const key = await deriveKey(password, salt);

  const encoder = new TextEncoder();
  const plaintext = encoder.encode(JSON.stringify(data));

  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    plaintext
  );

  return {
    ciphertext: bufferToBase64(ciphertext),
    iv: bufferToBase64(iv),
    salt: bufferToBase64(salt)
  };
}

export async function decryptVault(encryptedData: EncryptedData, password: string): Promise<any> {
  const salt = base64ToBuffer(encryptedData.salt);
  const iv = base64ToBuffer(encryptedData.iv);
  const ciphertext = base64ToBuffer(encryptedData.ciphertext);

  const key = await deriveKey(password, salt);

  try {
    const plaintext = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv },
      key,
      ciphertext
    );

    const decoder = new TextDecoder();
    const decryptedString = decoder.decode(plaintext);

    return JSON.parse(decryptedString);
  } catch (error) {
    throw new Error('Incorrect password or corrupted data');
  }
}

export async function verifyPassword(password: string, encryptedData: EncryptedData): Promise<boolean> {
  try {
    await decryptVault(encryptedData, password);
    return true;
  } catch {
    return false;
  }
}

function bufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function base64ToBuffer(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
