pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";

/// MarketBet - Circuit 30
/// Enables private betting in prediction markets
/// Proves:
/// 1. bet_amount <= max_bet
/// 2. bet_commitment is correctly computed
///
/// Public inputs:
///   - market_id: Unique market identifier
///   - bet_commitment: Hash(amount, position, blinding)
///   - max_bet: Maximum allowed bet
///   - timestamp: Bet placement time
///
/// Private inputs:
///   - bettor_wallet: Wallet address (hidden)
///   - bet_amount: Actual bet amount
///   - position: Outcome prediction (0, 1, etc.)
///   - blinding: Random blinding factor

template MarketBet() {
    // Public inputs (must match Rust struct MarketBetPublicInputs)
    signal input market_id;
    signal input bet_commitment;
    signal input max_bet;
    signal input timestamp;

    // Private inputs
    signal input bettor_wallet;
    signal input bet_amount;
    signal input position;
    signal input blinding;

    // ============================================
    // 1. Verify bet_amount <= max_bet
    // ============================================
    component amount_check = LessEqThan(64);
    amount_check.in[0] <== bet_amount;
    amount_check.in[1] <== max_bet;
    amount_check.out === 1;

    // ============================================
    // 2. Verify bet_commitment = Hash(amount, position, blinding)
    // ============================================
    component commitment_hasher = Poseidon(3);
    commitment_hasher.inputs[0] <== bet_amount;
    commitment_hasher.inputs[1] <== position;
    commitment_hasher.inputs[2] <== blinding;
    commitment_hasher.out === bet_commitment;

    // ============================================
    // 3. Prevent unused signal warnings
    // ============================================
    signal wallet_check;
    wallet_check <== bettor_wallet * bettor_wallet;

    signal market_check;
    market_check <== market_id * market_id;

    signal timestamp_check;
    timestamp_check <== timestamp * timestamp;
}

component main {public [market_id, bet_commitment, max_bet, timestamp]} = MarketBet();
