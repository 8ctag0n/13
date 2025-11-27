# Sistema de Precios Dinámicos

**ZyberLink Marketplace - Precios Inteligentes de Operaciones FHE**

---

## Descripción General

El Sistema de Precios Dinámicos es un mecanismo integral de fijación de precios para el mercado descentralizado de ZyberLink que determina automáticamente el pago mínimo requerido para operaciones de Cifrado Completamente Homomórfico (FHE) basándose en su complejidad computacional.

En lugar de usar una tarifa plana para todas las operaciones, ZyberLink implementa un **modelo de precios basado en niveles** donde cada operación FHE tiene un nivel de costo predeterminado (1-5) que refleja sus requisitos computacionales, con precios que escalan automáticamente según los parámetros de la operación y el número de provers requeridos.

## ¿Por Qué Precios Dinámicos?

### El Problema

Los mercados de cómputo tradicionales enfrentan varios desafíos:

1. **Los precios planos no reflejan el costo computacional** - Las operaciones simples subsidian a las complejas
2. **Los provers pierden dinero en operaciones costosas** - Llevando al abandono de trabajos y mala calidad de servicio
3. **El descubrimiento de precios es manual** - Los usuarios no conocen precios justos antes de crear trabajos
4. **No hay protección contra precios bajos** - Sistema vulnerable a spam y ataques DoS

### La Solución

El Sistema de Precios Dinámicos de ZyberLink aborda estos problemas mediante:

- **Cálculo automático de costos** basado en complejidad de operación (O(1) a O(n×m))
- **Niveles de precios transparentes** (1-5) visibles para todos los participantes
- **Agregación de costos multi-prover** - El costo total escala linealmente con el conteo de provers
- **Validación on-chain** - El programa Solana rechaza trabajos con precios bajos
- **Calculadora ROI** para que provers evalúen la rentabilidad de trabajos
- **API de estimación de costos en tiempo real** para integración frontend

## Arquitectura de un Vistazo

```
┌─────────────────────────────────────────────────────────────────┐
│                     FLUJO DE PRECIOS DINÁMICOS                  │
└─────────────────────────────────────────────────────────────────┘

Frontend (Svelte)                Backend API               Programa Solana
     │                                │                           │
     │  1. Seleccionar Operación     │                           │
     │     (Add, Histogram, etc)      │                           │
     │                                │                           │
     │  2. POST /api/estimate-cost   │                           │
     ├──────────────────────────────>│                           │
     │                                │                           │
     │                                │  3. FheOperation          │
     │                                │     .get_cost_config()    │
     │                                │                           │
     │  4. Respuesta de Costo         │                           │
     │     (tier, price, timeout)     │                           │
     │<──────────────────────────────┤                           │
     │                                │                           │
     │  5. Mostrar costo total        │                           │
     │     Usuario confirma           │                           │
     │                                │                           │
     │  6. Crear TX de Trabajo        │                           │
     ├───────────────────────────────────────────────────────────>│
     │                                │                           │
     │                                │  7. Validar precio        │
     │                                │     cost_config.min × n   │
     │                                │                           │
     │  8. Éxito/Error                │                           │
     │<───────────────────────────────────────────────────────────┤
     │                                │                           │
```

## Niveles de Complejidad

ZyberLink categoriza las operaciones FHE en 5 niveles de complejidad:

| Nivel | Complejidad | Operaciones | Precio/Prover | Timeout | Casos de Uso |
|------|------------|------------|--------------|---------|-----------|
| **1** | O(1) | Add, Multiply | 0.001 SOL | 60s | Aritmética simple |
| **2** | O(n) | Sum | 0.001 + n×0.0001 SOL | 60 + n×2s | Conteo censal |
| **3** | O(1) + bootstrap | Threshold, RangeCheck | 0.005 SOL | 300s | Verificación de edad |
| **4** | O(n) + predicados | Average, CountIf | Variable | Variable | Estadísticas condicionales |
| **5** | O(n×m) | Histogram | 0.1 + 0.02×m^1.5 SOL | 600 + m×100s | Votación, distribuciones |

**Nota:** Todos los precios son **por prover**. Costo total del trabajo = precio_nivel × número_de_provers.

## Ejemplos de Precios

### Ejemplo 1: Suma Simple (Nivel 1)
```
Operación: Add(5)
Provers: 3
───────────────────────────
Costo por prover: 0.001 SOL (1,000,000 lamports)
Mínimo total:     0.003 SOL (3,000,000 lamports)
Timeout:          60 segundos
```

### Ejemplo 2: Verificación de Edad (Nivel 3)
```
Operación: Threshold { threshold: 18, greater_or_equal: true }
Provers: 3
───────────────────────────
Costo por prover: 0.005 SOL (5,000,000 lamports)
Mínimo total:     0.015 SOL (15,000,000 lamports)
Timeout:          300 segundos (5 minutos)
```

### Ejemplo 3: Histograma de Votación (Nivel 5)
```
Operación: Histogram { bins: 5 }  // 5 candidatos
Provers: 3
───────────────────────────
Costo por prover: ~0.324 SOL (324,000,000 lamports)
Mínimo total:     ~0.972 SOL (972,000,000 lamports)
Timeout:          1100 segundos (~18 minutos)
```

### Ejemplo 4: Suma Censal (Nivel 2 - Escala con conteo)
```
Operación: Sum { expected_count: 10,000 }
Provers: 3
───────────────────────────
Costo por prover: 1.001 SOL (1,001,000,000 lamports)
  Base: 0.001 SOL
  Por item: 10,000 × 0.0001 SOL = 1.0 SOL
Mínimo total:   3.003 SOL (3,003,000,000 lamports)
Timeout:        20,060 segundos (~5.5 horas)
```

## Características Principales

### 1. Configuración Automática de Costos

Cada `FheOperation` tiene un método integrado `get_cost_config()` que devuelve:

```rust
pub struct OperationCostConfig {
    pub min_payment_lamports: u64,  // Precio mínimo por prover
    pub timeout_seconds: i64,        // Timeout dinámico
    pub complexity_tier: u8,         // Nivel 1-5
}
```

### 2. Validación On-Chain

El programa Solana valida los precios durante la creación del trabajo:

```rust
// En el procesador create_job (líneas 109-128)
let cost_config = fhe_op.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (required_provers as u64);

if price_lamports < total_min_price {
    return Err(ZyberLinkProgramError::InvalidPrice);
}
```

**Resultado:** Los trabajos con precios bajos son rechazados a nivel de protocolo.

### 3. API de Estimación de Costos

El backend proporciona el endpoint `/api/estimate-cost` para precios en tiempo real:

```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 10,
    "required_provers": 3
  }'
```

**Respuesta:**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 732050808,
  "min_payment_sol": 0.732050808,
  "total_min_payment_lamports": 2196152424,
  "total_min_payment_sol": 2.196152424,
  "timeout_seconds": 1600,
  "estimated_compute_ms": 30500
}
```

### 4. Calculadora ROI para Provers

Los nodos prover incluyen una calculadora ROI inteligente para evaluar la rentabilidad del trabajo:

```rust
let calculator = ROICalculator::new(20.0, 1.5);  // 20% ROI mínimo, 1.5x overhead
let roi = calculator.evaluate_job(&circuit_type, price_lamports, required_provers);

if roi.is_profitable {
    println!("ROI Esperado: {:.1}%", roi.roi_percentage);
    // Aceptar trabajo
}
```

### 5. Integración Frontend

La UI de Crear Trabajo muestra dinámicamente los costos mientras los usuarios configuran las operaciones:

- **Dropdown de operación** - Seleccionar operación FHE
- **Entradas de parámetros** - Parámetros específicos de operación (bins, thresholds, etc.)
- **Slider de provers** - Ajustar número de provers
- **Visualización de costo en tiempo real** - Muestra el costo total en SOL antes de enviar
- **Indicadores de advertencia** - Alertas para operaciones costosas

## Estado de Implementación

| Componente | Estado | Ubicación |
|-----------|--------|----------|
| Tipos Core | ✅ Implementado | `/shared/types/src/fhe.rs` |
| Validación Solana | ✅ Implementado | `/programs/zyberlink/src/processor/create_job.rs` |
| API Backend | ✅ Implementado | `/blink-server/src/api_handlers.rs` (líneas 342-432) |
| UI Frontend | ✅ Implementado | `/frontend/src/lib/components/CreateJob.svelte` |
| Builders SDK | ✅ Implementado | `/sdk/src/instructions/marketplace.rs` |
| Calculadora ROI | ✅ Implementado | `/prover-node/src/roi_calculator.rs` |
| Tests Unitarios | ✅ 57 tests pasando | `/shared/types/src/fhe.rs` |
| Tests de Integración | ✅ 4 tests pasando | `/programs/zyberlink/tests/dynamic_pricing_tests.rs` |
| Script E2E | ✅ Implementado | `/test-dynamic-pricing-e2e.sh` |

## Testing

### Ejecutar Todos los Tests

```bash
# Tests unitarios (shared types)
cd /home/deploy/experimental/zyberlink-demo/shared/types
cargo test --lib

# Tests de integración del programa
cd /home/deploy/experimental/zyberlink-demo/programs/zyberlink
cargo test-sbf

# Tests de calculadora ROI
cd /home/deploy/experimental/zyberlink-demo/prover-node
cargo test roi_calculator::tests

# Test E2E (requiere backend ejecutándose)
cd /home/deploy/experimental/zyberlink-demo
./test-dynamic-pricing-e2e.sh
```

### Cobertura de Tests

- **57 tests unitarios** en `shared/types` cubriendo todos los 5 niveles
- **4 tests de integración** validando aplicación de precios on-chain
- **7 tests E2E** verificando estimación de costos API
- **4 tests de calculadora ROI** asegurando cálculos de rentabilidad

## Primeros Pasos

### Para Creadores de Trabajos

1. **Estimar costos** antes de crear trabajos:
   ```bash
   curl -X POST http://localhost:8080/api/estimate-cost \
     -H "Content-Type: application/json" \
     -d '{"operation": "add", "operation_value": 5, "required_provers": 3}'
   ```

2. **Usar SDK** con precios validados:
   ```rust
   let operation = FheOperation::Add(5);
   let cost_config = operation.get_cost_config();
   let total_price = cost_config.min_payment_lamports * 3;  // 3 provers

   let ix = builder.create_fhe_job(
       creator,
       job_id,
       &encrypted_data,
       fhe_config,
       total_price,
       cost_config.timeout_seconds
   )?;
   ```

3. **Integración frontend** - Usar el componente CreateJob con estimación en tiempo real

### Para Provers

1. **Configurar umbrales ROI** en nodo prover:
   ```rust
   let calculator = ROICalculator::new(
       15.0,   // 15% ROI mínimo
       1.2     // 20% overhead operacional
   );
   ```

2. **Evaluar trabajos** antes de reclamar:
   ```rust
   let roi = calculator.evaluate_job(circuit_type, price, provers);
   if !roi.is_profitable {
       log::warn!("Trabajo no rentable, omitiendo");
       continue;
   }
   ```

## Garantías de Seguridad

1. **Aplicación a nivel de protocolo** - Trabajos con precios bajos rechazados on-chain
2. **Protección contra front-running** - Los precios mínimos son determinísticos
3. **Protección de provers** - Calculadora ROI previene trabajo no rentable
4. **Mitigación DoS** - Operaciones costosas requieren pago proporcional

## Hoja de Ruta

### Fase 7 (Futuro)
- [ ] Ajuste dinámico de niveles basado en carga de red
- [ ] Dashboard de análisis de precios históricos
- [ ] Sistema de puja de provers para tarifas premium
- [ ] Precios multi-token (soporte USDC, wZEC para precios dinámicos)

## Lecturas Adicionales

- **[Guía de Implementación Técnica](./precios-dinamicos-tecnico.md)** - Inmersión profunda en algoritmos y estructuras de datos
- **[Guía de Uso con Ejemplos](./precios-dinamicos-uso.md)** - Ejemplos prácticos de integración
- **[Referencia API](./referencia-api-completa.md)** - Documentación API completa
- **[Optimización de Histograma](./optimizacion-histograma-fhe.md)** - Análisis de complejidad Nivel 5

## Contribuir

¿Encontraste un problema con los precios? ¿Ves costos incorrectos? Abre un issue o PR en [ZyberLink GitHub](https://github.com/yourorg/zyberlink).

---

**Última Actualización:** 2025-11-21
**Versión:** 1.0.0
**Estado:** Listo para Producción
