#!/usr/bin/env node
// Helper to compute Poseidon hashes using circomlibjs

const { buildPoseidon } = require("circomlibjs");

async function main() {
  const poseidon = await buildPoseidon();

  // Example: Poseidon hash of 2 inputs
  const hash2 = (a, b) => {
    const F = poseidon.F;
    return poseidon([F.e(a), F.e(b)]);
  };

  // Example: Poseidon hash of 3 inputs
  const hash3 = (a, b, c) => {
    const F = poseidon.F;
    return poseidon([F.e(a), F.e(b), F.e(c)]);
  };

  // Parse command line args
  if (process.argv.length < 4) {
    console.error("Usage: node compute_poseidon.js <num_inputs> <input1> [input2] [input3]");
    console.error("Example: node compute_poseidon.js 2 123 456");
    process.exit(1);
  }

  const numInputs = parseInt(process.argv[2]);
  const inputs = process.argv.slice(3).map(x => BigInt(x));

  if (inputs.length !== numInputs) {
    console.error(`Expected ${numInputs} inputs, got ${inputs.length}`);
    process.exit(1);
  }

  let result;
  if (numInputs === 2) {
    result = hash2(inputs[0], inputs[1]);
  } else if (numInputs === 3) {
    result = hash3(inputs[0], inputs[1], inputs[2]);
  } else {
    console.error("Only 2 or 3 inputs supported");
    process.exit(1);
  }

  console.log(poseidon.F.toString(result));
}

main().catch(console.error);
