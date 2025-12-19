import { Transaction } from '@solana/web3.js';

const unsignedTxBase64 = process.argv[2];
const txBytes = Buffer.from(unsignedTxBase64, 'base64');

console.log('Bytes length:', txBytes.length);
console.log('First 10 bytes:', Array.from(txBytes.slice(0, 10)));

try {
    const tx = Transaction.from(txBytes);
    console.log('Deserialized OK');
    console.log('Fee payer:', tx.feePayer?.toBase58());
    console.log('Recent blockhash:', tx.recentBlockhash);
    console.log('Num signatures:', tx.signatures.length);
} catch (e) {
    console.log('Failed to deserialize:', e.message);
}
