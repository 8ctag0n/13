# VERIFY_GROTH16 - Especificación Técnica

## Propósito

Circuito que prueba que una verificación Groth16 fue ejecutada correctamente.
Permite verificar proofs P1 (de cualquier tamaño) off-chain y generar un proof P2
compacto (256 bytes) que se verifica on-chain.

## Modelo de Verificación

### MVP/Demo: Modelo Attestation (~8K constraints)

El backend verifica P1 nativamente y genera un witness de attestation:

```
1. Backend recibe P1 del prover
2. Backend verifica P1 usando snarkjs (nativo, ~1ms)
3. Backend calcula: witness = Poseidon(proof_elements, vk_elements, result)
4. Circuito verifica que witness es consistente con inputs
5. Circuito genera P2 (256 bytes)
```

**Seguridad:** El backend es trusted (genera P2 de todas formas). El witness
vincula criptográficamente el resultado al proof - no se puede generar P2
válido sin haber verificado P1.

### Producción: Pairing Completo (~50K constraints)

Verificación trustless del pairing dentro del circuito usando circom-pairing.

## Matemática de Groth16

### Verificación Groth16 (lo que queremos probar dentro del circuito)

Dado:
- Verification Key: `vk = (α, β, γ, δ, IC[0..l])`
- Proof: `π = (A, B, C)` donde A,C ∈ G1, B ∈ G2
- Public Inputs: `x[1..l]`

La verificación Groth16 comprueba:

```
e(A, B) = e(α, β) · e(∑(x[i] · IC[i]), γ) · e(C, δ)
```

Donde:
- `e: G1 × G2 → GT` es el pairing bilineal (BN254)
- `∑(x[i] · IC[i])` es la combinación lineal de los public inputs con IC

### Curva BN254 (alt_bn128)

```
Parámetros:
- p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
- r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
- G1: y² = x³ + 3 (mod p)
- G2: y² = x³ + 3/(i+9) sobre Fp2
```

## Estructura del Circuito

### Public Inputs (3 elementos, 96 bytes on-chain)

```
signal input vk_hash;           // Poseidon(vk) - 32 bytes
signal input public_inputs_hash; // Poseidon(x[1..l]) - 32 bytes
signal input verification_result; // 0 o 1 - indica si P1 es válido
```

### Private Inputs (Witness)

```
// Proof P1 (256 bytes)
signal input proof_a[2];        // G1 point (x, y)
signal input proof_b[2][2];     // G2 point ((x0, x1), (y0, y1))
signal input proof_c[2];        // G1 point (x, y)

// Verification Key (variable, típicamente 500-2000 bytes)
signal input vk_alpha[2];       // G1
signal input vk_beta[2][2];     // G2
signal input vk_gamma[2][2];    // G2
signal input vk_delta[2][2];    // G2
signal input vk_ic[MAX_IC][2];  // Array de G1 points
signal input num_public_inputs; // Número de public inputs (≤ MAX_IC - 1)

// Public Inputs originales de P1
signal input public_inputs[MAX_PUBLIC_INPUTS];
```

### Constraints

#### 1. Verificar hash del VK (≈ 500 constraints)

```circom
component vk_hasher = PoseidonVK();
vk_hasher.alpha <== vk_alpha;
vk_hasher.beta <== vk_beta;
vk_hasher.gamma <== vk_gamma;
vk_hasher.delta <== vk_delta;
vk_hasher.ic <== vk_ic;
vk_hasher.num_ic <== num_public_inputs + 1;

vk_hash === vk_hasher.out;
```

#### 2. Verificar hash de public inputs (≈ 300 constraints)

```circom
component pi_hasher = PoseidonArray(MAX_PUBLIC_INPUTS);
pi_hasher.in <== public_inputs;
pi_hasher.len <== num_public_inputs;

public_inputs_hash === pi_hasher.out;
```

#### 3. Calcular combinación lineal IC (≈ 5K constraints)

```circom
// vk_x = IC[0] + ∑(public_inputs[i] * IC[i+1])
component msm = MultiScalarMulG1(MAX_PUBLIC_INPUTS);
msm.scalars <== public_inputs;
msm.points <== vk_ic[1:];
msm.num_points <== num_public_inputs;

component vk_x_add = G1Add();
vk_x_add.p1 <== vk_ic[0];
vk_x_add.p2 <== msm.out;
signal vk_x[2] <== vk_x_add.out;
```

#### 4. Verificar ecuación de pairing (≈ 40K constraints)

```circom
// e(A, B) =? e(α, β) · e(vk_x, γ) · e(C, δ)
//
// Equivalente a verificar:
// e(A, B) · e(-α, β) · e(-vk_x, γ) · e(-C, δ) = 1
//
// O usando la forma de producto:
// e(-A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1

component pairing_check = Groth16PairingCheck();
pairing_check.negA <== G1Neg(proof_a);
pairing_check.B <== proof_b;
pairing_check.alpha <== vk_alpha;
pairing_check.beta <== vk_beta;
pairing_check.vk_x <== vk_x;
pairing_check.gamma <== vk_gamma;
pairing_check.C <== proof_c;
pairing_check.delta <== vk_delta;

// pairing_check.out = 1 si pairing es válido, 0 si no
verification_result === pairing_check.out;
```

## Estimación de Constraints

### MVP (Modelo Attestation)

| Componente | Constraints |
|------------|-------------|
| Poseidon VK hash | 500 |
| Poseidon PI hash | 300 |
| MSM (IC combination) | 5,000 |
| G1 operations | 500 |
| Attestation check (2× Poseidon) | 600 |
| Misc (boolean checks, etc) | 100 |
| **TOTAL MVP** | **~7,000** |

### Producción (Pairing Completo)

| Componente | Constraints |
|------------|-------------|
| Poseidon VK hash | 500 |
| Poseidon PI hash | 300 |
| MSM (IC combination) | 5,000 |
| G1 operations | 2,000 |
| Pairing check (4 pairings) | 40,000 |
| Misc (range checks, etc) | 2,200 |
| **TOTAL PROD** | **~50,000** |

## Constantes

```circom
// Máximo número de public inputs soportados
// (ajustar según necesidad, afecta tamaño del circuito)
var MAX_PUBLIC_INPUTS = 32;
var MAX_IC = 33; // MAX_PUBLIC_INPUTS + 1
```

## Output

El circuito genera un proof P2 (Groth16) de 256 bytes:
- A: 64 bytes (G1)
- B: 128 bytes (G2)
- C: 64 bytes (G1)

Este P2 se verifica on-chain con ~200K compute units.

## Seguridad

### Propiedades garantizadas:

1. **Soundness**: No se puede generar P2 válido sin conocer P1 válido
2. **Binding**: El vk_hash vincula P2 a un circuito específico
3. **Completeness**: Si P1 es válido, siempre se puede generar P2 válido

### Consideraciones:

- El VK del circuito P1 debe ser conocido y hasheado correctamente
- Los public inputs deben coincidir exactamente
- El trusted setup de VERIFY_GROTH16 es independiente del de P1

## Implementación

Ver `circuit.circom` para la implementación completa.
