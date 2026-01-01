pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// ClaimPrivate - Circuit 41
/// Enables private claiming of winnings from settled markets
///
/// Public inputs:
///   - bet_commitment: from Position PDA
///   - nullifier_hash: for Nullifier PDA (prevents double-claim)
///   - old_balance_commitment, new_balance_commitment: balance state transition
///   - market_resolution: winning side (0 or 1)
///   - total_winning_pool, total_losing_pool: for payout calculation
///
/// Private inputs:
///   - bet_amount, side, secret, balance, new_balance, nonce, new_nonce

template ClaimPrivate() {
    // Public inputs
    signal input bet_commitment;
    signal input nullifier_hash;
    signal input old_balance_commitment;
    signal input new_balance_commitment;
    signal input market_resolution;
    signal input total_winning_pool;
    signal input total_losing_pool;

    // Private inputs
    signal input bet_amount;
    signal input side;
    signal input secret;
    signal input balance;
    signal input new_balance;
    signal input nonce;
    signal input new_nonce;

    // 1. Verify bet_commitment = Poseidon(bet_amount, side, secret)
    component bet_comm_hasher = Poseidon(3);
    bet_comm_hasher.inputs[0] <== bet_amount;
    bet_comm_hasher.inputs[1] <== side;
    bet_comm_hasher.inputs[2] <== secret;
    bet_comm_hasher.out === bet_commitment;

    // 2. Verify side == market_resolution (bet on winning side)
    side === market_resolution;

    // 3. Verify nullifier_hash = Poseidon(secret, bet_commitment)
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== bet_commitment;
    nullifier_hasher.out === nullifier_hash;

    // 4. Calculate payout = bet_amount + (bet_amount * total_losing_pool / total_winning_pool)
    signal numerator <== bet_amount * total_losing_pool;
    signal profit_share <-- numerator \ total_winning_pool;
    signal remainder <-- numerator % total_winning_pool;

    // Verify division
    signal reconstructed <== profit_share * total_winning_pool + remainder;
    reconstructed === numerator;

    // Verify remainder < total_winning_pool
    component remainder_lt = LessThan(64);
    remainder_lt.in[0] <== remainder;
    remainder_lt.in[1] <== total_winning_pool;
    remainder_lt.out === 1;

    signal payout <== bet_amount + profit_share;

    // 5. Verify new_balance = balance + payout
    new_balance === balance + payout;

    // 6. Verify old_balance_commitment = Poseidon(balance, nonce)
    component old_comm_hasher = Poseidon(2);
    old_comm_hasher.inputs[0] <== balance;
    old_comm_hasher.inputs[1] <== nonce;
    old_comm_hasher.out === old_balance_commitment;

    // 7. Verify new_balance_commitment = Poseidon(new_balance, new_nonce)
    component new_comm_hasher = Poseidon(2);
    new_comm_hasher.inputs[0] <== new_balance;
    new_comm_hasher.inputs[1] <== new_nonce;
    new_comm_hasher.out === new_balance_commitment;

    // 8. Verify new_nonce = nonce + 1
    new_nonce === nonce + 1;

    // 9. Range checks
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    component winning_pool_positive = GreaterThan(64);
    winning_pool_positive.in[0] <== total_winning_pool;
    winning_pool_positive.in[1] <== 0;
    winning_pool_positive.out === 1;

    // Binary checks
    side * (side - 1) === 0;
    market_resolution * (market_resolution - 1) === 0;
}

component main {public [
    bet_commitment,
    nullifier_hash,
    old_balance_commitment,
    new_balance_commitment,
    market_resolution,
    total_winning_pool,
    total_losing_pool
]} = ClaimPrivate();
