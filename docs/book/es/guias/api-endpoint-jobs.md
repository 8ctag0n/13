# GET /api/jobs - Endpoint de Listado de Jobs

## Descripción General

Endpoint para listar jobs en el marketplace de ZyberLink con filtrado opcional por estado.

**URL:** `GET /api/jobs`

**Propósito:** Permite a los clientes web ver y monitorear jobs en el marketplace de forma dinámica.

---

## Parámetros de Query

| Parámetro | Tipo | Requerido | Descripción |
|-----------|------|-----------|-------------|
| `status` | string | No | Filtrar jobs por estado. Valores válidos: `pending_tx`, `active`, `completed`, `failed` |

---

## Formato de Response

```json
{
  "jobs": [
    {
      "job_id": 123,
      "creator_pubkey": "FdMVQxVLxGhYd8hyBE5hKoCVzYAuXioTeMV2u1VP1Wcj",
      "operation": "add",
      "operation_value": 10,
      "price_lamports": 3000000,
      "required_provers": 3,
      "consensus_threshold": 2,
      "status": "active",
      "payment_method": "SOL",
      "created_at": "2025-11-22T22:00:00Z"
    }
  ],
  "count": 1
}
```

---

## Ejemplos

### 1. Listar todos los jobs

```bash
curl http://127.0.0.1:8080/api/jobs
```

**Response:**
```json
{
  "jobs": [...],
  "count": 42
}
```

### 2. Listar solo jobs activos

```bash
curl "http://127.0.0.1:8080/api/jobs?status=active"
```

**Response:**
```json
{
  "jobs": [
    {
      "job_id": 123,
      "status": "active",
      ...
    }
  ],
  "count": 5
}
```

### 3. Listar jobs pendientes (esperando confirmación blockchain)

```bash
curl "http://127.0.0.1:8080/api/jobs?status=pending_tx"
```

### 4. Listar jobs completados

```bash
curl "http://127.0.0.1:8080/api/jobs?status=completed"
```

### 5. Estado inválido (retorna error 400)

```bash
curl "http://127.0.0.1:8080/api/jobs?status=invalid"
```

**Response:**
```json
{
  "error": "Invalid status: invalid. Valid values: pending_tx, active, completed, failed"
}
```

---

## Valores de Estado de Job

| Estado | Descripción |
|--------|-------------|
| `pending_tx` | Job creado en backend, esperando confirmación de transacción blockchain |
| `active` | Job confirmado on-chain, listo para que provers reclamen y computen |
| `completed` | Computación de job finalizada, resultados enviados |
| `failed` | Job falló (timeout, prueba inválida, etc.) |

---

## Casos de Uso

### 1. Dashboard Web UI
Mostrar actividad del marketplace en vivo:
```javascript
fetch('http://127.0.0.1:8080/api/jobs?status=active')
  .then(res => res.json())
  .then(data => {
    console.log(`${data.count} active jobs in marketplace`);
    data.jobs.forEach(job => {
      console.log(`Job ${job.job_id}: ${job.operation} - ${job.price_lamports} lamports`);
    });
  });
```

### 2. Monitorear Progreso de Job
Hacer polling para cambios de estado:
```bash
# Watch jobs transitioning to completed
watch -n 5 'curl -s "http://127.0.0.1:8080/api/jobs?status=completed" | jq ".count"'
```

### 3. Análisis
Calcular métricas del marketplace:
```bash
# Total jobs across all statuses
curl -s http://127.0.0.1:8080/api/jobs | jq ".count"

# Jobs by status
for status in pending_tx active completed failed; do
  count=$(curl -s "http://127.0.0.1:8080/api/jobs?status=$status" | jq ".count")
  echo "$status: $count"
done
```

---

## Notas de Arquitectura

### ¿Por qué este endpoint?

Mientras que **los provers descubren jobs directamente on-chain** vía `find_pending_jobs()` (fuente de verdad = blockchain), este endpoint proporciona:

1. **Web UI/UX** - Mostrar marketplace dinámicamente
2. **Monitoreo** - Rastrear salud del sistema y actividad
3. **Análisis** - Obtener insights sobre patrones de jobs
4. **Debugging** - Inspeccionar estados de jobs durante desarrollo

### Los provers no necesitan este endpoint

Los provers usan el `find_pending_jobs()` del SDK que consulta Solana directamente:

```rust
// Provers use this (on-chain discovery)
let jobs = find_pending_jobs(&rpc_client, &program_id)?;
```

Este endpoint es puramente para **clientes web** y **herramientas de monitoreo**.

---

## Consideraciones de Rendimiento

- **Query de Base de Datos:** Obtiene de la tabla PostgreSQL `temp_job_data`
- **Sin paginación:** Actualmente devuelve todos los jobs para el estado dado
- **Mejora futura:** Agregar paginación para conjuntos de resultados grandes (`?limit=100&offset=0`)

---

## Responses de Error

### 400 Bad Request - Estado Inválido
```json
{
  "error": "Invalid status: xyz. Valid values: pending_tx, active, completed, failed"
}
```

### 500 Internal Server Error - Problema de Base de Datos
```json
{
  "error": "Database error: connection failed"
}
```

---

## Endpoints Relacionados

- `POST /api/jobs/validate-and-build` - Crear un nuevo job
- `GET /api/jobs/{job_id}/status` - Obtener estado de job específico
- `GET /api/jobs/{job_id}/compute-data` - Obtener datos de computación FHE para provers
- `POST /api/jobs/{job_id}/confirm` - Confirmar transacción de job
- `DELETE /api/jobs/{job_id}` - Eliminar job completado/fallido

---

## Detalles de Implementación

**Archivo:** `blink-server/src/api_handlers.rs:377`

**Query de Base de Datos:** Usa `JobQueries::get_jobs_by_status()` que consulta:
```sql
SELECT * FROM temp_job_data WHERE status = $1 ORDER BY created_at DESC
```

**Formato de Response:** Convierte modelos `TempJobData` de base de datos a DTOs `JobListItem`.

---

## Testing

```bash
# Start backend
cd blink-server
cargo run --release

# In another terminal
curl http://127.0.0.1:8080/api/jobs | jq .

# Test with filters
curl "http://127.0.0.1:8080/api/jobs?status=active" | jq .
```

---

## Changelog

### 2025-11-22
- Implementación inicial del endpoint `GET /api/jobs`
- Parámetro de query opcional `status` con validación
- Devuelve lista de jobs con conteo
- Manejo de errores para valores de estado inválidos
