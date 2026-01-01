import { Connection, Transaction, Keypair } from '@solana/web3.js';
import { readFileSync } from 'fs';

const unsignedTxBase64 = process.argv[2];
const keypairPath = process.argv[3];

const txBytes = Buffer.from(unsignedTxBase64, 'base64');
const keypairData = JSON.parse(readFileSync(keypairPath, 'utf-8'));
const keypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));

console.log('--- Before ---');
const tx = Transaction.from(txBytes);
console.log('Recent blockhash:', tx.recentBlockhash);

console.log('\n--- Getting new blockhash ---');
const connection = new Connection('http://localhost:8899', 'confirmed');
const { blockhash } = await connection.getLatestBlockhash();
console.log('New blockhash:', blockhash);

tx.recentBlockhash = blockhash;
tx.feePayer = keypair.publicKey;
console.log('\n--- After assignment ---');
console.log('Recent blockhash:', tx.recentBlockhash);

tx.sign(keypair);
console.log('\n--- After signing ---');
console.log('Recent blockhash:', tx.recentBlockhash);
console.log('Signatures:', tx.signatures.map(s => s.signature ? 'signed' : 'empty'));

const serialized = tx.serialize();
console.log('\n--- Final serialized ---');
console.log('Length:', serialized.length);

// Deserialize to verify
const txCheck = Transaction.from(serialized);
console.log('Verified blockhash:', txCheck.recentBlockhash);
