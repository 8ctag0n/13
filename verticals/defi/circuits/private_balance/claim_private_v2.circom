pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// ClaimPrivateV2 - Circuit 34
/// Simplified claim for V2 equitative payout model
///
/// V2 Payout Model:
///   payout = vault_total / winner_count (calculated on-chain, not in circuit)
///   Privacy > Proportional fairness. V3 with FHE will add proportional payouts.
///
/// Public inputs (138 bytes total):
///   - bet_commitment: 32 bytes (from PositionV2 PDA)
///   - resolution: 1 byte (winning side: 0=NO, 1=YES)
///   - nullifier_hash: 32 bytes (for Nullifier PDA, prevents double-claim)
///   - bet_amount: 8 bytes (revealed for reference, not used in payout calc)
///   - bet_side: 1 byte (revealed to verify winner)
///   - old_balance_commitment: 32 bytes
///   - new_balance_commitment: 32 bytes
///
/// Private inputs:
///   - secret: the secret used to create bet_commitment
///   - balance: current private balance
///   - new_balance: balance after claiming payout
///   - nonce: current balance nonce
///   - new_nonce: new balance nonce (= nonce + 1)
///   - payout_amount: the payout (passed from on-chain calculation)
///
/// The circuit verifies:
///   1. User knows secret for bet_commitment
///   2. bet_side matches resolution (won the bet)
///   3. nullifier_hash is correctly computed
///   4. Balance state transition is valid

template ClaimPrivateV2() {
    // Public inputs (order matches 138-byte layout)
    signal input bet_commitment;      // 32 bytes - field element
    signal input resolution;          // 1 byte - 0 or 1
    signal input nullifier_hash;      // 32 bytes - field element
    signal input bet_amount;          // 8 bytes - revealed amount
    signal input bet_side;            // 1 byte - revealed side
    signal input old_balance_commitment;  // 32 bytes
    signal input new_balance_commitment;  // 32 bytes

    // Private inputs
    signal input secret;
    signal input balance;
    signal input new_balance;
    signal input nonce;
    signal input new_nonce;
    signal input payout_amount;  // From on-chain: vault_total / winner_count

    // 1. Verify bet_commitment = Poseidon(bet_amount, bet_side, secret)
    component bet_comm_hasher = Poseidon(3);
    bet_comm_hasher.inputs[0] <== bet_amount;
    bet_comm_hasher.inputs[1] <== bet_side;
    bet_comm_hasher.inputs[2] <== secret;
    bet_comm_hasher.out === bet_commitment;

    // 2. Verify bet_side == resolution (user bet on winning side)
    bet_side === resolution;

    // 3. Verify nullifier_hash = Poseidon(secret, bet_commitment)
    // This prevents double-claiming with the same bet
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== bet_commitment;
    nullifier_hasher.out === nullifier_hash;

    // 4. Verify new_balance = balance + payout_amount
    // payout_amount is trusted from on-chain calculation (vault_total / winner_count)
    new_balance === balance + payout_amount;

    // 5. Verify old_balance_commitment = Poseidon(balance, nonce)
    component old_comm_hasher = Poseidon(2);
    old_comm_hasher.inputs[0] <== balance;
    old_comm_hasher.inputs[1] <== nonce;
    old_comm_hasher.out === old_balance_commitment;

    // 6. Verify new_balance_commitment = Poseidon(new_balance, new_nonce)
    component new_comm_hasher = Poseidon(2);
    new_comm_hasher.inputs[0] <== new_balance;
    new_comm_hasher.inputs[1] <== new_nonce;
    new_comm_hasher.out === new_balance_commitment;

    // 7. Verify nonce increment
    new_nonce === nonce + 1;

    // 8. Range/validity checks
    // bet_amount must be positive
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    // payout_amount must be positive (at least 1 lamport)
    component payout_positive = GreaterThan(64);
    payout_positive.in[0] <== payout_amount;
    payout_positive.in[1] <== 0;
    payout_positive.out === 1;

    // Binary checks for side values
    bet_side * (bet_side - 1) === 0;      // bet_side is 0 or 1
    resolution * (resolution - 1) === 0;  // resolution is 0 or 1
}

component main {public [
    bet_commitment,
    resolution,
    nullifier_hash,
    bet_amount,
    bet_side,
    old_balance_commitment,
    new_balance_commitment
]} = ClaimPrivateV2();
