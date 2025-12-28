pragma circom 2.1.6;

include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/comparators.circom";
include "../node_modules/circomlib/circuits/gates.circom";

/// DepositPrivate - Circuit 42
/// Enables private deposits to user balance
///
/// Public inputs:
///   - old_balance_commitment: Poseidon(balance, nonce) - 0 if first deposit
///   - new_balance_commitment: Poseidon(new_balance, new_nonce)
///   - deposit_amount: Amount being deposited (public from transfer)
///
/// Private inputs:
///   - balance, new_balance, nonce, new_nonce

template DepositPrivate() {
    // Public inputs
    signal input old_balance_commitment;
    signal input new_balance_commitment;
    signal input deposit_amount;

    // Private inputs
    signal input balance;
    signal input new_balance;
    signal input nonce;
    signal input new_nonce;

    // 1. Compute expected old commitment
    component old_comm_hasher = Poseidon(2);
    old_comm_hasher.inputs[0] <== balance;
    old_comm_hasher.inputs[1] <== nonce;

    // Check if first deposit (balance == 0 AND nonce == 0)
    component balance_is_zero = IsZero();
    balance_is_zero.in <== balance;

    component nonce_is_zero = IsZero();
    nonce_is_zero.in <== nonce;

    signal is_first_deposit <== balance_is_zero.out * nonce_is_zero.out;

    // For first deposit, old_balance_commitment must be 0
    // For existing balance, old_balance_commitment must match hash
    component old_comm_is_zero = IsZero();
    old_comm_is_zero.in <== old_balance_commitment;

    // Verify: (is_first_deposit AND old_comm_is_zero) OR (NOT first_deposit AND hash matches)
    signal first_deposit_valid <== is_first_deposit * old_comm_is_zero.out;

    component commitment_matches = IsEqual();
    commitment_matches.in[0] <== old_comm_hasher.out;
    commitment_matches.in[1] <== old_balance_commitment;

    signal not_first_deposit <== 1 - is_first_deposit;
    signal existing_valid <== not_first_deposit * commitment_matches.out;

    signal valid <== first_deposit_valid + existing_valid;
    valid === 1;

    // 2. Verify new_balance = balance + deposit_amount
    new_balance === balance + deposit_amount;

    // 3. Verify new_balance_commitment = Poseidon(new_balance, new_nonce)
    component new_comm_hasher = Poseidon(2);
    new_comm_hasher.inputs[0] <== new_balance;
    new_comm_hasher.inputs[1] <== new_nonce;
    new_comm_hasher.out === new_balance_commitment;

    // 4. Verify nonce increments
    new_nonce === nonce + 1;

    // 5. Verify deposit_amount > 0
    component deposit_positive = GreaterThan(64);
    deposit_positive.in[0] <== deposit_amount;
    deposit_positive.in[1] <== 0;
    deposit_positive.out === 1;
}

component main {public [
    old_balance_commitment,
    new_balance_commitment,
    deposit_amount
]} = DepositPrivate();
