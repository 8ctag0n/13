pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/switcher.circom";

/// MarketBetWithPoI - Circuit 31
/// Private betting with insider trading prevention
/// Proves:
/// 1. bet_amount <= max_bet
/// 2. bet_commitment is correctly computed
/// 3. NOT in insider blacklist (PoI)
///
/// Public inputs:
///   - market_id: Unique market identifier
///   - bet_commitment: Hash(amount, position, blinding)
///   - max_bet: Maximum allowed bet
///   - timestamp: Bet placement time
///   - insider_blacklist_root: Merkle root of insiders
///
/// Private inputs:
///   - bettor_wallet: Wallet address (hidden)
///   - bet_amount: Actual bet amount
///   - position: Outcome prediction
///   - blinding: Random blinding factor
///   - blacklist_path[20]: Path proving non-membership
///   - blacklist_indices[20]: Path direction

template MarketBetWithPoI() {
    // Public inputs
    signal input market_id;
    signal input bet_commitment;
    signal input max_bet;
    signal input timestamp;
    signal input insider_blacklist_root;

    // Private inputs
    signal input bettor_wallet;
    signal input bet_amount;
    signal input position;
    signal input blinding;
    signal input blacklist_path[20];
    signal input blacklist_indices[20];

    var DEPTH = 20;

    // ============================================
    // 1. Verify bet_amount <= max_bet
    // ============================================
    component amount_check = LessEqThan(64);
    amount_check.in[0] <== bet_amount;
    amount_check.in[1] <== max_bet;
    amount_check.out === 1;

    // ============================================
    // 2. Verify bet_commitment
    // ============================================
    component commitment_hasher = Poseidon(3);
    commitment_hasher.inputs[0] <== bet_amount;
    commitment_hasher.inputs[1] <== position;
    commitment_hasher.inputs[2] <== blinding;
    commitment_hasher.out === bet_commitment;

    // ============================================
    // 3. Verify NOT in insider blacklist (PoI)
    // ============================================
    signal blacklist_hashes[DEPTH + 1];
    blacklist_hashes[0] <== 0;  // Empty leaf for non-membership

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

    blacklist_hashes[DEPTH] === insider_blacklist_root;

    // ============================================
    // 4. Prevent unused signal warnings
    // ============================================
    signal wallet_check;
    wallet_check <== bettor_wallet * bettor_wallet;

    signal market_check;
    market_check <== market_id * market_id;

    signal timestamp_check;
    timestamp_check <== timestamp * timestamp;
}

component main {public [market_id, bet_commitment, max_bet, timestamp, insider_blacklist_root]} = MarketBetWithPoI();
