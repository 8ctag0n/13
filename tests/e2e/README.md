# E2E Tests - ZyberLink

## Overview

Este directorio contiene tests end-to-end para el marketplace de ZyberLink, incluyendo tests completos para la funcionalidad FHE (Fully Homomorphic Encryption).

## Tipos de Tests

### 1. Tests Self-Executing (Recomendados)

**Archivo:** `tests/fhe_self_executing.rs`

Tests que usan `solana-program-test` para crear un validator in-memory. **No requieren validator externo**.

**Ventajas:**
- ⚡ Rápidos (~5 minutos para suite completa)
- 🎯 Determinísticos (estado fresh cada test)
- ✅ Perfectos para CI/CD
- 🔧 No requieren setup manual

**Ejecutar:**
```bash
# Todos los tests self-executing
cargo test --test fhe_self_executing

# Test específico  
cargo test --test fhe_self_executing test_create_fhe_job_self_executing -- --nocapture

# Con output detallado
cargo test --test fhe_self_executing -- --nocapture --test-threads=1
```

**Cobertura:**
- ✅ Create FHE Job
- ✅ Multi-Prover Claiming (3 provers)
- ✅ FHE Result Submission
- ✅ Consensus and Finalization (2-of-3)

**Status:** 14/14 tests passing ✅

### 2. Tests con Validator Externo

**Archivo:** `tests/fhe_job_e2e.rs`

Tests originales que requieren `solana-test-validator` corriendo externamente.

**Marcados con `#[ignore]`** porque los tests self-executing proveen la misma cobertura sin necesidad de validator externo.

## Documentación Adicional

- **SELF_EXECUTING_TESTS.md** - Guía detallada de uso
- **docs/research/fhe-integration-status.md** - Reporte técnico completo

## Quick Start

```bash
# Run all self-executing FHE tests (no external validator needed)
cargo test --test fhe_self_executing

# Expected output:
# test result: ok. 14 passed; 0 failed; 0 ignored
# Finished in ~5 minutes
```

**Status:** 14/14 self-executing tests passing ✅

**Last Updated:** 2025-11-16
