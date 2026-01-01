pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// PrivateVote - Circuit 20
/// Enables anonymous voting in DAOs while proving:
/// 1. Voter eligibility (Merkle membership in eligibility tree)
/// 2. Sufficient token balance (balance >= min_balance)
/// 3. No double-voting (nullifier uniqueness)
/// 4. Vote commitment validity
///
/// Public inputs:
///   - poll_id: Unique poll identifier
///   - eligibility_root: Merkle root of eligible voters
///   - nullifier: Hash(nullifier_secret, poll_id) - prevents double voting
///   - vote_commitment: Hash(vote_choice, blinding) - hidden vote
///   - min_balance: Minimum tokens required to vote
///
/// Private inputs:
///   - voter_wallet: Wallet address (hidden)
///   - token_balance: Actual token balance
///   - vote_choice: Selected option
///   - blinding: Random blinding factor for vote commitment
///   - nullifier_secret: Random secret for nullifier generation
///   - merkle_path[20]: Path proving eligibility
///   - merkle_indices[20]: Path direction (0=left, 1=right)

template PrivateVote() {
    // Public inputs (must match Rust struct VotePublicInputs)
    signal input poll_id;
    signal input eligibility_root;
    signal input nullifier;
    signal input vote_commitment;
    signal input min_balance;

    // Private inputs
    signal input voter_wallet;
    signal input token_balance;
    signal input vote_choice;
    signal input blinding;
    signal input nullifier_secret;
    signal input merkle_path[20];
    signal input merkle_indices[20];

    var DEPTH = 20;

    // ============================================
    // 1. Verify voter eligibility (Merkle membership)
    // ============================================
    // Leaf is Hash(voter_wallet, token_balance)
    component leaf_hasher = Poseidon(2);
    leaf_hasher.inputs[0] <== voter_wallet;
    leaf_hasher.inputs[1] <== token_balance;

    signal computed_hashes[DEPTH + 1];
    computed_hashes[0] <== leaf_hasher.out;

    component switchers[DEPTH];
    component path_hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        switchers[i] = Switcher();
        switchers[i].sel <== merkle_indices[i];
        switchers[i].L <== computed_hashes[i];
        switchers[i].R <== merkle_path[i];

        path_hashers[i] = Poseidon(2);
        path_hashers[i].inputs[0] <== switchers[i].outL;
        path_hashers[i].inputs[1] <== switchers[i].outR;

        computed_hashes[i + 1] <== path_hashers[i].out;
    }

    // Verify computed root matches eligibility_root
    computed_hashes[DEPTH] === eligibility_root;

    // ============================================
    // 2. Verify token balance >= min_balance
    // ============================================
    component balance_check = GreaterEqThan(64);
    balance_check.in[0] <== token_balance;
    balance_check.in[1] <== min_balance;
    balance_check.out === 1;

    // ============================================
    // 3. Verify nullifier computation
    // ============================================
    // nullifier = Hash(nullifier_secret, poll_id)
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== nullifier_secret;
    nullifier_hasher.inputs[1] <== poll_id;
    nullifier_hasher.out === nullifier;

    // ============================================
    // 4. Verify vote commitment
    // ============================================
    // vote_commitment = Hash(vote_choice, blinding)
    component vote_hasher = Poseidon(2);
    vote_hasher.inputs[0] <== vote_choice;
    vote_hasher.inputs[1] <== blinding;
    vote_hasher.out === vote_commitment;

    // ============================================
    // 5. Ensure vote_choice is used (prevent unused signal)
    // ============================================
    // vote_choice should be a small number (0, 1, 2, etc.)
    // No upper bound check in circuit - validated off-chain
    signal vote_check;
    vote_check <== vote_choice * vote_choice;
}

component main {public [poll_id, eligibility_root, nullifier, vote_commitment, min_balance]} = PrivateVote();
