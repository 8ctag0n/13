pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";

/*
 * Circuit 32: MarketClaim
 *
 * Proves eligibility to claim payout from a settled futarchy market
 * without revealing bet amount or identity.
 *
 * Public Inputs:
 *   - market_id:      Market identifier
 *   - nullifier:      Poseidon(secret, bet_commitment) - prevents double claims
 *   - payout_amount:  Amount being claimed
 *   - resolution:     Market outcome (1=YES won, 0=NO won)
 *   - total_pool:     Total amount in market pool
 *   - winning_pool:   Amount in winning side pool
 *   - bet_commitment: User's original bet commitment
 *   - timestamp:      Claim timestamp (binds input set)
 *
 * Private Inputs:
 *   - secret:         User's secret key (32 bytes as field element)
 *   - bet_amount:     Original bet amount
 *   - bet_side:       Which side user bet (1=YES, 0=NO)
 *   - blinding:       Randomness used in commitment
 *
 * Constraints (~500):
 *   1. bet_commitment == Poseidon(bet_amount, bet_side, blinding)
 *   2. nullifier == Poseidon(secret, bet_commitment)
 *   3. bet_side == resolution (user must have bet on winning side)
 *   4. payout_amount == (bet_amount * total_pool) / winning_pool (with rounding)
 *   5. Range checks for amounts
 */

template MarketClaim() {
    // ===== Public Inputs =====
    signal input market_id;
    signal input nullifier;
    signal input payout_amount;
    signal input resolution;      // 0 or 1
    signal input total_pool;
    signal input winning_pool;
    signal input bet_commitment;
    signal input timestamp;

    // ===== Private Inputs =====
    signal input secret;
    signal input bet_amount;
    signal input bet_side;        // 0 or 1
    signal input blinding;

    // ===== Constraint 1: Verify bet_commitment =====
    // bet_commitment = Poseidon(bet_amount, bet_side, blinding)
    component commitment_hasher = Poseidon(3);
    commitment_hasher.inputs[0] <== bet_amount;
    commitment_hasher.inputs[1] <== bet_side;
    commitment_hasher.inputs[2] <== blinding;
    commitment_hasher.out === bet_commitment;

    // ===== Constraint 2: Verify nullifier =====
    // nullifier = Poseidon(secret, bet_commitment)
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== bet_commitment;
    nullifier_hasher.out === nullifier;

    // ===== Constraint 3: Verify winner =====
    // User must have bet on winning side
    bet_side === resolution;

    // ===== Constraint 4: Verify payout calculation =====
    // payout = bet_amount * total_pool / winning_pool
    //
    // We use the constraint:
    //   bet_amount * total_pool = expected_payout * winning_pool + remainder
    //   where 0 <= remainder < winning_pool
    //
    // Allow payout_amount to be expected_payout or expected_payout - 1
    // to tolerate rounding behavior.

    signal numerator <== bet_amount * total_pool;

    // Compute quotient and remainder (non-constrained)
    signal expected_payout <-- numerator \ winning_pool;
    signal remainder <-- numerator % winning_pool;

    // Constraint: numerator = expected_payout * winning_pool + remainder
    signal reconstructed <== expected_payout * winning_pool + remainder;
    reconstructed === numerator;

    // Constraint: remainder < winning_pool (valid division)
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

    // ===== Constraint 5: Range checks =====

    // bet_amount > 0
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    // payout_amount > 0
    component payout_positive = GreaterThan(64);
    payout_positive.in[0] <== payout_amount;
    payout_positive.in[1] <== 0;
    payout_positive.out === 1;

    // payout_amount <= total_pool
    component payout_bounded = LessEqThan(64);
    payout_bounded.in[0] <== payout_amount;
    payout_bounded.in[1] <== total_pool;
    payout_bounded.out === 1;

    // winning_pool > 0 (prevent division by zero)
    component pool_positive = GreaterThan(64);
    pool_positive.in[0] <== winning_pool;
    pool_positive.in[1] <== 0;
    pool_positive.out === 1;

    // bet_side is binary (0 or 1)
    bet_side * (bet_side - 1) === 0;

    // resolution is binary (0 or 1)
    resolution * (resolution - 1) === 0;

    // Bind market_id and timestamp to avoid unused warnings.
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
