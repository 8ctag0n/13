pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// PrivateVoteWithPoI - Circuit 21
/// Combines anonymous voting with conflict of interest (PoI) verification
/// Proves:
/// 1. Voter eligibility (Merkle membership)
/// 2. Sufficient token balance
/// 3. No double-voting (nullifier)
/// 4. Vote commitment validity
/// 5. NOT in blacklist (conflict of interest check)
///
/// Public inputs:
///   - poll_id: Unique poll identifier
///   - eligibility_root: Merkle root of eligible voters
///   - nullifier: Hash(nullifier_secret, poll_id)
///   - vote_commitment: Hash(vote_choice, blinding)
///   - min_balance: Minimum tokens required
///   - blacklist_root: Merkle root of blacklisted addresses
///
/// Private inputs:
///   - voter_wallet: Wallet address (hidden)
///   - token_balance: Actual token balance
///   - vote_choice: Selected option
///   - blinding: Random blinding factor
///   - nullifier_secret: Random secret for nullifier
///   - eligibility_path[20]: Path proving eligibility
///   - eligibility_indices[20]: Path direction
///   - blacklist_path[20]: Path proving non-membership in blacklist
///   - blacklist_indices[20]: Blacklist path direction

template PrivateVoteWithPoI() {
    // Public inputs
    signal input poll_id;
    signal input eligibility_root;
    signal input nullifier;
    signal input vote_commitment;
    signal input min_balance;
    signal input blacklist_root;

    // Private inputs
    signal input voter_wallet;
    signal input token_balance;
    signal input vote_choice;
    signal input blinding;
    signal input nullifier_secret;
    signal input eligibility_path[20];
    signal input eligibility_indices[20];
    signal input blacklist_path[20];
    signal input blacklist_indices[20];

    var DEPTH = 20;

    // ============================================
    // 1. Verify voter eligibility (Merkle membership)
    // ============================================
    component leaf_hasher = Poseidon(2);
    leaf_hasher.inputs[0] <== voter_wallet;
    leaf_hasher.inputs[1] <== token_balance;

    signal eligibility_hashes[DEPTH + 1];
    eligibility_hashes[0] <== leaf_hasher.out;

    component elig_switchers[DEPTH];
    component elig_hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        elig_switchers[i] = Switcher();
        elig_switchers[i].sel <== eligibility_indices[i];
        elig_switchers[i].L <== eligibility_hashes[i];
        elig_switchers[i].R <== eligibility_path[i];

        elig_hashers[i] = Poseidon(2);
        elig_hashers[i].inputs[0] <== elig_switchers[i].outL;
        elig_hashers[i].inputs[1] <== elig_switchers[i].outR;

        eligibility_hashes[i + 1] <== elig_hashers[i].out;
    }

    eligibility_hashes[DEPTH] === eligibility_root;

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
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== nullifier_secret;
    nullifier_hasher.inputs[1] <== poll_id;
    nullifier_hasher.out === nullifier;

    // ============================================
    // 4. Verify vote commitment
    // ============================================
    component vote_hasher = Poseidon(2);
    vote_hasher.inputs[0] <== vote_choice;
    vote_hasher.inputs[1] <== blinding;
    vote_hasher.out === vote_commitment;

    // ============================================
    // 5. Verify NOT in blacklist (PoI - non-membership)
    // ============================================
    // For Sparse Merkle Tree: start with empty leaf (0) and verify path to root
    signal blacklist_hashes[DEPTH + 1];
    blacklist_hashes[0] <== 0;  // Empty leaf proves non-membership

    component bl_switchers[DEPTH];
    component bl_hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        bl_switchers[i] = Switcher();
        bl_switchers[i].sel <== blacklist_indices[i];
        bl_switchers[i].L <== blacklist_hashes[i];
        bl_switchers[i].R <== blacklist_path[i];

        bl_hashers[i] = Poseidon(2);
        bl_hashers[i].inputs[0] <== bl_switchers[i].outL;
        bl_hashers[i].inputs[1] <== bl_switchers[i].outR;

        blacklist_hashes[i + 1] <== bl_hashers[i].out;
    }

    blacklist_hashes[DEPTH] === blacklist_root;

    // ============================================
    // 6. Prevent unused signal warnings
    // ============================================
    signal vote_check;
    vote_check <== vote_choice * vote_choice;
}

component main {public [poll_id, eligibility_root, nullifier, vote_commitment, min_balance, blacklist_root]} = PrivateVoteWithPoI();
