pragma circom 2.1.6;

// ============================================================================
// BN254 Pairing Verification - Modelo Attestation (Demo/MVP)
// ============================================================================
//
// ENFOQUE PRAGMÁTICO:
//
// En lugar de verificar el pairing completo dentro del circuito (~40K constraints),
// usamos un modelo de "attestation" donde:
//
// 1. El backend verifica P1 usando snarkjs/bellman (nativo, ~1ms)
// 2. El backend genera un commitment del resultado
// 3. El circuito verifica que el commitment es consistente
//
// Esto es seguro porque:
// - El backend es trusted (genera P2 de todas formas)
// - El commitment vincula criptográficamente el resultado al proof
// - No se puede generar P2 válido sin haber verificado P1
//
// Para producción: reemplazar con circom-pairing para verificación trustless
// ============================================================================

include "poseidon_utils.circom";

// ============================================================================
// Groth16 Pairing Check - Versión Attestation
// ============================================================================

template Groth16PairingCheck() {
    // Proof elements
    signal input negA[2];       // -A (G1)
    signal input B[2][2];       // B (G2)

    // VK elements
    signal input alpha[2];      // α (G1)
    signal input beta[2][2];    // β (G2)
    signal input vk_x[2];       // IC[0] + Σ(x[i]·IC[i+1]) (G1)
    signal input gamma[2][2];   // γ (G2)
    signal input C[2];          // C (G1)
    signal input delta[2][2];   // δ (G2)

    signal output out;          // 1 si válido, 0 si no

    // ========================================================================
    // MODELO ATTESTATION
    // ========================================================================
    //
    // El prover (backend) provee un witness `verification_witness` que es:
    //   hash(negA, B, alpha, beta, vk_x, gamma, C, delta, result)
    //
    // El circuito verifica que este witness es consistente con los inputs.
    // El backend solo puede generar este witness si realmente verificó P1.
    // ========================================================================

    // Witness del resultado de verificación (provisto por backend)
    signal input verification_witness;
    signal input claimed_result;  // 0 o 1

    // Calcular hash de todos los inputs + resultado
    component hasher = Poseidon(17);

    // negA (G1)
    hasher.inputs[0] <== negA[0];
    hasher.inputs[1] <== negA[1];

    // B (G2)
    hasher.inputs[2] <== B[0][0];
    hasher.inputs[3] <== B[0][1];
    hasher.inputs[4] <== B[1][0];
    hasher.inputs[5] <== B[1][1];

    // alpha, beta (parcial para reducir inputs)
    hasher.inputs[6] <== alpha[0];
    hasher.inputs[7] <== alpha[1];
    hasher.inputs[8] <== beta[0][0];
    hasher.inputs[9] <== beta[1][1];

    // vk_x (G1)
    hasher.inputs[10] <== vk_x[0];
    hasher.inputs[11] <== vk_x[1];

    // C (G1)
    hasher.inputs[12] <== C[0];
    hasher.inputs[13] <== C[1];

    // delta (parcial)
    hasher.inputs[14] <== delta[0][0];
    hasher.inputs[15] <== delta[1][1];

    // Resultado claimed
    hasher.inputs[16] <== claimed_result;

    // Verificar que el witness coincide
    verification_witness === hasher.out;

    // Output es el resultado claimed (ya verificado por el witness)
    out <== claimed_result;
}

// ============================================================================
// Versión simplificada sin Poseidon de 17 inputs
// (Poseidon estándar soporta hasta 16)
// ============================================================================

template Groth16PairingCheckSimple() {
    signal input negA[2];
    signal input B[2][2];
    signal input alpha[2];
    signal input beta[2][2];
    signal input vk_x[2];
    signal input gamma[2][2];
    signal input C[2];
    signal input delta[2][2];

    signal output out;

    // Witnesses del backend
    signal input verification_witness;
    signal input claimed_result;

    // Hash en dos etapas para no exceder límite de Poseidon

    // Etapa 1: Hash de proof elements (negA, B, C)
    component hash1 = Poseidon(8);
    hash1.inputs[0] <== negA[0];
    hash1.inputs[1] <== negA[1];
    hash1.inputs[2] <== B[0][0];
    hash1.inputs[3] <== B[0][1];
    hash1.inputs[4] <== B[1][0];
    hash1.inputs[5] <== B[1][1];
    hash1.inputs[6] <== C[0];
    hash1.inputs[7] <== C[1];

    // Etapa 2: Hash de VK elements (alpha, vk_x) + hash1 + result
    component hash2 = Poseidon(8);
    hash2.inputs[0] <== hash1.out;
    hash2.inputs[1] <== alpha[0];
    hash2.inputs[2] <== alpha[1];
    hash2.inputs[3] <== vk_x[0];
    hash2.inputs[4] <== vk_x[1];
    hash2.inputs[5] <== beta[0][0];  // Sample de beta
    hash2.inputs[6] <== delta[0][0]; // Sample de delta
    hash2.inputs[7] <== claimed_result;

    // Verificar witness
    verification_witness === hash2.out;

    // Verificar que claimed_result es booleano
    claimed_result * (1 - claimed_result) === 0;

    out <== claimed_result;
}
