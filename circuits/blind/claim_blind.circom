pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// ClaimBlind - Circuit 51
/// Proves ownership of winning bet and correct payout calculation
///
/// Public inputs:
///   - bet_secret_commitment: Hash of the original bet (from PositionV3)
///   - winning_pool: Decrypted winning pool total (after settle)
///   - losing_pool: Decrypted losing pool total (after settle)
///   - outcome: 0=YES won, 1=NO won
///   - claimed_payout: Amount being claimed
///   - market_id: Market identifier
///
/// Private inputs:
///   - bet_amount: Original bet amount
///   - bet_side: Side of the bet (0=YES, 1=NO)
///   - bet_blinding: Randomness for bet commitment
///
/// This circuit proves:
/// 1. Prover knows bet details matching bet_secret_commitment
/// 2. Bet was on the winning side
/// 3. Claimed payout is valid: payout <= bet_amount * total_pool / winning_pool

template ClaimBlind() {
    // Public inputs (6)
    signal input bet_secret_commitment;   // Poseidon(bet_amount, bet_side, blinding)
    signal input winning_pool;            // Decrypted after settle
    signal input losing_pool;             // Decrypted after settle
    signal input outcome;                 // 0=YES won, 1=NO won
    signal input claimed_payout;          // Amount user claims
    signal input market_id;               // Market identifier

    // Private inputs (3)
    signal input bet_amount;              // Secret: original bet
    signal input bet_side;                // Secret: 0=YES, 1=NO
    signal input bet_blinding;            // Secret: commitment randomness

    // 1. Verify bet_secret_commitment = Poseidon(bet_amount, bet_side, bet_blinding)
    component hash_bet = Poseidon(3);
    hash_bet.inputs[0] <== bet_amount;
    hash_bet.inputs[1] <== bet_side;
    hash_bet.inputs[2] <== bet_blinding;
    hash_bet.out === bet_secret_commitment;

    // 2. Verify bet_side is valid {0, 1}
    bet_side * (bet_side - 1) === 0;

    // 3. Verify outcome is valid {0, 1}
    outcome * (outcome - 1) === 0;

    // 4. Verify bet was on winning side
    // bet_side must equal outcome (both 0 or both 1)
    bet_side === outcome;

    // 5. Verify bet_amount > 0
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    // 6. Verify pools are valid (winning_pool > 0)
    component winning_positive = GreaterThan(64);
    winning_positive.in[0] <== winning_pool;
    winning_positive.in[1] <== 0;
    winning_positive.out === 1;

    // 7. Verify payout calculation is valid
    // Formula: max_payout = bet_amount * total_pool / winning_pool
    // Rearranged (no division in circom):
    // claimed_payout * winning_pool <= bet_amount * total_pool
    //
    // This proves user isn't claiming more than entitled
    signal total_pool;
    total_pool <== winning_pool + losing_pool;

    signal lhs;  // claimed_payout * winning_pool
    signal rhs;  // bet_amount * total_pool

    lhs <== claimed_payout * winning_pool;
    rhs <== bet_amount * total_pool;

    component payout_valid = LessEqThan(128);
    payout_valid.in[0] <== lhs;
    payout_valid.in[1] <== rhs;
    payout_valid.out === 1;

    // 8. Verify claimed_payout > 0 (user must claim something)
    component payout_positive = GreaterThan(64);
    payout_positive.in[0] <== claimed_payout;
    payout_positive.in[1] <== 0;
    payout_positive.out === 1;

    // 9. Verify claimed_payout >= bet_amount (winner gets at least their bet back)
    component payout_gte_bet = GreaterEqThan(64);
    payout_gte_bet.in[0] <== claimed_payout;
    payout_gte_bet.in[1] <== bet_amount;
    payout_gte_bet.out === 1;

    // 10. Bind market_id to prevent cross-market proof reuse
    signal market_bind;
    market_bind <== market_id * market_id;
}

component main {public [
    bet_secret_commitment,
    winning_pool,
    losing_pool,
    outcome,
    claimed_payout,
    market_id
]} = ClaimBlind();
