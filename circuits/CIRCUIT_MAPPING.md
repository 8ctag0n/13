# ZyberLink Circuit Mapping

Este documento describe los circuitos ZK implementados y sus especificaciones.

## Resumen de Circuitos

| Circuit ID | Nombre | Constraints | Public Inputs | Descripción |
|------------|--------|-------------|---------------|-------------|
| 10 | ProofOfInnocence | ~4,883 | 3 | Non-membership en blacklist |
| 20 | PrivateVote | ~5,675 | 5 | Votación anónima DAO |
| 21 | PrivateVoteWithPoI | ~10,555 | 6 | Votación + verificación blacklist |
| 30 | MarketBet | ~332 | 4 | Apuesta privada en mercados |
| 31 | MarketBetWithPoI | ~5,212 | 5 | Apuesta + anti-insider trading |
| 32 | MarketClaim | ~510 | 5 | Reclamar ganancias de mercado |
| 40 | PortfolioCompliance | ~4,883 | 3 | Compliance regulatorio |
| 41 | PortfolioNetWorth | ~5,189 | 4 | Prueba de patrimonio mínimo |

## Detalles por Circuito

### Circuit 10: ProofOfInnocence

**Propósito:** Probar que una wallet NO está en una lista negra (OFAC, sanciones, etc.)

**Public Inputs:**
- `blacklist_root` - Merkle root del árbol de blacklist
- `threshold` - Umbral de exposición permitida (normalmente 0)
- `timestamp` - Timestamp de la prueba

**Private Inputs:**
- `wallet_address` - Dirección de wallet a verificar
- `merkle_path[20]` - Camino del Sparse Merkle Tree
- `merkle_indices[20]` - Direcciones del camino (0=izq, 1=der)

**Archivo:** `circuits/poi/proof_of_innocence.circom`

---

### Circuit 20: PrivateVote

**Propósito:** Votación anónima en DAOs con prevención de doble voto

**Public Inputs:**
- `poll_id` - ID único de la votación
- `eligibility_root` - Merkle root de votantes elegibles
- `nullifier` - Hash(secret, poll_id) para prevenir doble voto
- `vote_commitment` - Hash(vote_choice, blinding)
- `min_balance` - Balance mínimo de tokens requerido

**Private Inputs:**
- `voter_wallet` - Wallet del votante
- `token_balance` - Balance real de tokens
- `vote_choice` - Opción votada (0, 1, 2, etc.)
- `blinding` - Factor de blinding para commitment
- `nullifier_secret` - Secreto para nullifier
- `merkle_path[20]` - Camino de elegibilidad
- `merkle_indices[20]` - Direcciones del camino

**Archivo:** `circuits/vote/private_vote.circom`

---

### Circuit 21: PrivateVoteWithPoI

**Propósito:** Votación anónima + verificación de no conflicto de interés

**Public Inputs:** (todos los de Circuit 20) +
- `blacklist_root` - Merkle root de conflictos de interés

**Private Inputs:** (todos los de Circuit 20) +
- `blacklist_path[20]` - Camino de non-membership
- `blacklist_indices[20]` - Direcciones del camino

**Archivo:** `circuits/vote/private_vote_poi.circom`

---

### Circuit 30: MarketBet

**Propósito:** Apuestas privadas en mercados de predicción

**Public Inputs:**
- `market_id` - ID del mercado
- `bet_commitment` - Hash(amount, position, blinding)
- `max_bet` - Apuesta máxima permitida
- `timestamp` - Timestamp de la apuesta

**Private Inputs:**
- `bettor_wallet` - Wallet del apostador
- `bet_amount` - Monto apostado
- `position` - Posición/predicción (0, 1, etc.)
- `blinding` - Factor de blinding

**Archivo:** `circuits/market/market_bet.circom`

---

### Circuit 31: MarketBetWithPoI

**Propósito:** Apuesta + prevención de insider trading

**Public Inputs:** (todos los de Circuit 30) +
- `insider_blacklist_root` - Merkle root de insiders

**Private Inputs:** (todos los de Circuit 30) +
- `blacklist_path[20]` - Camino de non-membership
- `blacklist_indices[20]` - Direcciones del camino

**Archivo:** `circuits/market/market_bet_poi.circom`

---

### Circuit 32: MarketClaim

**Propósito:** Reclamar ganancias de mercado con prueba de apuesta ganadora

**Public Inputs:**
- `market_id` - ID del mercado
- `nullifier` - Hash(secret, bet_commitment) para prevenir doble claim
- `payout_amount` - Monto a pagar
- `resolution` - Resultado ganador (0/1)
- `total_pool` - Pool total del mercado
- `winning_pool` - Pool del lado ganador
- `bet_commitment` - Commitment original de la apuesta
- `timestamp` - Timestamp del claim

**Private Inputs:**
- `secret` - Secreto del usuario
- `bet_amount` - Monto apostado originalmente
- `bet_side` - Lado apostado (0/1)
- `blinding` - Factor de blinding original

**Archivo:** `circuits/market/market_claim.circom`

---

### Circuit 40: PortfolioCompliance

**Propósito:** Probar cumplimiento regulatorio (no exposición a direcciones sancionadas)

**Public Inputs:**
- `compliance_root` - Merkle root de lista de compliance
- `threshold` - Umbral máximo de exposición (normalmente 0)
- `timestamp` - Timestamp de la prueba

**Private Inputs:**
- `wallet_address` - Wallet a verificar
- `merkle_path[20]` - Camino de non-membership
- `merkle_indices[20]` - Direcciones del camino

**Archivo:** `circuits/portfolio/compliance.circom`

---

### Circuit 41: PortfolioNetWorth

**Propósito:** Probar patrimonio mínimo sin revelar monto exacto

**Public Inputs:**
- `min_threshold` - Umbral mínimo a probar
- `price_oracle_root` - Merkle root de precios del oráculo
- `timestamp` - Timestamp del snapshot
- `net_worth_commitment` - Hash(net_worth, blinding)

**Private Inputs:**
- `net_worth` - Patrimonio real
- `blinding` - Factor de blinding
- `oracle_path[20]` - Camino en árbol de oráculo
- `oracle_indices[20]` - Direcciones del camino

**Archivo:** `circuits/portfolio/net_worth.circom`

---

## Estructura de Directorios

```
circuits/
├── poi/
│   └── proof_of_innocence.circom
├── vote/
│   ├── private_vote.circom
│   └── private_vote_poi.circom
├── market/
│   ├── market_bet.circom
│   ├── market_bet_poi.circom
│   └── market_claim.circom
├── portfolio/
│   ├── compliance.circom
│   └── net_worth.circom
├── test-proofs/
│   ├── inputs/          # JSON inputs de prueba
│   ├── outputs/         # Proofs generadas
│   ├── generate_all_proofs.sh
│   └── validate_vks.sh
└── lib/                 # Librerías compartidas
    ├── bn254.circom
    ├── pairing.circom
    └── poseidon_utils.circom

verification_keys/
├── circuit_10_vkey.json
├── circuit_20_vkey.json
├── circuit_21_vkey.json
├── circuit_30_vkey.json
├── circuit_31_vkey.json
├── circuit_32_vkey.json
├── circuit_40_vkey.json
└── circuit_41_vkey.json
```

## Uso

### Generar todas las proofs
```bash
cd circuits/test-proofs
./generate_all_proofs.sh
```

### Validar VKeys
```bash
cd circuits/test-proofs
./validate_vks.sh
```

### Compilar un circuito
```bash
cd circuits/<dir>
../circom <circuit>.circom --r1cs --wasm --sym -o . -l ../node_modules
```

### Generar zkey
```bash
npx snarkjs groth16 setup circuit.r1cs ../ptau/pot14_final.ptau circuit_0000.zkey
npx snarkjs zkey contribute circuit_0000.zkey circuit_0001.zkey --name="first" -v -e="entropy"
npx snarkjs zkey beacon circuit_0001.zkey circuit_final.zkey <beacon> 10 -n="Final"
npx snarkjs zkey export verificationkey circuit_final.zkey circuit_vkey.json
```

## Powers of Tau

- `pot14_final.ptau` - Para circuitos hasta ~16K constraints
- `pot18_final.ptau` - Para circuitos hasta ~256K constraints

Solo Circuit 21 (PrivateVoteWithPoI) requiere pot18 por tener ~10.5K constraints que superan el límite de pot14.
