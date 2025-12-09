# VERIFY_GROTH16 - Keys Generated

## Circuit Specifications

- **Constraints**: 228,394
- **Wires**: 228,341
- **Private Inputs**: 122
- **Public Inputs**: 3
  - `vk_hash`
  - `public_inputs_hash`
  - `verification_result`
- **Curve**: BN254 (bn128)
- **Protocol**: Groth16

## Generated Files

### Proving Keys
- `circuit_0000.zkey` (98 MB) - Initial setup from PTAU
- `circuit_final.zkey` (98 MB) - After phase 2 contribution
  - Circuit Hash: `8cc196a5 5a6165aa a4210b29 8af93c71 ...`
  - Contribution Hash: `3661ebac 1fade2de b989ef8a 409e7bd6 ...`

### Verification Key
- `verification_key.json` (3.3 KB)

**Structure**:
```json
{
  "protocol": "groth16",
  "curve": "bn128",
  "nPublic": 3,
  "vk_alpha_1": [x, y, z],        // G1 point (3 coordinates)
  "vk_beta_2": [[...], [...], [...]], // G2 point
  "vk_gamma_2": [[...], [...], [...]], // G2 point
  "vk_delta_2": [[...], [...], [...]], // G2 point
  "vk_alphabeta_12": [...],       // GT pairing result
  "IC": [p0, p1, p2, p3]          // 4 G1 points (nPublic + 1)
}
```

## PTAU Used

- **File**: `../ptau/pot18_final.ptau` (289 MB)
- **Powers**: 2^18 = 262,144 constraints
- **Source**: Hermez Ceremony
- **URL**: https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_18.ptau

## Backend Integration

### Rust Compatibility

El verification key generado es compatible con `ark-groth16` y `bellman`:

**Fields mapping**:
```rust
// ark-groth16
pub struct VerifyingKey<E: PairingEngine> {
    pub alpha_g1: E::G1Affine,        // vk_alpha_1
    pub beta_g2: E::G2Affine,         // vk_beta_2
    pub gamma_g2: E::G2Affine,        // vk_gamma_2
    pub delta_g2: E::G2Affine,        // vk_delta_2
    pub gamma_abc_g1: Vec<E::G1Affine>, // IC array
}
```

**Coordinate format**:
- G1 points: `[x, y, z]` (projective) or `[x, y]` (affine)
- G2 points: `[[x1, x2], [y1, y2], [z1, z2]]`
- Field elements: decimal strings (converted to `Fr`)

### JSON Parsing

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct VkJson {
    pub protocol: String,
    pub curve: String,
    #[serde(rename = "nPublic")]
    pub n_public: usize,
    pub vk_alpha_1: [String; 3],
    pub vk_beta_2: [[String; 2]; 3],
    pub vk_gamma_2: [[String; 2]; 3],
    pub vk_delta_2: [[String; 2]; 3],
    #[serde(rename = "IC")]
    pub ic: Vec<[String; 3]>,
}
```

## Next Steps

1. **Integration Tests**: Generate valid witness with correct inputs
2. **Backend Proof Generation**: Use `circuit_final.zkey` to generate proofs
3. **Backend Verification**: Parse `verification_key.json` and verify proofs
4. **Optimization**: Consider trusted setup ceremony for production

## Testing

Para generar una prueba válida, necesitas:
1. Inputs matemáticamente válidos que cumplan las constraints
2. Generar witness: `snarkjs wtns calculate circuit.wasm input.json witness.wtns`
3. Generar proof: `snarkjs groth16 prove circuit_final.zkey witness.wtns proof.json public.json`
4. Verificar: `snarkjs groth16 verify verification_key.json public.json proof.json`

## Production Considerations

- **Trusted Setup**: Realizar ceremonia multi-party para production
- **Key Security**: `circuit_final.zkey` debe estar protegido (permite generar proofs)
- **VK Distribution**: `verification_key.json` es público y puede distribuirse
- **Circuit Audit**: Auditar el circuito antes de deployment
