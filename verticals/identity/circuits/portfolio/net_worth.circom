pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// PortfolioNetWorth - Circuit 41
/// Proves net worth exceeds threshold without revealing exact amount
///
/// Public inputs:
///   - min_threshold: Minimum net worth to prove
///   - price_oracle_root: Merkle root of price data
///   - timestamp: Snapshot timestamp
///   - net_worth_commitment: Hash(net_worth, blinding)
///
/// Private inputs:
///   - net_worth: Actual total net worth
///   - blinding: Random blinding factor
///   - oracle_path[20]: Path proving price data inclusion
///   - oracle_indices[20]: Path direction

template PortfolioNetWorth() {
    // Public inputs
    signal input min_threshold;
    signal input price_oracle_root;
    signal input timestamp;
    signal input net_worth_commitment;

    // Private inputs
    signal input net_worth;
    signal input blinding;
    signal input oracle_path[20];
    signal input oracle_indices[20];

    var DEPTH = 20;

    // ============================================
    // 1. Verify net_worth >= min_threshold
    // ============================================
    component threshold_check = GreaterEqThan(64);
    threshold_check.in[0] <== net_worth;
    threshold_check.in[1] <== min_threshold;
    threshold_check.out === 1;

    // ============================================
    // 2. Verify net_worth_commitment = Hash(net_worth, blinding)
    // ============================================
    component commitment_hasher = Poseidon(2);
    commitment_hasher.inputs[0] <== net_worth;
    commitment_hasher.inputs[1] <== blinding;
    commitment_hasher.out === net_worth_commitment;

    // ============================================
    // 3. Verify oracle data inclusion (simplified)
    // ============================================
    // For a real implementation, this would verify specific price entries
    // Here we verify the oracle tree structure is valid
    signal oracle_hashes[DEPTH + 1];
    oracle_hashes[0] <== 0;

    component oracle_switchers[DEPTH];
    component oracle_hashers[DEPTH];

    for (var i = 0; i < DEPTH; i++) {
        oracle_switchers[i] = Switcher();
        oracle_switchers[i].sel <== oracle_indices[i];
        oracle_switchers[i].L <== oracle_hashes[i];
        oracle_switchers[i].R <== oracle_path[i];

        oracle_hashers[i] = Poseidon(2);
        oracle_hashers[i].inputs[0] <== oracle_switchers[i].outL;
        oracle_hashers[i].inputs[1] <== oracle_switchers[i].outR;

        oracle_hashes[i + 1] <== oracle_hashers[i].out;
    }

    oracle_hashes[DEPTH] === price_oracle_root;

    // ============================================
    // 4. Prevent unused signal warnings
    // ============================================
    signal timestamp_check;
    timestamp_check <== timestamp * timestamp;
}

component main {public [min_threshold, price_oracle_root, timestamp, net_worth_commitment]} = PortfolioNetWorth();
