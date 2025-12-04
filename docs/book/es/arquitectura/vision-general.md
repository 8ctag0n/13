# Visión General del Sistema

ZyberLink es un marketplace descentralizado para cómputo que preserva privacidad sobre Solana.

## Concepto Central

Las aplicaciones pueden externalizar operaciones FHE/ZK pesadas a una red de provers independientes. El consenso on-chain asegura corrección y automatiza pagos.

**Innovación clave:** el consenso multi-prover elimina el punto único de falla manteniendo garantías criptográficas.

## Arquitectura Alto Nivel

```
┌──────────────────────────────────────────────────────────┐
│                Aplicación Cliente                        │
└────────────────────┬─────────────────────────────────────┘
                     │ 1. Crear job encriptado
                     ▼
┌──────────────────────────────────────────────────────────┐
│        Programa Marketplace en Solana                    │
│  - Queue de jobs                                         │
│  - Registro de provers                                   │
│  - Verificación de consenso                              │
│  - Distribución de pagos                                 │
└────────────────────┬─────────────────────────────────────┘
        ┌────────────┼────────────┐
        ▼            ▼            ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Prover A │  │ Prover B │  │ Prover C │
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │ 3. Ejecutan de forma independiente
     ▼
 result_A / result_B / result_C
                     ▼
┌──────────────────────────────────────────────────────────┐
│         Verificación de Consenso                         │
│ Si 2+ provers coinciden → Aceptar y pagar                │
│ Si hay mismatch → Penalizar prover deshonesto            │
└──────────────────────────────────────────────────────────┘
```

## Componentes del Sistema

### 1. SDK Cliente
Interfaz sencilla para crear jobs, encriptar datos y obtener resultados. Soporta Rust (JS/Python planificado).

### 2. Programa Marketplace en Solana
Coordina distribución de jobs, consenso y pagos:
- Queue de trabajos pendientes
- Registro y capacidad de provers
- Motor de consenso (umbral 2-de-3 o 3-de-5)
- Escrow y pagos automáticos

### 3. Nodos Prover
Ejecutan computación FHE/ZK con TFHE-rs y envían resultados. Pueden correr con TUI para monitoreo.

### 4. Interfaz Web/TUI
Panel para crear jobs, subir `witness.bin`, seguir progreso y ver estadísticas de provers.

## Beneficios
- **Sin confianza en un único prover**: resiliencia a fallas y ataques.
- **Privacidad preservada**: datos siempre encriptados.
- **Pagos automáticos**: incentivos alineados y penalización a resultados incorrectos.
- **Escala horizontal**: más provers → más throughput.

## Limitaciones
- Requiere mayoría honesta (no tolera colusión de todos los provers).
- El costo/latencia FHE sigue siendo alto para jobs grandes.
