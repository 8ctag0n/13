# MAPEO DE ENDPOINTS ACTUALES

Documento generado para la migración a arquitectura de 3 capas.
Fecha: 2025-12-10

## BLINK-SERVER (Puerto 3000)

### Core
| Método | Endpoint | Archivo | Migrar a |
|--------|----------|---------|----------|
| GET | `/health` | main.rs | public-api |
| GET | `/actions.json` | main.rs | public-api |
| GET | `/static/zyberlink-icon.svg` | main.rs | public-api |

### Solana Actions (Legacy Blinks)
| Método | Endpoint | Archivo | Migrar a |
|--------|----------|---------|----------|
| GET | `/api/actions/fund-prover` | actions.rs | public-api |
| POST | `/api/actions/fund-prover` | actions.rs | public-api |

### FHE Jobs
| Método | Endpoint | Archivo | Nuevo path interno |
|--------|----------|---------|-------------------|
| GET | `/api/jobs/fhe` | api_handlers.rs | `/internal/fhe/list` |
| POST | `/api/jobs/fhe/validate-and-build` | api_handlers.rs | `/internal/fhe/validate-and-build` |
| GET | `/api/jobs/fhe/{id}` | api_handlers.rs | `/internal/fhe/{id}` |
| GET | `/api/jobs/fhe/{id}/status` | api_handlers.rs | `/internal/fhe/{id}/status` |
| GET | `/api/jobs/fhe/{id}/compute-data` | api_handlers.rs | `/internal/fhe/{id}/compute-data` |
| POST | `/api/jobs/fhe/{id}/confirm` | api_handlers.rs | `/internal/fhe/{id}/confirm` |
| GET | `/api/jobs/fhe/{id}/chain-status` | api_handlers.rs | `/internal/fhe/{id}/chain-status` |
| GET | `/api/jobs/fhe/{id}/provers` | api_handlers.rs | `/internal/fhe/{id}/provers` |
| GET | `/api/jobs/fhe/{id}/result` | api_handlers.rs | `/internal/fhe/{id}/result` |
| DELETE | `/api/jobs/fhe/{id}` | api_handlers.rs | `/internal/fhe/{id}` |

### ZK Jobs
| Método | Endpoint | Archivo | Nuevo path interno |
|--------|----------|---------|-------------------|
| GET | `/api/jobs/zk` | zk_handlers.rs | `/internal/zk/list` |
| POST | `/api/jobs/zk/validate-and-build` | zk_handlers.rs | `/internal/zk/validate-and-build` |
| GET | `/api/jobs/zk/{id}/status` | zk_handlers.rs | `/internal/zk/{id}/status` |
| POST | `/api/jobs/zk/{id}/confirm` | zk_handlers.rs | `/internal/zk/{id}/confirm` |
| POST | `/api/jobs/zk/{id}/submit-proof` | zk_handlers.rs | `/internal/zk/{id}/submit-proof` |
| GET | `/api/jobs/zk/{id}/attestation` | zk_handlers.rs | `/internal/zk/{id}/attestation` |

### Storage
| Método | Endpoint | Archivo | Nuevo path interno |
|--------|----------|---------|-------------------|
| POST | `/witness` | api_handlers.rs | `/internal/witness` |
| GET | `/witness/{commitment}` | api_handlers.rs | `/internal/witness/{commitment}` |
| POST | `/fhe-result` | api_handlers.rs | `/internal/fhe-result` |
| GET | `/fhe-result/{commitment}` | api_handlers.rs | `/internal/fhe-result/{commitment}` |
| POST | `/api/server-key/upload` | api_handlers.rs | `/internal/server-key/upload` |
| GET | `/api/server-key/{hash}/exists` | api_handlers.rs | `/internal/server-key/{hash}/exists` |

### Stats & Metrics
| Método | Endpoint | Archivo | Nuevo path interno |
|--------|----------|---------|-------------------|
| GET | `/api/stats/network` | api_handlers.rs | `/internal/stats/network` |
| GET | `/api/metrics` | api_handlers.rs | `/internal/metrics` |

### Pricing
| Método | Endpoint | Archivo | Migrar a |
|--------|----------|---------|----------|
| POST | `/api/estimate-cost` | api_handlers.rs | x402 o public-api |
| POST | `/api/price-recommendation` | api_handlers.rs | x402 o public-api |

---

## X402-SERVER (Puerto 8081)

### Core
| Método | Endpoint | Archivo | Mantener |
|--------|----------|---------|----------|
| GET | `/health` | handlers.rs | SI |

### Quotes & Pricing
| Método | Endpoint | Archivo | Mantener |
|--------|----------|---------|----------|
| POST | `/api/quote` | handlers.rs | SI |
| POST | `/api/estimate` | handlers.rs | SI |

### Payment
| Método | Endpoint | Archivo | Mantener |
|--------|----------|---------|----------|
| POST | `/api/build-payment` | handlers.rs | SI |
| POST | `/api/confirm` | handlers.rs | SI |

### Token Management (Interno)
| Método | Endpoint | Archivo | Mantener |
|--------|----------|---------|----------|
| POST | `/api/validate` | handlers.rs | SI |
| POST | `/api/mark-used` | handlers.rs | SI |
| GET | `/api/token/{id}/status` | handlers.rs | SI |

### Protected Operations (DUPLICADOS - eliminar)
| Método | Endpoint | Archivo | Accion |
|--------|----------|---------|--------|
| POST | `/api/witness` | handlers.rs | ELIMINAR (proxy a blink) |
| POST | `/api/create-job` | handlers.rs | ELIMINAR (proxy a blink) |

---

## DOCKER-COMPOSE.YML

### Servicios Actuales
| Servicio | Puerto Expuesto | Puerto Interno |
|----------|-----------------|----------------|
| validator | 8899, 8900 | - |
| nginx | 9000 | 80 |
| webapp | - | 80 |
| backend | - | 8080 |
| postgres | 5432 | 5432 |

### Servicios Faltantes
| Servicio | Puerto | Estado |
|----------|--------|--------|
| x402 | 8081 | NO EXISTE |
| public-api | 3000 | NO EXISTE |

---

## RESUMEN DE CAMBIOS POR SERVICIO

### blink-server (actual -> interno)
- Renombrar TODOS `/api/*` -> `/internal/*`
- Cambiar puerto: 3000 -> 8080
- Cambiar host: 0.0.0.0 -> 127.0.0.1
- Eliminar CORS
- Total endpoints: ~25

### x402-server (gateway)
- Agregar endpoints `/gateway/*` para proxy
- Agregar rate limiting por IP
- Agregar cliente HTTP para blink
- Eliminar `/api/witness` y `/api/create-job` duplicados
- Total endpoints actuales: 10

### public-api (NUEVO)
- Crear desde cero
- Puerto 3000 (unico expuesto)
- Proxy a x402 y blink
- Rate limit 100 req/min
- Total endpoints: ~15 (proxy)

---

## CONFIGURACION ACTUAL

### blink-server env vars
```
HOST=0.0.0.0
PORT=3000 (debe cambiar a 8080)
DATABASE_URL=postgresql://...
SOLANA_RPC_URL=http://localhost:8899
PROGRAM_ID=ZyberLinkProgram...
X402_URL=http://localhost:8081
VK_DIRECTORY=./verification_keys
SERVER_KEYPAIR_PATH=~/.config/solana/id.json
CLEANUP_INTERVAL_SECS=3600
```

### x402-server env vars
```
X402_HOST=0.0.0.0
X402_PORT=8081
DATABASE_URL=postgresql://...
```

### Nuevas env vars necesarias
```
# public-api
PUBLIC_API_HOST=0.0.0.0
PUBLIC_API_PORT=3000
X402_URL=http://x402:8081
BLINK_URL=http://blink:8080
RATE_LIMIT_REQUESTS_PER_MINUTE=100

# x402 (nuevas)
BLINK_URL=http://blink:8080
RATE_LIMIT_PER_IP=10
```
