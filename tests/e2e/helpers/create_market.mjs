#!/usr/bin/env node
/**
 * Create Market Helper
 *
 * Usage: node create_market.mjs <keypair_path> <market_id>
 *
 * Creates a futarchy market on-chain with the given market_id.
 */

import { readFileSync } from 'fs';
import {
    Connection,
    Transaction,
    TransactionInstruction,
    Keypair,
    PublicKey,
    SystemProgram,
    SYSVAR_CLOCK_PUBKEY,
    sendAndConfirmTransaction
} from '@solana/web3.js';
const RPC_URL = process.env.SOLANA_RPC_URL || 'http://localhost:8899';
const PROGRAM_ID = process.env.FUTARCHY_PROGRAM_ID || 'AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij';

// Manual borsh serialization for CreateMarket
// Layout: variant (u8) + market_id (u64) + question_hash (32 bytes) + end_time (i64) + max_bet (u64)
function serializeCreateMarket(marketId, questionHash, endTime, maxBet) {
    const buffer = Buffer.alloc(1 + 8 + 32 + 8 + 8); // 57 bytes total
    let offset = 0;

    // variant: 0 for CreateMarket
    buffer.writeUInt8(0, offset);
    offset += 1;

    // market_id: u64 LE
    buffer.writeBigUInt64LE(BigInt(marketId), offset);
    offset += 8;

    // question_hash: 32 bytes
    Buffer.from(questionHash).copy(buffer, offset);
    offset += 32;

    // end_time: i64 LE
    buffer.writeBigInt64LE(BigInt(endTime), offset);
    offset += 8;

    // max_bet: u64 LE
    buffer.writeBigUInt64LE(BigInt(maxBet), offset);

    return buffer;
}

function findMarketPDA(programId, marketId) {
    const buffer = Buffer.alloc(8);
    buffer.writeBigUInt64LE(BigInt(marketId));
    return PublicKey.findProgramAddressSync(
        [Buffer.from('market'), buffer],
        programId
    );
}

function findEscrowPDA(programId, marketId) {
    const buffer = Buffer.alloc(8);
    buffer.writeBigUInt64LE(BigInt(marketId));
    return PublicKey.findProgramAddressSync(
        [Buffer.from('escrow'), buffer],
        programId
    );
}

async function main() {
    const args = process.argv.slice(2);

    if (args.length < 2) {
        console.error('Usage: node create_market.mjs <keypair_path> <market_id>');
        process.exit(1);
    }

    const [keypairPath, marketIdStr] = args;
    const marketId = parseInt(marketIdStr);

    // Load keypair
    const keypairData = JSON.parse(readFileSync(keypairPath, 'utf-8'));
    const keypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));

    const connection = new Connection(RPC_URL, 'confirmed');
    const programId = new PublicKey(PROGRAM_ID);

    // Derive PDAs
    const [marketPda, marketBump] = findMarketPDA(programId, marketId);
    const [escrowPda, escrowBump] = findEscrowPDA(programId, marketId);

    console.log('Program ID:', PROGRAM_ID);
    console.log('Market ID:', marketId);
    console.log('Authority:', keypair.publicKey.toBase58());
    console.log('Market PDA:', marketPda.toBase58());
    console.log('Escrow PDA:', escrowPda.toBase58());

    // Create instruction data
    const questionHash = new Uint8Array(32).fill(0); // Empty question hash for test
    const endTime = Math.floor(Date.now() / 1000) + 86400; // 24 hours from now
    const maxBet = 10_000_000_000; // 10 SOL

    const serialized = serializeCreateMarket(marketId, questionHash, endTime, maxBet);
    console.log('Instruction data size:', serialized.length, 'bytes');

    // Build instruction
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: keypair.publicKey, isSigner: true, isWritable: true }, // Authority
            { pubkey: marketPda, isSigner: false, isWritable: true },        // Market PDA
            { pubkey: escrowPda, isSigner: false, isWritable: true },        // Escrow PDA
            { pubkey: keypair.publicKey, isSigner: false, isWritable: false }, // Oracle (using authority as oracle for test)
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        ],
        programId: programId,
        data: Buffer.from(serialized),
    });

    // Build and send transaction
    const tx = new Transaction().add(instruction);
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    tx.recentBlockhash = blockhash;
    tx.feePayer = keypair.publicKey;
    tx.sign(keypair);

    console.log('\nSending transaction...');

    try {
        const sig = await sendAndConfirmTransaction(connection, tx, [keypair], {
            commitment: 'confirmed',
            skipPreflight: false,
        });
        console.log('Success! Transaction:', sig);
        console.log(JSON.stringify({
            success: true,
            signature: sig,
            market_id: marketId,
            market_pda: marketPda.toBase58(),
            escrow_pda: escrowPda.toBase58()
        }));
    } catch (e) {
        console.error('Failed:', e.message);
        if (e.logs) {
            console.log('Logs:', e.logs.join('\n'));
        }
        process.exit(1);
    }
}

main();
