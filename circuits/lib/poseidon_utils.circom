pragma circom 2.1.6;

// ============================================================================
// Poseidon Hash Utilities for VERIFY_GROTH16
// ============================================================================

include "circomlib/circuits/poseidon.circom";

// Hash de array de longitud variable (hasta max_len elementos)
template PoseidonArray(max_len) {
    signal input in[max_len];
    signal input len;  // Número real de elementos a hashear
    signal output out;

    // Usamos un esquema de hash incremental
    // H(x1, x2, ..., xn) = Poseidon(Poseidon(...Poseidon(x1, x2), x3), ..., xn)

    signal hashes[max_len];

    // Primer hash: h0 = Poseidon(in[0], 0)
    component first_hash = Poseidon(2);
    first_hash.inputs[0] <== in[0];
    first_hash.inputs[1] <== 0;
    hashes[0] <== first_hash.out;

    // Hashes sucesivos
    component hashers[max_len - 1];
    component selectors[max_len - 1];

    for (var i = 1; i < max_len; i++) {
        hashers[i - 1] = Poseidon(2);
        hashers[i - 1].inputs[0] <== hashes[i - 1];
        hashers[i - 1].inputs[1] <== in[i];

        // Solo incluir si i < len
        selectors[i - 1] = Selector();
        selectors[i - 1].condition <== i < len ? 1 : 0;
        selectors[i - 1].in_true <== hashers[i - 1].out;
        selectors[i - 1].in_false <== hashes[i - 1];

        hashes[i] <== selectors[i - 1].out;
    }

    out <== hashes[max_len - 1];
}

// Selector simple: if condition then in_true else in_false
template Selector() {
    signal input condition;
    signal input in_true;
    signal input in_false;
    signal output out;

    out <== condition * (in_true - in_false) + in_false;
}

// Hash de Verification Key Groth16
// VK consiste en:
// - alpha: G1 point (2 elementos)
// - beta: G2 point (4 elementos)
// - gamma: G2 point (4 elementos)
// - delta: G2 point (4 elementos)
// - IC: array de G1 points (2 * num_ic elementos)
template PoseidonVK(max_ic) {
    signal input alpha[2];
    signal input beta[2][2];
    signal input gamma[2][2];
    signal input delta[2][2];
    signal input ic[max_ic][2];
    signal input num_ic;
    signal output out;

    // Concatenar todo en un array plano y hashear
    // Total: 2 + 4 + 4 + 4 + 2*max_ic = 14 + 2*max_ic elementos

    var total_elements = 14 + 2 * max_ic;

    signal flat[14 + 2 * max_ic];

    // Alpha
    flat[0] <== alpha[0];
    flat[1] <== alpha[1];

    // Beta (G2 = 4 elementos)
    flat[2] <== beta[0][0];
    flat[3] <== beta[0][1];
    flat[4] <== beta[1][0];
    flat[5] <== beta[1][1];

    // Gamma
    flat[6] <== gamma[0][0];
    flat[7] <== gamma[0][1];
    flat[8] <== gamma[1][0];
    flat[9] <== gamma[1][1];

    // Delta
    flat[10] <== delta[0][0];
    flat[11] <== delta[0][1];
    flat[12] <== delta[1][0];
    flat[13] <== delta[1][1];

    // IC points
    for (var i = 0; i < max_ic; i++) {
        flat[14 + 2*i] <== ic[i][0];
        flat[14 + 2*i + 1] <== ic[i][1];
    }

    // Hash todo
    component hasher = PoseidonArray(14 + 2 * max_ic);
    hasher.in <== flat;
    hasher.len <== 14 + 2 * num_ic;  // Solo hashear los IC que existen

    out <== hasher.out;
}

// Hash de un punto G1
template PoseidonG1() {
    signal input p[2];
    signal output out;

    component hasher = Poseidon(2);
    hasher.inputs[0] <== p[0];
    hasher.inputs[1] <== p[1];

    out <== hasher.out;
}

// Hash de un punto G2
template PoseidonG2() {
    signal input p[2][2];
    signal output out;

    component hasher = Poseidon(4);
    hasher.inputs[0] <== p[0][0];
    hasher.inputs[1] <== p[0][1];
    hasher.inputs[2] <== p[1][0];
    hasher.inputs[3] <== p[1][1];

    out <== hasher.out;
}

// Hash del proof P1 (A, B, C)
template PoseidonProof() {
    signal input A[2];      // G1
    signal input B[2][2];   // G2
    signal input C[2];      // G1

    signal output out;

    // Total: 2 + 4 + 2 = 8 elementos
    component hasher = Poseidon(8);
    hasher.inputs[0] <== A[0];
    hasher.inputs[1] <== A[1];
    hasher.inputs[2] <== B[0][0];
    hasher.inputs[3] <== B[0][1];
    hasher.inputs[4] <== B[1][0];
    hasher.inputs[5] <== B[1][1];
    hasher.inputs[6] <== C[0];
    hasher.inputs[7] <== C[1];

    out <== hasher.out;
}
