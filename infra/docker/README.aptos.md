# Aptos Local Testnet - Docker Setup

Setup local de Aptos para desarrollo y testing de ZyberLink.

## Quick Start

```bash
# Levantar nodo local de Aptos
cd infra/docker
docker-compose -f docker-compose.aptos.yml up -d

# Ver logs
docker-compose -f docker-compose.aptos.yml logs -f aptos-node

# Detener
docker-compose -f docker-compose.aptos.yml down

# Limpiar datos (restart fresh)
docker-compose -f docker-compose.aptos.yml down -v
```

## Endpoints Disponibles

| Servicio | URL | Descripción |
|----------|-----|-------------|
| REST API | http://localhost:8080 | Aptos Node REST API |
| Faucet | http://localhost:9081 | Faucet para obtener tokens APT |

## Verificar que está funcionando

```bash
# Health check
curl http://localhost:8080/v1

# Ver info del ledger
curl http://localhost:8080/v1 | jq '{chain_id, ledger_version, node_role}'

# Obtener tokens de la faucet (reemplazar ADDRESS)
curl -X POST "http://localhost:9081/mint?amount=100000000&address=0xYOUR_ADDRESS"
```

## Inicializar cuenta local

```bash
# Crear cuenta local
aptos init --network local --rest-url http://localhost:8080 --faucet-url http://localhost:9081

# Ver balance
aptos account list --account default

# Fondear cuenta
aptos account fund-with-faucet --account default
```

## Deploy de contratos

```bash
# Compilar
cd aptos/contracts
aptos move compile

# Publicar en local testnet
aptos move publish \
  --url http://localhost:8080 \
  --faucet-url http://localhost:9081 \
  --private-key-file ~/.aptos/config.yaml

# Ejecutar función
aptos move run \
  --function-id 'default::jobs::initialize' \
  --url http://localhost:8080
```

## Configuración del AptosClient

Para que el `chain-client` se conecte al nodo local:

```rust
use zyberlink_chain_client::AptosClient;

let client = AptosClient::new(
    "http://localhost:8080".to_string(),
    Some("http://localhost:9081".to_string()),
)?;
```

## Troubleshooting

### El nodo no arranca
```bash
# Limpiar todo y empezar de cero
docker-compose -f docker-compose.aptos.yml down -v
docker-compose -f docker-compose.aptos.yml up -d
```

### Verificar logs
```bash
docker logs -f zyberlink-aptos-node
```

### El faucet no responde
Esperar 30-40 segundos después de `docker-compose up`. El nodo tarda en inicializar.

## Diferencias con Devnet

| Aspecto | Local Testnet | Devnet |
|---------|---------------|--------|
| URL | http://localhost:8080 | https://fullnode.devnet.aptoslabs.com/v1 |
| Faucet | http://localhost:8081 | https://faucet.devnet.aptoslabs.com |
| Reset | Manual (`down -v`) | Periódico (weekly) |
| Velocidad | Instantánea | ~4s por bloque |
| Estado | Privado | Compartido |
