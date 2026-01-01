pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// PlaceBetPrivate - Circuit 40
/// Enables private betting with hidden balance tracking
///
/// Public inputs:
///   - old_balance_commitment: Poseidon(balance, nonce) - current state
///   - new_balance_commitment: Poseidon(new_balance, new_nonce) - updated state
///   - bet_commitment: Poseidon(bet_amount, side, secret) - for Position PDA
///   - market_id: Market identifier
///   - max_bet: Maximum allowed bet amount
///
/// Private inputs:
///   - balance, new_balance, bet_amount, side, secret, nonce, new_nonce

template PlaceBetPrivate() {
    // Public inputs
    signal input old_balance_commitment;
    signal input new_balance_commitment;
    signal input bet_commitment;
    signal input market_id;
    signal input max_bet;

    // Private inputs
    signal input balance;
    signal input new_balance;
    signal input bet_amount;
    signal input side;
    signal input secret;
    signal input nonce;
    signal input new_nonce;

    // 1. Verify old_balance_commitment = Poseidon(balance, nonce)
    component old_comm_hasher = Poseidon(2);
    old_comm_hasher.inputs[0] <== balance;
    old_comm_hasher.inputs[1] <== nonce;
    old_comm_hasher.out === old_balance_commitment;

    // 2. Verify new_balance_commitment = Poseidon(new_balance, new_nonce)
    component new_comm_hasher = Poseidon(2);
    new_comm_hasher.inputs[0] <== new_balance;
    new_comm_hasher.inputs[1] <== new_nonce;
    new_comm_hasher.out === new_balance_commitment;

    // 3. Verify bet_commitment = Poseidon(bet_amount, side, secret)
    component bet_comm_hasher = Poseidon(3);
    bet_comm_hasher.inputs[0] <== bet_amount;
    bet_comm_hasher.inputs[1] <== side;
    bet_comm_hasher.inputs[2] <== secret;
    bet_comm_hasher.out === bet_commitment;

    // 4. Verify new_balance = balance - bet_amount
    new_balance === balance - bet_amount;

    // 5. Verify balance >= bet_amount (no negative balance)
    component balance_sufficient = GreaterEqThan(64);
    balance_sufficient.in[0] <== balance;
    balance_sufficient.in[1] <== bet_amount;
    balance_sufficient.out === 1;

    // 6. Verify bet_amount > 0
    component bet_positive = GreaterThan(64);
    bet_positive.in[0] <== bet_amount;
    bet_positive.in[1] <== 0;
    bet_positive.out === 1;

    // 7. Verify bet_amount <= max_bet
    component bet_within_max = LessEqThan(64);
    bet_within_max.in[0] <== bet_amount;
    bet_within_max.in[1] <== max_bet;
    bet_within_max.out === 1;

    // 8. Verify side in {0, 1}
    side * (side - 1) === 0;

    // 9. Verify new_nonce = nonce + 1
    new_nonce === nonce + 1;

    // 10. Bind market_id to prevent proof reuse
    signal market_bind;
    market_bind <== market_id * market_id;
}

component main {public [
    old_balance_commitment,
    new_balance_commitment,
    bet_commitment,
    market_id,
    max_bet
]} = PlaceBetPrivate();
