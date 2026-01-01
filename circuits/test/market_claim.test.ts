import { expect } from "chai";
import path from "path";
import { buildPoseidon } from "circomlibjs";

describe("MarketClaim Circuit", () => {
    let circuit: any;
    let poseidon: any;

    before(async () => {
        const circom_tester = require("circom_tester");
        circuit = await circom_tester.wasm(
            path.join(__dirname, "../market/market_claim.circom")
        );
        poseidon = await buildPoseidon();
    });

    function toField(value: bigint): bigint {
        return poseidon.F.toObject(value);
    }

    it("accepts a valid claim", async () => {
        const betAmount = 75n;
        const betSide = 1n;
        const blinding = 222n;
        const secret = 111n;
        const totalPool = 200n;
        const winningPool = 100n;
        const payoutAmount = 150n;

        const betCommitment = toField(poseidon([betAmount, betSide, blinding]));
        const nullifier = toField(poseidon([secret, betCommitment]));

        const input = {
            market_id: "1",
            nullifier: nullifier.toString(),
            payout_amount: payoutAmount.toString(),
            resolution: "1",
            total_pool: totalPool.toString(),
            winning_pool: winningPool.toString(),
            bet_commitment: betCommitment.toString(),
            timestamp: "1700000000",
            secret: secret.toString(),
            bet_amount: betAmount.toString(),
            bet_side: betSide.toString(),
            blinding: blinding.toString(),
        };

        await circuit.calculateWitness(input, true);
    });

    it("rejects a claim on the losing side", async () => {
        const betAmount = 10n;
        const betSide = 0n;
        const blinding = 333n;
        const secret = 444n;
        const totalPool = 100n;
        const winningPool = 60n;
        const payoutAmount = 16n;

        const betCommitment = toField(poseidon([betAmount, betSide, blinding]));
        const nullifier = toField(poseidon([secret, betCommitment]));

        const input = {
            market_id: "2",
            nullifier: nullifier.toString(),
            payout_amount: payoutAmount.toString(),
            resolution: "1",
            total_pool: totalPool.toString(),
            winning_pool: winningPool.toString(),
            bet_commitment: betCommitment.toString(),
            timestamp: "1700000001",
            secret: secret.toString(),
            bet_amount: betAmount.toString(),
            bet_side: betSide.toString(),
            blinding: blinding.toString(),
        };

        let threw = false;
        try {
            await circuit.calculateWitness(input, true);
        } catch {
            threw = true;
        }
        expect(threw).to.equal(true);
    });
});
