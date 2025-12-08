# ZyberLink ZK Circuits

Circuitos de verificación para el sistema dual-proof de ZyberLink.

## Arquitectura

```
Prover genera P1 → Backend verifica P1 → Backend genera P2 → On-chain verifica P2
      (any size)      (VERIFY_*)            (256 bytes)        (~200K CU)
```

## Circuitos Disponibles

| Circuito | Verifica | Constraints | Estado |
|----------|----------|-------------|--------|
| VERIFY_GROTH16 | Proofs Groth16 | ~50K | MVP |
| VERIFY_PLONK | Proofs PLONK | ~80K | Q2 |
| VERIFY_HALO2 | Proofs Halo2 | ~100K | Q3 |

## Estructura

```
circuits/
├── lib/                    # Templates compartidos
│   ├── bn254.circom       # Operaciones de curva BN254
│   ├── poseidon_utils.circom  # Hash utilities
│   └── pairing.circom     # Verificación de pairing
├── verify-groth16/        # VERIFY_GROTH16
│   ├── SPEC.md           # Especificación técnica
│   ├── circuit.circom    # Circuito principal
│   ├── input.json        # Input de ejemplo
│   └── build.sh          # Script de compilación
├── verify-plonk/          # VERIFY_PLONK (pendiente)
├── verify-halo2/          # VERIFY_HALO2 (pendiente)
├── test/                  # Tests
└── scripts/               # Scripts de utilidad
```

## Requisitos

- Node.js >= 18
- Circom 2.1.6+
- snarkjs 0.7+

## Setup

```bash
# Instalar dependencias
npm install

# Descargar Powers of Tau
npm run setup:ptau

# Compilar VERIFY_GROTH16
npm run build:groth16

# Ejecutar tests
npm test
```

## Public Inputs (On-chain)

Cada circuito VERIFY_* tiene 3 public inputs:

| Input | Tamaño | Descripción |
|-------|--------|-------------|
| `vk_hash` | 32 bytes | Hash del verification key de P1 |
| `public_inputs_hash` | 32 bytes | Hash de los public inputs de P1 |
| `verification_result` | 1 bit | 1 si P1 válido, 0 si no |

Total on-chain: 96 bytes + proof P2 (256 bytes) = 352 bytes

## Uso desde Backend

```typescript
import { verifyGroth16 } from '@zyberlink/circuits';

// Recibir P1 del prover
const p1 = await receiveProofFromProver(jobId);

// Verificar P1 y generar P2
const { p2, publicInputs } = await verifyGroth16({
    proof: p1,
    vk: circuitVK,
    inputs: originalPublicInputs
});

// Enviar P2 on-chain
await submitResult(jobId, p2, publicInputs);
```

## Trusted Setup

Cada circuito VERIFY_* requiere su propio trusted setup (Phase 2).

```bash
# Usar Powers of Tau existente (Phase 1)
# Hermez: https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_16.ptau

# Generar zkey (Phase 2)
snarkjs groth16 setup circuit.r1cs ptau.ptau circuit_0000.zkey

# Contribuir a la ceremonia
snarkjs zkey contribute circuit_0000.zkey circuit_final.zkey

# Exportar verification key
snarkjs zkey export verificationkey circuit_final.zkey vk.json
```
