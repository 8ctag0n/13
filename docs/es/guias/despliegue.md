# Guía de Despliegue

Despliega ZyberLink en ambientes de producción.

## Resumen

Esta guía cubre:
- Despliegue del programa de Solana
- Setup de nodos prover
- Integración de SDK cliente

## Prerrequisitos

**Ambiente de producción:**
- Linux (Ubuntu 22.04+ recomendado)
- Rust 1.75+
- Solana CLI 2.1+
- 8GB+ RAM, 4+ cores

**Para provers:**
- 16GB+ RAM recomendado
- 8+ cores
- Conexión estable y uptime 24/7

## Parte 1: Programa de Solana

### 1.1 Build

Reemplaza `<ORG>` por tu organizacion de GitHub o mirror.

```bash
git clone https://github.com/<ORG>/zyb-chain.git
cd zyb-chain/solana
cargo build-sbf --manifest-path=programs/zyberlink/Cargo.toml
```

### 1.2 Configurar Solana CLI
```bash
solana config set --url https://api.mainnet-beta.solana.com
solana config set --keypair /ruta/a/tu/keypair.json
solana config get
```

### 1.3 Fondos para deploy
```bash
solana balance   # Necesitas ~5-10 SOL
```

### 1.4 Deploy
```bash
solana program deploy target/deploy/zyberlink.so
# Guarda el Program ID mostrado
```

### 1.5 Verificación
```bash
solana program show <PROGRAM_ID>
```

## Parte 2: Nodo Prover

### 2.1 Instalar dependencias
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2.2 Build

Reemplaza `<ORG>` por tu organizacion de GitHub o mirror.

```bash
git clone https://github.com/<ORG>/zyb-compute.git
cd zyb-compute
cargo build --release --bin zyberlink-prover
# Binario: target/release/zyberlink-prover
```

### 2.3 Configurar
```bash
mkdir -p ~/.zyberlink
nano ~/.zyberlink/config.toml
```

```toml
[network]
rpc_url = "https://api.mainnet-beta.solana.com"
program_id = "PROGRAM_ID_AQUI"

[prover]
wallet_path = "/ruta/a/prover/keypair.json"
auto_claim = true
max_concurrent_jobs = 3

[fhe]
enabled = true
max_computation_time_seconds = 60
```

Asistente opcional:
```bash
cd zyb-compute
cargo run --release -- wizard
```

### 2.4 Ejecutar
```bash
# Primer plano
./target/release/zyberlink-prover

# Background
nohup ./target/release/zyberlink-prover > prover.log 2>&1 &
```

Systemd (recomendado):
```ini
[Unit]
Description=ZyberLink Prover Node
After=network.target

[Service]
Type=simple
User=tuusuario
WorkingDirectory=/home/tuusuario/zyb-compute
ExecStart=/home/tuusuario/zyb-compute/target/release/zyberlink-prover
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now zyberlink-prover
sudo journalctl -u zyberlink-prover -f
```

## Parte 3: SDK Cliente

### 3.1 Dependencia
```toml
[dependencies]
zyberlink-sdk = "0.1"
solana-client = "1.18"
solana-sdk = "1.18"
tokio = { version = "1", features = ["full"] }
```

### 3.2 Ejemplo básico
```rust
use zyberlink_sdk::MarketplaceClient;
use solana_sdk::signature::{Keypair, read_keypair_file};
use solana_client::rpc_client::RpcClient;

let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
let payer = read_keypair_file("/ruta/keypair.json")?;
let program_id = "PROGRAM_ID".parse()?;
let client = MarketplaceClient::new(rpc, payer, program_id);
```

## Monitoreo

- Estado systemd: `sudo systemctl status zyberlink-prover`
- Logs: `sudo journalctl -u zyberlink-prover -n 100`
- Balance: `solana balance /ruta/a/prover/keypair.json`
- Stats: `./target/release/zyberlink-prover stats`

## Seguridad

- Nunca comprometer keypairs en git
- Firewall: `sudo ufw enable && sudo ufw allow 22/tcp`
- Mantener el sistema actualizado

## Troubleshooting

- **Falla deploy:** revisa balance y usa `--max-len` si el binario es grande.
- **Prover cae:** revisa logs, RAM, timeouts de RPC, fondos.
- **No se reclaman jobs:** verifica `program_id` y que haya SOL en la wallet.
