pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// PlaceBetBlind - Circuit 50
/// Enables fully private betting where pool totals remain hidden
///
/// Public inputs:
///   - pool_commitment_before: Hash of encrypted pool state before bet
///   - pool_commitment_after: Hash of encrypted pool state after bet
///   - bet_ciphertext_hash: Hash of FHE-encrypted bet amount
///   - market_id: Market identifier
///   - max_bet: Maximum allowed bet amount
///
/// Private inputs:
///   - bet_amount, bet_side
///   - pool_yes_before, pool_no_before (prover knows plaintext pools)
///   - blinding_before, blinding_after (commitment randomness)
///
/// This circuit proves:
/// 1. Prover knows the plaintext pool values matching pool_commitment_before
/// 2. Pool update is correct: pool_side_after = pool_side_before + bet_amount
/// 3. Updated pools match pool_commitment_after
/// 4. Bet amount is valid (> 0, <= max_bet)
/// 5. Bet side is valid (0 or 1)

template PlaceBetBlind() {
    // Public inputs (5)
    signal input pool_commitment_before;  // Poseidon(pool_yes, pool_no, blinding)
    signal input pool_commitment_after;
    signal input bet_ciphertext_hash;     // Hash of FHE ciphertext (off-chain)
    signal input market_id;
    signal input max_bet;

    // Private inputs (7)
    signal input bet_amount;              // Secret bet amount
    signal input bet_side;                // Secret side: 0=YES, 1=NO
    signal input pool_yes_before;         // Plaintext pool values (prover knows)
    signal input pool_no_before;
    signal input pool_yes_after;          // Updated pools
    signal input pool_no_after;
    signal input blinding_before;         // Commitment randomness
    signal input blinding_after;

    // 1. Verify pool_commitment_before = Poseidon(pool_yes_before, pool_no_before, blinding_before)
    component hash_before = Poseidon(3);
    hash_before.inputs[0] <== pool_yes_before;
    hash_before.inputs[1] <== pool_no_before;
    hash_before.inputs[2] <== blinding_before;
    hash_before.out === pool_commitment_before;

    // 2. Verify pool_commitment_after = Poseidon(pool_yes_after, pool_no_after, blinding_after)
    component hash_after = Poseidon(3);
    hash_after.inputs[0] <== pool_yes_after;
    hash_after.inputs[1] <== pool_no_after;
    hash_after.inputs[2] <== blinding_after;
    hash_after.out === pool_commitment_after;

    // 3. Verify pool update is correct based on bet_side
    // If bet_side = 0 (YES): pool_yes_after = pool_yes_before + bet_amount, pool_no_after = pool_no_before
    // If bet_side = 1 (NO):  pool_yes_after = pool_yes_before, pool_no_after = pool_no_before + bet_amount

    // pool_yes_after = pool_yes_before + bet_amount * (1 - bet_side)
    signal yes_delta;
    yes_delta <== bet_amount * (1 - bet_side);
    pool_yes_after === pool_yes_before + yes_delta;

    // pool_no_after = pool_no_before + bet_amount * bet_side
    signal no_delta;
    no_delta <== bet_amount * bet_side;
    pool_no_after === pool_no_before + no_delta;

    // 4. Verify bet_amount > 0
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    // 5. Verify bet_amount <= max_bet
    component bet_within_max = LessEqThan(64);
    bet_within_max.in[0] <== bet_amount;
    bet_within_max.in[1] <== max_bet;
    bet_within_max.out === 1;

    // 6. Verify bet_side in {0, 1}
    bet_side * (bet_side - 1) === 0;

    // 7. Bind bet_ciphertext_hash to prevent proof reuse
    // (Prover must use unique ciphertext per bet)
    signal ciphertext_bind;
    ciphertext_bind <== bet_ciphertext_hash * bet_ciphertext_hash;

    // 8. Bind market_id to prevent cross-market proof reuse
    signal market_bind;
    market_bind <== market_id * market_id;
}

component main {public [
    pool_commitment_before,
    pool_commitment_after,
    bet_ciphertext_hash,
    market_id,
    max_bet
]} = PlaceBetBlind();
