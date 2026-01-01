pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";

/// WithdrawPrivate - Circuit 43
/// Enables private withdrawals from user balance
///
/// Public inputs:
///   - old_balance_commitment: Poseidon(balance, nonce)
///   - new_balance_commitment: Poseidon(new_balance, new_nonce)
///   - withdraw_amount: Amount being withdrawn (public from transfer)
///
/// Private inputs:
///   - balance, new_balance, nonce, new_nonce

template WithdrawPrivate() {
    // Public inputs
    signal input old_balance_commitment;
    signal input new_balance_commitment;
    signal input withdraw_amount;

    // Private inputs
    signal input balance;
    signal input new_balance;
    signal input nonce;
    signal input new_nonce;

    // 1. Verify old_balance_commitment = Poseidon(balance, nonce)
    component old_comm_hasher = Poseidon(2);
    old_comm_hasher.inputs[0] <== balance;
    old_comm_hasher.inputs[1] <== nonce;
    old_comm_hasher.out === old_balance_commitment;

    // 2. Verify balance >= withdraw_amount
    component balance_sufficient = GreaterEqThan(64);
    balance_sufficient.in[0] <== balance;
    balance_sufficient.in[1] <== withdraw_amount;
    balance_sufficient.out === 1;

    // 3. Verify new_balance = balance - withdraw_amount
    new_balance === balance - withdraw_amount;

    // 4. Verify new_balance_commitment = Poseidon(new_balance, new_nonce)
    component new_comm_hasher = Poseidon(2);
    new_comm_hasher.inputs[0] <== new_balance;
    new_comm_hasher.inputs[1] <== new_nonce;
    new_comm_hasher.out === new_balance_commitment;

    // 5. Verify new_nonce = nonce + 1
    new_nonce === nonce + 1;

    // 6. Verify withdraw_amount > 0
    component withdraw_positive = GreaterThan(64);
    withdraw_positive.in[0] <== withdraw_amount;
    withdraw_positive.in[1] <== 0;
    withdraw_positive.out === 1;
}

component main {public [
    old_balance_commitment,
    new_balance_commitment,
    withdraw_amount
]} = WithdrawPrivate();
