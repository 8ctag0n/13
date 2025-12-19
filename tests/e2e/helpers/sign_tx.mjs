#!/usr/bin/env node
/**
 * Sign Transaction Helper
 *
 * Usage: node sign_tx.mjs <keypair_path> <unsigned_tx_base64>
 *
 * Takes an unsigned transaction (base64) and signs it with the provided keypair.
 * Outputs the signed transaction as base64.
 */

import { readFileSync } from 'fs';
import {
    Connection,
    Transaction,
    VersionedTransaction,
    Keypair
} from '@solana/web3.js';

const RPC_URL = process.env.SOLANA_RPC_URL || 'http://localhost:8899';

async function main() {
    const args = process.argv.slice(2);

    if (args.length < 2) {
        console.error('Usage: node sign_tx.mjs <keypair_path> <unsigned_tx_base64>');
        process.exit(1);
    }

    const [keypairPath, unsignedTxBase64] = args;

    // Load keypair
    const keypairData = JSON.parse(readFileSync(keypairPath, 'utf-8'));
    const keypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));

    // Decode unsigned transaction
    const txBytes = Buffer.from(unsignedTxBase64, 'base64');

    // Connect to get recent blockhash
    const connection = new Connection(RPC_URL, 'confirmed');
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();

    // Try to deserialize as legacy Transaction first
    let transaction;
    try {
        transaction = Transaction.from(txBytes);
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = keypair.publicKey;
        transaction.sign(keypair);
    } catch (e) {
        // Try as VersionedTransaction
        try {
            transaction = VersionedTransaction.deserialize(txBytes);
            transaction.message.recentBlockhash = blockhash;
            transaction.sign([keypair]);
        } catch (e2) {
            console.error('Failed to deserialize transaction:', e2.message);
            process.exit(1);
        }
    }

    // Serialize signed transaction
    const signedTxBytes = transaction.serialize();
    const signedTxBase64 = Buffer.from(signedTxBytes).toString('base64');

    // Output result as JSON
    console.log(JSON.stringify({
        signed_tx: signedTxBase64,
        pubkey: keypair.publicKey.toBase58(),
        blockhash: blockhash
    }));
}

main().catch(err => {
    console.error('Error:', err.message);
    process.exit(1);
});
