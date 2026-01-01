/**
 * Attestation Helper para VERIFY_GROTH16
 *
 * Este módulo genera el `verification_witness` que el backend
 * debe proveer al circuito después de verificar P1.
 */

// @ts-ignore - circomlibjs types
import { buildPoseidon } from "circomlibjs";

export interface G1Point {
    x: bigint;
    y: bigint;
}

export interface G2Point {
    x: [bigint, bigint];
    y: [bigint, bigint];
}

export interface Groth16Proof {
    a: G1Point;
    b: G2Point;
    c: G1Point;
}

export interface VerificationKey {
    alpha: G1Point;
    beta: G2Point;
    gamma: G2Point;
    delta: G2Point;
    ic: G1Point[];
}

// BN254 field modulus
const BN254_P = 21888242871839275222246405745257275088696311157297823662689037894645226208583n;

let poseidon: any = null;

async function getPoseidon() {
    if (!poseidon) {
        poseidon = await buildPoseidon();
    }
    return poseidon;
}

/**
 * Calcula -P en G1 (negación del punto)
 */
export function negateG1(p: G1Point): G1Point {
    return {
        x: p.x,
        y: BN254_P - p.y
    };
}

/**
 * Calcula vk_x = IC[0] + Σ(publicInputs[i] · IC[i+1])
 *
 * NOTA: Esta es una versión simplificada que asume que el backend
 * ya tiene vk_x pre-calculado. En producción, usar una biblioteca
 * de curvas elípticas como @noble/curves.
 */
export function computeVkX(vk: VerificationKey, publicInputs: bigint[]): G1Point {
    // Placeholder - en producción usar MSM real
    // Por ahora retornamos IC[0] como aproximación
    return vk.ic[0];
}

/**
 * Genera el verification_witness para el modelo attestation.
 *
 * El witness es: Poseidon(hash1, alpha, vk_x, beta[0][0], delta[0][0], result)
 * donde hash1 = Poseidon(negA, B, C)
 *
 * @param proof - El proof P1 que fue verificado
 * @param vk - Verification key del circuito P1
 * @param vkX - El punto vk_x pre-calculado
 * @param verificationResult - true si P1 es válido, false si no
 */
export async function generateVerificationWitness(
    proof: Groth16Proof,
    vk: VerificationKey,
    vkX: G1Point,
    verificationResult: boolean
): Promise<bigint> {
    const poseidon = await getPoseidon();
    const F = poseidon.F;

    // Negar A
    const negA = negateG1(proof.a);

    // Hash 1: Poseidon(negA.x, negA.y, B[0][0], B[0][1], B[1][0], B[1][1], C.x, C.y)
    const hash1Inputs = [
        F.e(negA.x),
        F.e(negA.y),
        F.e(proof.b.x[0]),
        F.e(proof.b.x[1]),
        F.e(proof.b.y[0]),
        F.e(proof.b.y[1]),
        F.e(proof.c.x),
        F.e(proof.c.y)
    ];
    const hash1 = poseidon(hash1Inputs);

    // Hash 2: Poseidon(hash1, alpha.x, alpha.y, vkX.x, vkX.y, beta[0][0], delta[0][0], result)
    const hash2Inputs = [
        hash1,
        F.e(vk.alpha.x),
        F.e(vk.alpha.y),
        F.e(vkX.x),
        F.e(vkX.y),
        F.e(vk.beta.x[0]),
        F.e(vk.delta.x[0]),
        F.e(verificationResult ? 1n : 0n)
    ];
    const witness = poseidon(hash2Inputs);

    return F.toObject(witness);
}

/**
 * Verifica un proof Groth16 usando snarkjs y genera el witness de attestation.
 *
 * @param proof - Proof P1 en formato snarkjs
 * @param publicSignals - Public signals del proof
 * @param vk - Verification key en formato snarkjs
 * @returns El witness de attestation y el resultado de verificación
 */
export async function verifyAndAttest(
    proof: any,
    publicSignals: string[],
    vk: any
): Promise<{ witness: bigint; result: boolean; vkX: G1Point }> {
    // Importar snarkjs dinámicamente
    const snarkjs = await import("snarkjs");

    // Verificar el proof usando snarkjs (nativo, rápido)
    const result = await snarkjs.groth16.verify(vk, publicSignals, proof);

    // Convertir formatos
    const proofConverted: Groth16Proof = {
        a: { x: BigInt(proof.pi_a[0]), y: BigInt(proof.pi_a[1]) },
        b: {
            x: [BigInt(proof.pi_b[0][0]), BigInt(proof.pi_b[0][1])],
            y: [BigInt(proof.pi_b[1][0]), BigInt(proof.pi_b[1][1])]
        },
        c: { x: BigInt(proof.pi_c[0]), y: BigInt(proof.pi_c[1]) }
    };

    const vkConverted: VerificationKey = {
        alpha: { x: BigInt(vk.vk_alpha_1[0]), y: BigInt(vk.vk_alpha_1[1]) },
        beta: {
            x: [BigInt(vk.vk_beta_2[0][0]), BigInt(vk.vk_beta_2[0][1])],
            y: [BigInt(vk.vk_beta_2[1][0]), BigInt(vk.vk_beta_2[1][1])]
        },
        gamma: {
            x: [BigInt(vk.vk_gamma_2[0][0]), BigInt(vk.vk_gamma_2[0][1])],
            y: [BigInt(vk.vk_gamma_2[1][0]), BigInt(vk.vk_gamma_2[1][1])]
        },
        delta: {
            x: [BigInt(vk.vk_delta_2[0][0]), BigInt(vk.vk_delta_2[0][1])],
            y: [BigInt(vk.vk_delta_2[1][0]), BigInt(vk.vk_delta_2[1][1])]
        },
        ic: vk.IC.map((ic: string[]) => ({
            x: BigInt(ic[0]),
            y: BigInt(ic[1])
        }))
    };

    // Calcular vk_x (simplificado - usar IC[0] + public inputs * IC[1..])
    const publicInputsBigInt = publicSignals.map(s => BigInt(s));
    const vkX = computeVkX(vkConverted, publicInputsBigInt);

    // Generar witness de attestation
    const witness = await generateVerificationWitness(
        proofConverted,
        vkConverted,
        vkX,
        result
    );

    return { witness, result, vkX };
}

/**
 * Ejemplo de uso completo del flujo de attestation.
 */
export async function exampleUsage() {
    // 1. Backend recibe proof P1 del prover
    const proofP1 = {/* proof en formato snarkjs */};
    const publicSignals = ["123", "456"];
    const vk = {/* verification key en formato snarkjs */};

    // 2. Verificar y generar attestation
    // const { witness, result, vkX } = await verifyAndAttest(proofP1, publicSignals, vk);

    // 3. Generar input para VERIFY_GROTH16
    // const circuitInput = {
    //     vk_hash: computeVkHash(vk),
    //     public_inputs_hash: computePIHash(publicSignals),
    //     verification_result: result ? "1" : "0",
    //     proof_a: [...],
    //     // ... resto de inputs
    //     verification_witness: witness.toString()
    // };

    // 4. Generar P2 usando snarkjs
    // const { proof: p2, publicSignals: p2Signals } = await snarkjs.groth16.fullProve(
    //     circuitInput,
    //     "verify_groth16.wasm",
    //     "verify_groth16_final.zkey"
    // );

    // 5. Enviar P2 on-chain
    // await submitToSolana(p2, p2Signals);
}
