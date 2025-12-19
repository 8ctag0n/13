import { Connection, Transaction, Keypair } from '@solana/web3.js';
import { readFileSync } from 'fs';

const unsignedTxBase64 = process.argv[2];
const keypairPath = process.argv[3];

const txBytes = Buffer.from(unsignedTxBase64, 'base64');
const keypairData = JSON.parse(readFileSync(keypairPath, 'utf-8'));
const keypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));

const tx = Transaction.from(txBytes);
const connection = new Connection('http://localhost:8899', 'confirmed');
const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();

tx.recentBlockhash = blockhash;
tx.feePayer = keypair.publicKey;
tx.sign(keypair);

console.log('Sending transaction...');
try {
    const signature = await connection.sendRawTransaction(tx.serialize(), {
        skipPreflight: false,
        preflightCommitment: 'confirmed'
    });
    console.log('Signature:', signature);
    
    const confirmation = await connection.confirmTransaction({
        signature,
        blockhash,
        lastValidBlockHeight
    }, 'confirmed');
    console.log('Confirmation:', confirmation);
} catch (e) {
    console.log('Error:', e.message);
    if (e.logs) console.log('Logs:', e.logs);
}
