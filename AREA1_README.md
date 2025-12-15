# AREA 1: EXPANSION DE CIRCUITOS Y GENERACION DE PRUEBAS

**Branch**: `circuit-expansion-proofs`
**Worktree**: `/home/deploy/2025q4/13-area1-circuits`
**Esfuerzo estimado**: 25-32 horas (1 semana)
**Prioridad**: ALTA

---

## OBJETIVO

Expandir el sistema de circuitos para cubrir mas casos de uso y asegurar que la generacion/verificacion de pruebas funcione end-to-end para todos los circuit types.

---

## ESTADO ACTUAL

**Circuitos Existentes**:
```
circuits/
├── simple/              # Circuito basico (Multiplier 3x5=15)
│   ├── circuit.circom
│   ├── proof.json       # Proof valido generado
│   └── simple_vkey.json
│
├── verify-groth16/      # Meta-circuito (verifica proofs)
│   ├── circuit.circom   # 228K constraints
│   └── verification_key.json
│
└── lib/                 # Librerias compartidas
    ├── bn254.circom
    ├── pairing.circom
    └── poseidon_utils.circom
```

**VKs Cargadas**: 27 activas (circuits 10-52) en `verification_keys/`

**Gap Principal**: Los circuit types 10-52 tienen VKs pero NO tienen .circom implementado

---

## FASES DE IMPLEMENTACION

### FASE 1.1: Analisis de Circuit Types (2-3h)

**Objetivo**: Mapear que circuitos necesitamos implementar.

**Comandos**:
```bash
cd /home/deploy/2025q4/13-area1-circuits

# Listar definiciones de circuits en Rust
rg "pub enum|pub struct" src/programs/zk-generator/src/circuits/ -A 5

# Ver circuit types existentes
ls -la verification_keys/
```

**Entregable**: Crear `CIRCUIT_MAPPING.md` con tabla de mapeo.

---

### FASE 1.2: Implementar Circuitos Prioritarios (10-12h)

**Prioridad 1: Proof of Identity (PoI)**

```bash
mkdir -p circuits/poi
```

**Archivo**: `circuits/poi/identity_proof.circom`
```circom
pragma circom 2.0.0;

include "../lib/poseidon_utils.circom";

template IdentityProof() {
    // Public inputs
    signal input commitment;
    signal output result;

    // Private witnesses
    signal input privateKey;
    signal input nonce;
    signal input timestamp;

    // Verificar commitment = Poseidon(privateKey, nonce)
    component hasher = Poseidon(2);
    hasher.inputs[0] <== privateKey;
    hasher.inputs[1] <== nonce;
    commitment === hasher.out;

    result <== 1;
}

component main {public [commitment]} = IdentityProof();
```

**Compilacion**:
```bash
cd circuits/poi
../../circom identity_proof.circom --r1cs --wasm --sym

# Setup
snarkjs groth16 setup identity_proof.r1cs ../ptau/pot18_final.ptau identity_proof_0000.zkey
snarkjs zkey contribute identity_proof_0000.zkey identity_proof_final.zkey --name="dev" -v
snarkjs zkey export verificationkey identity_proof_final.zkey identity_proof_vkey.json

# Copiar VK
cp identity_proof_vkey.json ../../verification_keys/circuit_11_vkey.json
```

**Prioridad 2: Anonymous Vote**

```bash
mkdir -p circuits/vote
```

**Archivo**: `circuits/vote/anonymous_vote.circom`
```circom
pragma circom 2.0.0;

include "../lib/poseidon_utils.circom";

template AnonymousVote() {
    signal input nullifier;        // Public: evita doble voto
    signal input merkleRoot;       // Public: root del arbol de votantes
    signal output voteCommitment;  // Public: commitment del voto

    // Private
    signal input voterSecret;
    signal input voteOption;       // 0, 1, 2, etc.
    signal input merkleProof[20];
    signal input merklePathIndices[20];

    // 1. Verificar membership en arbol
    component verifier = MerkleTreeVerifier(20);
    verifier.leaf <== Poseidon(1).inputs[0] <== voterSecret;
    verifier.root <== merkleRoot;
    for (var i = 0; i < 20; i++) {
        verifier.pathElements[i] <== merkleProof[i];
        verifier.pathIndices[i] <== merklePathIndices[i];
    }

    // 2. Calcular nullifier
    component nullifierHasher = Poseidon(2);
    nullifierHasher.inputs[0] <== voterSecret;
    nullifierHasher.inputs[1] <== merkleRoot;
    nullifier === nullifierHasher.out;

    // 3. Commitment del voto
    component voteHasher = Poseidon(2);
    voteHasher.inputs[0] <== voteOption;
    voteHasher.inputs[1] <== voterSecret;
    voteCommitment <== voteHasher.out;
}

component main {public [nullifier, merkleRoot]} = AnonymousVote();
```

**Prioridad 3: Portfolio Balance**

```bash
mkdir -p circuits/portfolio
```

**Archivo**: `circuits/portfolio/balance_proof.circom`
```circom
pragma circom 2.0.0;

template BalanceProof() {
    signal input minBalance;       // Public: balance minimo requerido
    signal output isValid;         // Public: 1 si cumple

    // Private
    signal input actualBalance;

    // Verificar balance >= minBalance
    component gte = GreaterEqThan(64);
    gte.in[0] <== actualBalance;
    gte.in[1] <== minBalance;

    isValid <== gte.out;
}

component main {public [minBalance]} = BalanceProof();
```

---

### FASE 1.3: Generacion Automatizada de Proofs (4-5h)

**Crear script maestro**: `circuits/test-proofs/generate_all_proofs.sh`

```bash
#!/bin/bash
set -e

CIRCUITS_DIR="/home/deploy/2025q4/13-area1-circuits/circuits"
OUTPUT_DIR="$CIRCUITS_DIR/test-proofs/outputs"

mkdir -p "$OUTPUT_DIR"

echo "Generating test proofs for all circuits..."

# Circuit 10: Simple Multiplier
echo "Circuit 10: Simple Multiplier"
cd "$CIRCUITS_DIR/simple"
snarkjs wtns calculate circuit_js/circuit.wasm ../test-proofs/circuit_10_test.json witness.wtns
snarkjs groth16 prove simple_final.zkey witness.wtns "$OUTPUT_DIR/circuit_10_proof.json" "$OUTPUT_DIR/circuit_10_public.json"
snarkjs groth16 verify simple_vkey.json "$OUTPUT_DIR/circuit_10_public.json" "$OUTPUT_DIR/circuit_10_proof.json"
echo "Circuit 10 OK"

# Circuit 11: Identity Proof
echo "Circuit 11: Identity Proof"
cd "$CIRCUITS_DIR/poi"
snarkjs wtns calculate identity_proof_js/identity_proof.wasm ../test-proofs/circuit_11_test.json witness.wtns
snarkjs groth16 prove identity_proof_final.zkey witness.wtns "$OUTPUT_DIR/circuit_11_proof.json" "$OUTPUT_DIR/circuit_11_public.json"
snarkjs groth16 verify identity_proof_vkey.json "$OUTPUT_DIR/circuit_11_public.json" "$OUTPUT_DIR/circuit_11_proof.json"
echo "Circuit 11 OK"

echo "All test proofs generated!"
```

**Inputs de test**:

`circuits/test-proofs/circuit_10_test.json`:
```json
{
  "a": "3",
  "b": "5"
}
```

`circuits/test-proofs/circuit_11_test.json`:
```json
{
  "commitment": "12345678901234567890123456789012",
  "privateKey": "98765432109876543210987654321098",
  "nonce": "11111111111111111111111111111111",
  "timestamp": "1702123456"
}
```

---

### FASE 1.4: Verificacion On-Chain (6-8h)

**Objetivo**: Implementar witness generator para dispute proofs.

**Archivo a crear**: `src/blink-server/src/services/witness_generator.rs`

Este modulo genera el witness para el circuito verify-groth16, permitiendo disputes on-chain.

---

### FASE 1.5: Testing E2E (3-4h)

**Test completo**: `tests/circuits/e2e_proof_flow_test.rs`

```rust
#[tokio::test]
async fn test_full_zk_attestation_flow() {
    // 1. Create ZK job
    // 2. Confirm job
    // 3. Generate proof locally
    // 4. Submit proof
    // 5. Fetch attestation
    // 6. Generate P2 (dispute proof)
    // 7. Verify P2 locally
}
```

---

## CHECKPOINTS

### End of Week 1
- [ ] CIRCUIT_MAPPING.md completo
- [ ] Al menos 2 circuitos nuevos compilados (PoI + Vote)
- [ ] VKs generadas y copiadas a verification_keys/
- [ ] Proofs de test generados y verificados
- [ ] Script de automatizacion funcional

### End of Week 2
- [ ] 5+ circuitos implementados
- [ ] Witness generator en backend
- [ ] Tests E2E pasando
- [ ] Documentacion de cada circuito

---

## TROUBLESHOOTING

**"El circuito no compila"**
```bash
../../circom --inspect circuit.circom
ls -la ../lib/
```

**"snarkjs falla al generar proof"**
```bash
snarkjs wtns check circuit.r1cs witness.wtns
cat input.json
```

**"Verification falla"**
- Revisar valores en input.json
- Debug con `log()` en el circuito

---

## ARCHIVOS CLAVE

```
circuits/
├── poi/identity_proof.circom       # CREAR
├── vote/anonymous_vote.circom      # CREAR
├── portfolio/balance_proof.circom  # CREAR
└── test-proofs/
    ├── generate_all_proofs.sh      # CREAR
    └── outputs/                    # CREAR

src/blink-server/src/services/
└── witness_generator.rs            # CREAR (FASE 1.4)

verification_keys/
├── circuit_11_vkey.json            # COPIAR (PoI)
├── circuit_12_vkey.json            # COPIAR (Vote)
└── circuit_13_vkey.json            # COPIAR (Portfolio)
```

---

## CONTACTO

Para dudas o blockers, comunicar en `#zyberlink-3areas`
