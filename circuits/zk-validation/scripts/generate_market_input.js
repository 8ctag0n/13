#!/usr/bin/env node
// Generate valid input for MarketBet circuit (Circuit 30)

const { buildPoseidon } = require("circomlibjs");

async function main() {
  const poseidon = await buildPoseidon();
  const F = poseidon.F;

  // Private inputs
  const bettor_wallet = BigInt(888);
  const bet_amount = BigInt(50); // betting 50 tokens
  const position = BigInt(1); // predicting outcome 1
  const blinding = BigInt(11111);

  // Public inputs
  const market_id = BigInt(100);
  const max_bet = BigInt(1000); // max allowed bet
  const timestamp = BigInt(1702400000);

  // Compute bet_commitment = Poseidon(bet_amount, position, blinding)
  const bet_commitment = poseidon([
    F.e(bet_amount),
    F.e(position),
    F.e(blinding)
  ]);

  const input = {
    // Public inputs
    market_id: market_id.toString(),
    bet_commitment: F.toString(bet_commitment),
    max_bet: max_bet.toString(),
    timestamp: timestamp.toString(),

    // Private inputs
    bettor_wallet: bettor_wallet.toString(),
    bet_amount: bet_amount.toString(),
    position: position.toString(),
    blinding: blinding.toString()
  };

  console.log(JSON.stringify(input, null, 2));
}

main().catch(console.error);
