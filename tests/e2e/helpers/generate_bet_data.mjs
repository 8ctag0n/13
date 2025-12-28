#!/usr/bin/env node
/**
 * Generate Bet Data Helper
 *
 * Usage: node generate_bet_data.mjs [amount_lamports] [side]
 *
 * Generates mock data for a bet:
 * - ciphertext (random bytes, simulating FHE encrypted amount)
 * - ciphertext_hash (keccak256 of ciphertext)
 * - proof (256 bytes mock)
 * - public_inputs (80 bytes for circuit 30)
 */

import sha3 from 'js-sha3';
const { keccak256 } = sha3;

function randomBytes(length) {
    const bytes = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
        bytes[i] = Math.floor(Math.random() * 256);
    }
    return Buffer.from(bytes);
}

function main() {
    const args = process.argv.slice(2);
    const amountLamports = parseInt(args[0]) || 100_000_000; // 0.1 SOL default
    const side = args[1] === 'no' ? false : true; // default YES

    // Generate mock ciphertext (TFHE encrypted u64 is typically ~8KB but we use smaller for test)
    const ciphertext = randomBytes(1024);

    // Hash with keccak256 (matches server-side Keccak256)
    const ciphertextHash = keccak256(ciphertext);

    // Generate mock proof (Groth16 = 256 bytes)
    const proof = randomBytes(256);

    // Generate mock public inputs (circuit 30 = 80 bytes)
    const publicInputs = randomBytes(80);

    // Output as JSON
    // Note: js-sha3 keccak256() returns hex string directly
    const result = {
        ciphertext: ciphertext.toString('base64'),
        ciphertext_hash: ciphertextHash, // already hex string
        proof: proof.toString('base64'),
        public_inputs: publicInputs.toString('base64'),
        amount_lamports: amountLamports,
        side: side,
        circuit_type: 30
    };

    console.log(JSON.stringify(result, null, 2));
}

main();
