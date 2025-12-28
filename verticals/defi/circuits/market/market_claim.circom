pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";

/// MarketClaim - Circuit 32
/// Claim payouts from settled markets.
/// Proves:
/// 1. Original bet commitment matches.
/// 2. Bet side matches winning resolution.
/// 3. Nullifier prevents double-claim.
/// 4. Payout matches pool math (with rounding tolerance).
///
/// Public inputs:
///   - market_id: Market identifier
///   - nullifier: Poseidon(secret, bet_commitment)
///   - payout_amount: Calculated payout
///   - resolution: Winning outcome (0/1)
///   - total_pool: Total pool size
///   - winning_pool: Winning side pool
///   - bet_commitment: Original bet commitment
///   - timestamp: Claim timestamp (binds to input set)
///
/// Private inputs:
///   - secret: User secret
///   - bet_amount: Original bet amount
///   - bet_side: Bet side (0/1)
///   - blinding: Commitment blinding

template MarketClaim() {
    // Public inputs
    signal input market_id;
    signal input nullifier;
    signal input payout_amount;
    signal input resolution;
    signal input total_pool;
    signal input winning_pool;
    signal input bet_commitment;
    signal input timestamp;

    // Private inputs
    signal input secret;
    signal input bet_amount;
    signal input bet_side;
    signal input blinding;

    // 1. Verify bet_commitment matches original bet
    component commitment_hasher = Poseidon(3);
    commitment_hasher.inputs[0] <== bet_amount;
    commitment_hasher.inputs[1] <== bet_side;
    commitment_hasher.inputs[2] <== blinding;
    commitment_hasher.out === bet_commitment;

    // 2. Verify nullifier = Poseidon(secret, bet_commitment)
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== bet_commitment;
    nullifier_hasher.out === nullifier;

    // 3. Verify bet_side matches resolution
    bet_side === resolution;

    // 4. Verify payout calculation (with rounding tolerance)
    signal numerator <== bet_amount * total_pool;
    signal expected_payout <-- numerator \ winning_pool;
    signal remainder <-- numerator % winning_pool;

    signal reconstructed <== expected_payout * winning_pool + remainder;
    reconstructed === numerator;

    component remainder_lt = LessThan(64);
    remainder_lt.in[0] <== remainder;
    remainder_lt.in[1] <== winning_pool;
    remainder_lt.out === 1;

    component eq1 = IsEqual();
    eq1.in[0] <== payout_amount;
    eq1.in[1] <== expected_payout;

    component eq2 = IsEqual();
    eq2.in[0] <== payout_amount;
    eq2.in[1] <== expected_payout - 1;

    signal valid_payout <== eq1.out + eq2.out;
    component valid_gt_zero = GreaterThan(2);
    valid_gt_zero.in[0] <== valid_payout;
    valid_gt_zero.in[1] <== 0;
    valid_gt_zero.out === 1;

    // 5. Range checks
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    component payout_positive = GreaterThan(64);
    payout_positive.in[0] <== payout_amount;
    payout_positive.in[1] <== 0;
    payout_positive.out === 1;

    component payout_bounded = LessEqThan(64);
    payout_bounded.in[0] <== payout_amount;
    payout_bounded.in[1] <== total_pool;
    payout_bounded.out === 1;

    component pool_positive = GreaterThan(64);
    pool_positive.in[0] <== winning_pool;
    pool_positive.in[1] <== 0;
    pool_positive.out === 1;

    // Binary checks
    bet_side * (bet_side - 1) === 0;
    resolution * (resolution - 1) === 0;

    // market_id and timestamp are public inputs used for binding only.
    signal _market_bind;
    _market_bind <== market_id * timestamp;
}

component main {public [
    market_id,
    nullifier,
    payout_amount,
    resolution,
    total_pool,
    winning_pool,
    bet_commitment,
    timestamp
]} = MarketClaim();
