pragma circom 2.1.6;

// ============================================================================
// BN254 (alt_bn128) Field and Curve Operations
// ============================================================================
//
// Curva: y² = x³ + 3 sobre Fp
// p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
// r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
//
// G1: puntos en la curva sobre Fp
// G2: puntos en la curva sobre Fp2 (extensión cuadrática)
// ============================================================================

include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

// Constantes de la curva BN254
function bn254_p() {
    return 21888242871839275222246405745257275088696311157297823662689037894645226208583;
}

function bn254_r() {
    return 21888242871839275222246405745257275088548364400416034343698204186575808495617;
}

// Generador G1
function bn254_g1_x() {
    return 1;
}

function bn254_g1_y() {
    return 2;
}

// ============================================================================
// G1 Point Operations
// ============================================================================

// Verificar que un punto está en la curva G1: y² = x³ + 3
template G1OnCurve() {
    signal input p[2];  // (x, y)

    signal x2 <== p[0] * p[0];
    signal x3 <== x2 * p[0];
    signal y2 <== p[1] * p[1];

    // y² = x³ + 3
    y2 === x3 + 3;
}

// Negación de punto G1: -P = (x, -y)
template G1Neg() {
    signal input p[2];
    signal output out[2];

    out[0] <== p[0];
    // En el campo, -y = p - y
    out[1] <== bn254_p() - p[1];
}

// Suma de puntos G1 (caso general, P ≠ Q, P ≠ -Q)
// Fórmula:
//   λ = (y2 - y1) / (x2 - x1)
//   x3 = λ² - x1 - x2
//   y3 = λ(x1 - x3) - y1
template G1Add() {
    signal input p1[2];
    signal input p2[2];
    signal output out[2];

    // Calcular λ = (y2 - y1) / (x2 - x1)
    signal dy <== p2[1] - p1[1];
    signal dx <== p2[0] - p1[0];

    // dx_inv es el inverso de dx (witness)
    signal dx_inv;
    dx_inv <-- 1 / dx;
    dx * dx_inv === 1;

    signal lambda <== dy * dx_inv;

    // x3 = λ² - x1 - x2
    signal lambda2 <== lambda * lambda;
    out[0] <== lambda2 - p1[0] - p2[0];

    // y3 = λ(x1 - x3) - y1
    signal x1_minus_x3 <== p1[0] - out[0];
    out[1] <== lambda * x1_minus_x3 - p1[1];
}

// Duplicación de punto G1: 2P
// Fórmula:
//   λ = 3x² / 2y
//   x3 = λ² - 2x
//   y3 = λ(x - x3) - y
template G1Double() {
    signal input p[2];
    signal output out[2];

    signal x2 <== p[0] * p[0];
    signal numerator <== 3 * x2;
    signal denominator <== 2 * p[1];

    signal denom_inv;
    denom_inv <-- 1 / denominator;
    denominator * denom_inv === 1;

    signal lambda <== numerator * denom_inv;

    signal lambda2 <== lambda * lambda;
    out[0] <== lambda2 - 2 * p[0];

    signal x_minus_x3 <== p[0] - out[0];
    out[1] <== lambda * x_minus_x3 - p[1];
}

// Multiplicación escalar: k * P usando double-and-add
// n_bits: número de bits del escalar
template G1ScalarMul(n_bits) {
    signal input scalar;
    signal input point[2];
    signal output out[2];

    // Descomponer escalar en bits
    component bits = Num2Bits(n_bits);
    bits.in <== scalar;

    // Acumulador y punto corriente
    signal acc[n_bits + 1][2];
    signal running[n_bits + 1][2];

    // Inicializar: acc = punto al infinito (representado como (0,0) por simplicidad)
    // running = point
    acc[0][0] <== 0;
    acc[0][1] <== 0;
    running[0][0] <== point[0];
    running[0][1] <== point[1];

    component adders[n_bits];
    component doublers[n_bits];
    component muxX[n_bits];
    component muxY[n_bits];

    for (var i = 0; i < n_bits; i++) {
        // Si bit[i] = 1, acc = acc + running
        adders[i] = G1AddOrIdentity();
        adders[i].p1 <== acc[i];
        adders[i].p2 <== running[i];
        adders[i].selector <== bits.out[i];

        acc[i + 1][0] <== adders[i].out[0];
        acc[i + 1][1] <== adders[i].out[1];

        // running = 2 * running
        doublers[i] = G1Double();
        doublers[i].p <== running[i];
        running[i + 1][0] <== doublers[i].out[0];
        running[i + 1][1] <== doublers[i].out[1];
    }

    out[0] <== acc[n_bits][0];
    out[1] <== acc[n_bits][1];
}

// Suma condicional: si selector = 1, out = p1 + p2; si selector = 0, out = p1
template G1AddOrIdentity() {
    signal input p1[2];
    signal input p2[2];
    signal input selector;  // 0 o 1

    signal output out[2];

    // Calcular suma real
    component adder = G1Add();
    adder.p1 <== p1;
    adder.p2 <== p2;

    // out = selector * (p1 + p2) + (1 - selector) * p1
    // Simplificado: out = p1 + selector * (adder.out - p1)
    out[0] <== selector * (adder.out[0] - p1[0]) + p1[0];
    out[1] <== selector * (adder.out[1] - p1[1]) + p1[1];
}

// Multi-scalar multiplication: ∑(scalars[i] * points[i])
template MultiScalarMulG1(max_points) {
    signal input scalars[max_points];
    signal input points[max_points][2];
    signal input num_points;  // Número real de puntos a procesar
    signal output out[2];

    // Calcular cada scalar * point
    component scalar_muls[max_points];
    for (var i = 0; i < max_points; i++) {
        scalar_muls[i] = G1ScalarMul(254);  // 254 bits para campo BN254
        scalar_muls[i].scalar <== scalars[i];
        scalar_muls[i].point <== points[i];
    }

    // Sumar todos los resultados
    signal partial_sums[max_points + 1][2];
    partial_sums[0][0] <== 0;
    partial_sums[0][1] <== 0;

    component adders[max_points];
    component enable_checks[max_points];
    for (var i = 0; i < max_points; i++) {
        // Verificar si i < num_points usando LessThan
        enable_checks[i] = LessThan(32);
        enable_checks[i].in[0] <== i;
        enable_checks[i].in[1] <== num_points;

        adders[i] = G1AddConditional();
        adders[i].p1 <== partial_sums[i];
        adders[i].p2 <== scalar_muls[i].out;
        adders[i].enable <== enable_checks[i].out;

        partial_sums[i + 1][0] <== adders[i].out[0];
        partial_sums[i + 1][1] <== adders[i].out[1];
    }

    out[0] <== partial_sums[max_points][0];
    out[1] <== partial_sums[max_points][1];
}

// Suma condicional con enable
template G1AddConditional() {
    signal input p1[2];
    signal input p2[2];
    signal input enable;  // 0 o 1
    signal output out[2];

    component adder = G1Add();
    adder.p1 <== p1;
    adder.p2 <== p2;

    // Si enable = 0, out = p1; si enable = 1, out = p1 + p2
    out[0] <== enable * (adder.out[0] - p1[0]) + p1[0];
    out[1] <== enable * (adder.out[1] - p1[1]) + p1[1];
}

// ============================================================================
// G2 Point Operations (sobre Fp2)
// ============================================================================

// Un punto G2 tiene coordenadas en Fp2
// Representamos cada coordenada como [c0, c1] donde el valor es c0 + c1*u
// donde u² = -1 (o u² + 1 = 0)

// Multiplicación en Fp2: (a0 + a1*u)(b0 + b1*u) = (a0*b0 - a1*b1) + (a0*b1 + a1*b0)*u
template Fp2Mul() {
    signal input a[2];  // a0 + a1*u
    signal input b[2];  // b0 + b1*u
    signal output out[2];

    signal a0b0 <== a[0] * b[0];
    signal a1b1 <== a[1] * b[1];
    signal a0b1 <== a[0] * b[1];
    signal a1b0 <== a[1] * b[0];

    out[0] <== a0b0 - a1b1;      // Parte real
    out[1] <== a0b1 + a1b0;      // Parte imaginaria
}

// Cuadrado en Fp2: (a + b*u)² = (a² - b²) + (2ab)*u
template Fp2Square() {
    signal input a[2];
    signal output out[2];

    signal a0_sq <== a[0] * a[0];
    signal a1_sq <== a[1] * a[1];
    signal a0a1 <== a[0] * a[1];

    out[0] <== a0_sq - a1_sq;
    out[1] <== 2 * a0a1;
}

// Negación en Fp2
template Fp2Neg() {
    signal input a[2];
    signal output out[2];

    out[0] <== -a[0];
    out[1] <== -a[1];
}

// Suma en Fp2
template Fp2Add() {
    signal input a[2];
    signal input b[2];
    signal output out[2];

    out[0] <== a[0] + b[0];
    out[1] <== a[1] + b[1];
}

// Resta en Fp2
template Fp2Sub() {
    signal input a[2];
    signal input b[2];
    signal output out[2];

    out[0] <== a[0] - b[0];
    out[1] <== a[1] - b[1];
}
