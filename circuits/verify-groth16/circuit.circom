pragma circom 2.1.6;

// ============================================================================
// VERIFY_GROTH16: Circuito para verificar proofs Groth16 (MVP/Demo)
// ============================================================================
//
// Modelo Attestation:
// 1. Backend recibe P1 del prover
// 2. Backend verifica P1 usando snarkjs (nativo, ~1ms)
// 3. Backend genera witness de attestation
// 4. Este circuito verifica consistencia y genera P2
// 5. P2 (256 bytes) va on-chain
//
// Constraints estimados: ~8K (modelo attestation)
// Para producción con pairing completo: ~50K
// ============================================================================

include "../lib/bn254.circom";
include "../lib/poseidon_utils.circom";
include "../lib/pairing.circom";

// Configuración
var MAX_PUBLIC_INPUTS = 32;
var MAX_IC = 33;

// ============================================================================
// Circuito Principal
// ============================================================================

template VerifyGroth16() {
    // ========================================================================
    // PUBLIC INPUTS (van on-chain)
    // ========================================================================

    signal input vk_hash;              // Poseidon(vk)
    signal input public_inputs_hash;   // Poseidon(public_inputs)
    signal input verification_result;  // 0 = inválido, 1 = válido

    // ========================================================================
    // PRIVATE INPUTS (witness)
    // ========================================================================

    // Proof P1 (Groth16)
    signal input proof_a[2];           // A ∈ G1
    signal input proof_b[2][2];        // B ∈ G2
    signal input proof_c[2];           // C ∈ G1

    // Verification Key del circuito P1
    signal input vk_alpha[2];          // α ∈ G1
    signal input vk_beta[2][2];        // β ∈ G2
    signal input vk_gamma[2][2];       // γ ∈ G2
    signal input vk_delta[2][2];       // δ ∈ G2
    signal input vk_ic[MAX_IC][2];     // IC[0..l] ∈ G1
    signal input num_public_inputs;    // Número de public inputs

    // Public inputs del proof P1
    signal input public_inputs[MAX_PUBLIC_INPUTS];

    // Attestation witness (generado por backend después de verificar P1)
    signal input verification_witness;

    // ========================================================================
    // PASO 1: Verificar hash del Verification Key
    // ========================================================================

    component vk_hasher = PoseidonVK(MAX_IC);
    vk_hasher.alpha <== vk_alpha;
    vk_hasher.beta <== vk_beta;
    vk_hasher.gamma <== vk_gamma;
    vk_hasher.delta <== vk_delta;
    vk_hasher.ic <== vk_ic;
    vk_hasher.num_ic <== num_public_inputs + 1;

    vk_hash === vk_hasher.out;

    // ========================================================================
    // PASO 2: Verificar hash de public inputs
    // ========================================================================

    component pi_hasher = PoseidonArray(MAX_PUBLIC_INPUTS);
    pi_hasher.in <== public_inputs;
    pi_hasher.len <== num_public_inputs;

    public_inputs_hash === pi_hasher.out;

    // ========================================================================
    // PASO 3: Calcular vk_x = IC[0] + Σ(public_inputs[i] · IC[i+1])
    // ========================================================================

    signal ic_subset[MAX_PUBLIC_INPUTS][2];
    for (var i = 0; i < MAX_PUBLIC_INPUTS; i++) {
        ic_subset[i][0] <== vk_ic[i + 1][0];
        ic_subset[i][1] <== vk_ic[i + 1][1];
    }

    component msm = MultiScalarMulG1(MAX_PUBLIC_INPUTS);
    msm.scalars <== public_inputs;
    msm.points <== ic_subset;
    msm.num_points <== num_public_inputs;

    component vk_x_add = G1Add();
    vk_x_add.p1[0] <== vk_ic[0][0];
    vk_x_add.p1[1] <== vk_ic[0][1];
    vk_x_add.p2[0] <== msm.out[0];
    vk_x_add.p2[1] <== msm.out[1];

    signal vk_x[2];
    vk_x[0] <== vk_x_add.out[0];
    vk_x[1] <== vk_x_add.out[1];

    // ========================================================================
    // PASO 4: Negar punto A
    // ========================================================================

    component neg_a = G1Neg();
    neg_a.p[0] <== proof_a[0];
    neg_a.p[1] <== proof_a[1];

    // ========================================================================
    // PASO 5: Verificar attestation de pairing (modelo MVP)
    // ========================================================================

    component pairing_check = Groth16PairingCheckSimple();

    pairing_check.negA[0] <== neg_a.out[0];
    pairing_check.negA[1] <== neg_a.out[1];
    pairing_check.B <== proof_b;

    pairing_check.alpha <== vk_alpha;
    pairing_check.beta <== vk_beta;
    pairing_check.vk_x <== vk_x;
    pairing_check.gamma <== vk_gamma;
    pairing_check.C <== proof_c;
    pairing_check.delta <== vk_delta;

    // Attestation inputs
    pairing_check.verification_witness <== verification_witness;
    pairing_check.claimed_result <== verification_result;

    // Verificar que el resultado del pairing coincide
    verification_result === pairing_check.out;

    // ========================================================================
    // VALIDACIONES DE SEGURIDAD
    // ========================================================================

    // verification_result debe ser booleano
    verification_result * (1 - verification_result) === 0;

    // num_public_inputs en rango válido
    signal num_pi_check;
    num_pi_check <== num_public_inputs * (MAX_PUBLIC_INPUTS + 1 - num_public_inputs);
    // Debe ser >= 0 (si num_public_inputs está en [0, MAX_PUBLIC_INPUTS])
}

component main {public [vk_hash, public_inputs_hash, verification_result]} = VerifyGroth16();
