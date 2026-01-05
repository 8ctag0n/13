# Guía de la CLI Zyb

La CLI de ZyberLink (`zyb`) es la herramienta principal para interactuar con el protocolo ZYB desde la línea de comandos. Soporta operaciones tanto de FHE (Encriptación Totalmente Homomórfica) como de ZK (Zero-Knowledge).

## Instalación

### Prerrequisitos
- Rust 1.75+
- Solana CLI 2.3+

### Compilar desde Fuente
```bash
cd zyb-cli
cargo build --release
sudo cp target/release/zyb /usr/local/bin/
```

---

## Comandos FHE

Los comandos FHE te permiten encriptar datos localmente y desencriptar los resultados de las computaciones.

### 1. Encriptar Datos
Encripta valores (0-255) para su uso en trabajos de analytics privados o cumplimiento.

```bash
zyb fhe encrypt --values 10,20,30,40,50 --output ./fhe-output
```

**Archivos de Salida:**
- `witness.bin`: Datos encriptados + Clave de Servidor (subir a ZyberLink).
- `client_key.bin`: Tu clave privada de desencriptación (MANTENER SECRETA).

### 2. Desencriptar Resultado
Desencripta los resultados devueltos por los provers.

```bash
zyb fhe decrypt --result-path result.bin --client-key-path ./fhe-output/client_key.bin
```

---

## Comandos ZK On-Chain

Interactúa directamente con el programa `zk-generator` en Solana.

### 1. Crear Job ZK
Crea un nuevo job ZK directamente en la blockchain.

```bash
zyb zk onchain create \
  --circuit-type 10 \
  --witness witness.json \
  --price 0.1 \
  --keypair ~/.config/solana/id.json
```

### 2. Reclamar Job ZK (Provers)
Reclama un trabajo pendiente para su procesamiento.

```bash
zyb zk onchain claim --job-id <JOB_PDA> --keypair prover.json
```

### 3. Enviar Prueba ZK
Envía el hash de la prueba generada para finalizar el trabajo y recibir el pago.

```bash
zyb zk onchain submit --job-id <JOB_PDA> --proof proof.json --keypair prover.json
```

### 4. Consultar Estado
Consulta el estado de cualquier trabajo on-chain.

```bash
zyb zk onchain status --job-id <JOB_PDA>
```

---

## Referencia de Tipos de Circuito

| ID | Nombre | Categoría | Descripción |
|----|------|----------|-------------|
| 10 | ProofOfInnocence | Core | Prueba de no-membresía (lista negra) |
| 20 | PrivateVote | Votación | Voto anónimo en DAO |
| 30 | MarketBet | Mercado | Apuestas en prediction markets |
| 40 | PortfolioCompliance | Portafolio | Chequeos de cumplimiento regulatorio |

---

## Variables de Entorno

Configura tu entorno para un uso fluido de la CLI:

```bash
export SOLANA_RPC_URL="https://api.devnet.solana.com"
export SOLANA_KEYPAIR="~/.config/solana/id.json"
```

