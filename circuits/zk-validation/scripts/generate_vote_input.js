#!/usr/bin/env node
// Generate valid input for PrivateVote circuit (Circuit 20)

const { buildPoseidon } = require("circomlibjs");

async function main() {
  const poseidon = await buildPoseidon();
  const F = poseidon.F;

  // Private inputs
  const voter_wallet = BigInt(999);
  const token_balance = BigInt(1000);
  const vote_choice = BigInt(1); // voting for option 1
  const blinding = BigInt(12345);
  const nullifier_secret = BigInt(67890);
  const poll_id = BigInt(42);
  const min_balance = BigInt(100);

  // Compute leaf = Poseidon(voter_wallet, token_balance)
  const leaf = poseidon([F.e(voter_wallet), F.e(token_balance)]);

  // Simple merkle tree: only 1 leaf, rest are zeros
  // For depth 20, we need path and indices
  let current_hash = leaf;
  const merkle_path = [];
  const merkle_indices = [];

  for (let i = 0; i < 20; i++) {
    merkle_path.push("0");
    merkle_indices.push("0");
    // Hash with zero sibling on the right
    current_hash = poseidon([current_hash, F.e(0)]);
  }

  const eligibility_root = F.toString(current_hash);

  // Compute nullifier = Poseidon(nullifier_secret, poll_id)
  const nullifier = poseidon([F.e(nullifier_secret), F.e(poll_id)]);

  // Compute vote_commitment = Poseidon(vote_choice, blinding)
  const vote_commitment = poseidon([F.e(vote_choice), F.e(blinding)]);

  const input = {
    // Public inputs
    poll_id: poll_id.toString(),
    eligibility_root: eligibility_root,
    nullifier: F.toString(nullifier),
    vote_commitment: F.toString(vote_commitment),
    min_balance: min_balance.toString(),

    // Private inputs
    voter_wallet: voter_wallet.toString(),
    token_balance: token_balance.toString(),
    vote_choice: vote_choice.toString(),
    blinding: blinding.toString(),
    nullifier_secret: nullifier_secret.toString(),
    merkle_path: merkle_path,
    merkle_indices: merkle_indices
  };

  console.log(JSON.stringify(input, null, 2));
}

main().catch(console.error);
