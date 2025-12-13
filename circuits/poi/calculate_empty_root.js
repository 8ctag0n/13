const { buildPoseidon } = require("circomlibjs");

async function calculateEmptyTree() {
    const poseidon = await buildPoseidon();

    const DEPTH = 20;
    let currentHash = BigInt(0); // Empty leaf
    let emptyHashes = [currentHash.toString()];

    console.log("Calculating empty Sparse Merkle Tree hashes...\n");
    console.log(`Level 0 (leaf): ${currentHash.toString()}`);

    for (let i = 0; i < DEPTH; i++) {
        // At each level, hash(currentHash, currentHash) because both children are empty
        const inputs = [currentHash, currentHash];
        const hash = poseidon(inputs);
        currentHash = poseidon.F.toObject(hash);
        emptyHashes.push(currentHash.toString());
        console.log(`Level ${i + 1}: ${currentHash.toString()}`);
    }

    const emptyRoot = currentHash.toString();
    console.log(`\nEmpty Root (depth ${DEPTH}): ${emptyRoot}`);

    // Generate the valid input.json
    // For an empty tree, merkle_path at each level is the empty hash of that level
    const merkle_path = [];
    for (let i = 0; i < DEPTH; i++) {
        merkle_path.push(emptyHashes[i]);
    }

    const input = {
        blacklist_root: emptyRoot,
        threshold: "0",
        timestamp: "1702400000",
        wallet_address: "123456789",
        merkle_path: merkle_path,
        merkle_indices: Array(DEPTH).fill("0")
    };

    console.log("\n--- Valid input.json ---\n");
    console.log(JSON.stringify(input, null, 2));

    // Write to file
    const fs = require('fs');
    fs.writeFileSync('input_valid.json', JSON.stringify(input, null, 2));
    console.log("\nWritten to input_valid.json");
}

calculateEmptyTree().catch(console.error);
