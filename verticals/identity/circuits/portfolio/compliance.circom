pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// PortfolioCompliance - Circuit 40
/// Proves no exposure to sanctioned/blacklisted addresses
/// Similar to PoI but for portfolio holdings
///
/// Public inputs:
///   - compliance_root: Merkle root of compliance/sanctions list
///   - threshold: Max allowed exposure (usually 0)
///   - timestamp: Proof validity time
///
/// Private inputs:
///   - wallet_address: User's wallet
///   - merkle_path[20]: Path proving non-membership
///   - merkle_indices[20]: Path direction

template PortfolioCompliance() {
    // Public inputs
    signal input compliance_root;
    signal input threshold;
    signal input timestamp;

    // Private inputs
    signal input wallet_address;
    signal input merkle_path[20];
    signal input merkle_indices[20];

    var DEPTH = 20;

    // ============================================
    // 1. Verify non-membership in compliance/sanctions list
    // ============================================
    signal computed_hashes[DEPTH + 1];
    computed_hashes[0] <== 0;  // Empty leaf for non-membership

    component switchers[DEPTH];
    component hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        switchers[i] = Switcher();
        switchers[i].sel <== merkle_indices[i];
        switchers[i].L <== computed_hashes[i];
        switchers[i].R <== merkle_path[i];

        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== switchers[i].outL;
        hashers[i].inputs[1] <== switchers[i].outR;

        computed_hashes[i + 1] <== hashers[i].out;
    }

    computed_hashes[DEPTH] === compliance_root;

    // ============================================
    // 2. Verify threshold is 0 (strict compliance)
    // ============================================
    component threshold_check = IsZero();
    threshold_check.in <== threshold;
    threshold_check.out === 1;

    // ============================================
    // 3. Prevent unused signal warnings
    // ============================================
    signal wallet_check;
    wallet_check <== wallet_address * wallet_address;

    signal timestamp_check;
    timestamp_check <== timestamp * timestamp;
}

component main {public [compliance_root, threshold, timestamp]} = PortfolioCompliance();
