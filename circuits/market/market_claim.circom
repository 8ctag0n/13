pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";

/// MarketClaim - Circuit 32
/// Claim winnings from prediction markets
/// Proves:
/// 1. Original bet commitment matches
/// 2. Position matches winning outcome
/// 3. Claim nullifier prevents double-claim
///
/// Public inputs:
///   - market_id: Market identifier
///   - winning_outcome: Resolved outcome
///   - bet_commitment: Original bet commitment
///   - claim_nullifier: Hash(nullifier_secret, market_id)
///   - payout_amount: Calculated winnings
///
/// Private inputs:
///   - bettor_wallet: Wallet address
///   - bet_amount: Original bet amount
///   - position: Position that was bet
///   - blinding: Same blinding from bet
///   - nullifier_secret: Secret for nullifier

template MarketClaim() {
    // Public inputs
    signal input market_id;
    signal input winning_outcome;
    signal input bet_commitment;
    signal input claim_nullifier;
    signal input payout_amount;

    // Private inputs
    signal input bettor_wallet;
    signal input bet_amount;
    signal input position;
    signal input blinding;
    signal input nullifier_secret;

    // ============================================
    // 1. Verify bet_commitment matches original bet
    // ============================================
    component commitment_hasher = Poseidon(3);
    commitment_hasher.inputs[0] <== bet_amount;
    commitment_hasher.inputs[1] <== position;
    commitment_hasher.inputs[2] <== blinding;
    commitment_hasher.out === bet_commitment;

    // ============================================
    // 2. Verify position == winning_outcome
    // ============================================
    component outcome_check = IsEqual();
    outcome_check.in[0] <== position;
    outcome_check.in[1] <== winning_outcome;
    outcome_check.out === 1;

    // ============================================
    // 3. Verify claim_nullifier = Hash(nullifier_secret, market_id)
    // ============================================
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== nullifier_secret;
    nullifier_hasher.inputs[1] <== market_id;
    nullifier_hasher.out === claim_nullifier;

    // ============================================
    // 4. Prevent unused signal warnings
    // ============================================
    signal wallet_check;
    wallet_check <== bettor_wallet * bettor_wallet;

    signal payout_check;
    payout_check <== payout_amount * payout_amount;
}

component main {public [market_id, winning_outcome, bet_commitment, claim_nullifier, payout_amount]} = MarketClaim();
