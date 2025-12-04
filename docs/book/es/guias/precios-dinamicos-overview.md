# Sistema de Precios Dinámicos - Centro de Documentación

Bienvenido a la documentación integral del Sistema de Precios Dinámicos de ZyberLink. Este documento sirve como punto de entrada para entender cómo se valoran las operaciones FHE en el mercado descentralizado.

---

## Navegación Rápida

### Para Lectores Primerizos
Comienza aquí para una visión clara de qué es el pricing dinámico y por qué importa:
- **[modelo-precios-dinamicos.md](./modelo-precios-dinamicos.md)** - Documentación principal con ejemplos y overview de arquitectura

### Para Desarrolladores Integrando el Sistema
Si estás construyendo sobre ZyberLink, estas guías tienen ejemplos de código:
- **[precios-dinamicos-uso.md](./precios-dinamicos-uso.md)** - Guía práctica de integración con ejemplos de SDK y API
- **[precios-dinamicos-tecnico.md](./precios-dinamicos-tecnico.md)** - Detalles técnicos de implementación profunda

---

## Estructura de Documentación

```
SISTEMA DE PRECIOS DINÁMICOS
├── modelo-precios-dinamicos.md (333 líneas)
│   ├── Overview y Por qué Precios Dinámicos
│   ├── 5 Niveles de Complejidad (O(1) a O(n×m))
│   ├── Arquitectura de un vistazo
│   ├── Ejemplos del mundo real
│   ├── Información de testing
│   └── Guía de inicio
│
├── precios-dinamicos-tecnico.md (814 líneas)
│   ├── Arquitectura completa del sistema
│   ├── Estructuras de datos principales
│   ├── Algoritmos de precios detallados (por tier)
│   ├── Flujos de validación (on-chain y off-chain)
│   ├── Puntos de integración
│   ├── Casos límite y limitaciones
│   ├── Análisis de rendimiento
│   └── Mejoras futuras
│
└── precios-dinamicos-uso.md (996 líneas)
    ├── Guía de creadores de jobs con ejemplos
    ├── Evaluación de rentabilidad para provers
    ├── Ejemplos de integración SDK (Rust)
    ├── Ejemplos de integración API (JS/TS, Python)
    ├── Integración frontend (Svelte)
    ├── Mejores prácticas
    ├── Guía de troubleshooting
    └── FAQ
```

**Total:** 2,143 líneas de documentación integral

---

## Sistema de un Vistazo

### ¿Qué es el Pricing Dinámico?

El Sistema de Precios Dinámicos de ZyberLink determina automáticamente el pago mínimo requerido para operaciones FHE basado en su complejidad computacional.

**El Problema:** Las tarifas planas tradicionales significan que las operaciones simples subsidian las complejas, causando que los provers pierdan dinero en operaciones costosas.

**La Solución:** Pricing basado en tiers donde:
- Las operaciones se categorizan 1-5 por complejidad
- Los costos escalan automáticamente con parámetros de operación
- La validación ocurre on-chain a nivel de protocolo
- Los provers pueden evaluar rentabilidad antes de aceptar jobs

### 5 Niveles de Complejidad

| Tier | Complejidad | Costo Base | Operaciones | Ejemplo |
|------|-----------|-----------|-----------|---------|
| **1** | O(1) | 0.001 SOL | Add, Multiply | `5 + encrypt(x)` |
| **2** | O(n) | 0.001 + 0.0001×n SOL | Sum | Suma de 1,000 valores |
| **3** | O(1) + bootstrap | 0.005 SOL | Threshold, RangeCheck | ¿Edad >= 18? |
| **4** | O(n) + predicados | 0.005 + 0.0003×n SOL | CountIf, Average | Contar edad > 21 |
| **5** | O(n×m) exponencial | 0.1 + 0.02×m^1.5 SOL | Histogram | Conteo de votos 5 opciones |

**Todos los precios son por prover.** Costo total = precio_tier × numero_de_provers

### Estadísticas Clave

- **54+ tests pasando** validando todos los escenarios de pricing
- **2,143 líneas** de documentación integral
- **5 puntos de integración**: Tipos compartidos, programa Solana, API Backend, UI Frontend, calculadora ROI
- **100% validación on-chain** - Jobs con precio bajo rechazados a nivel de protocolo

---

## Overview de Componentes

### 1. Lógica Central de Pricing
**Archivo:** `/shared/types/src/fhe.rs`

Fuente única de verdad para todos los cálculos de pricing. Cada operación FHE tiene:
```rust
pub fn get_cost_config(&self) -> OperationCostConfig {
    // Retorna: min_payment_lamports, timeout_seconds, complexity_tier
}
```

### 2. Validación On-Chain
**Archivo:** `/programs/zyberlink/src/processor/create_job.rs` (líneas 97-144)

El programa Solana valida precios durante la creación del job:
```
Usuario envía job → Programa calcula mínimo → Rechaza si precio bajo
```

### 3. API de Estimación de Costos del Backend
**Archivo:** `/blink-server/src/api_handlers.rs` (líneas 342-430)

Endpoint: `POST /api/estimate-cost`

Provee cotizaciones de costo en tiempo real antes del envío del job.

### 4. Calculadora ROI para Provers
**Archivo:** `/prover-node/src/roi_calculator.rs`

Los provers usan esto para evaluar rentabilidad del job:
```
revenue_per_prover = price / required_provers
profit = revenue - (cost × operational_overhead)
roi_percentage = (profit / cost) × 100
```

Solo se aceptan jobs rentables.

### 5. Integración Frontend
**Archivo:** `/frontend/src/lib/components/CreateJob.svelte`

UI dinámica mostrando costos en tiempo real a medida que los usuarios ajustan parámetros.

---

## Casos de Uso Comunes

### Caso de Uso 1: Verificación de Edad (ZK-Passport)

Verificar que el usuario es 18+ sin revelar la edad.

**Operación:** `Threshold { threshold: 18, greater_or_equal: true }`
**Tier:** 3
**Costo:** 0.005 SOL por prover
**3 Provers:** 0.015 SOL total
**Timeout:** 300 segundos (5 minutos)

### Caso de Uso 2: Conteo de Población Censo

Contar 50,000 votos cifrados sin revelar datos individuales.

**Operación:** `Sum { expected_count: 50000 }`
**Tier:** 2
**Costo:** 0.001 + (50,000 × 0.0001) = 5.001 SOL por prover
**3 Provers:** 15.003 SOL total
**Timeout:** 100,060 segundos (~27.8 horas)

### Caso de Uso 3: Conteo Privado de Elecciones

Contar votos para 10 candidatos.

**Operación:** `Histogram { bins: 10 }`
**Tier:** 5
**Costo:** 0.1 + (0.02 × 10^1.5) ≈ 0.732 SOL por prover
**3 Provers:** ~2.196 SOL total
**Timeout:** 1,600 segundos (~26.7 minutos)

---

## Comenzando

### Para Creadores de Jobs (1 minuto)

1. **Estimar costo antes de crear:**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

2. **Usar el costo retornado** como tu pago mínimo
3. **Crear el job** con fondos suficientes en escrow

Ver [precios-dinamicos-uso.md](./precios-dinamicos-uso.md#para-creadores-de-jobs) para ejemplos completos.

### Para Provers (2 minutos)

1. **Configurar tus requisitos ROI:**
```rust
let calculator = ROICalculator::new(
    20.0,   // 20% ROI mínimo
    1.5     // 50% overhead operacional
);
```

2. **Evaluar jobs antes de reclamar:**
```rust
let roi = calculator.evaluate_job(&circuit_type, price, provers);
if roi.is_profitable {
    claim_job(&job)?;
}
```

Ver [precios-dinamicos-uso.md](./precios-dinamicos-uso.md#para-provers) para guía completa de provers.

### Para Desarrolladores (5 minutos)

1. **Calcular precio mínimo:**
```rust
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;
```

2. **Usar timeout dinámico:**
```rust
let ix = builder.create_fhe_job(
    creator,
    job_id,
    &encrypted_data,
    fhe_config,
    min_price,                      // o mayor
    cost_config.timeout_seconds     // ¡usar esto!
)?;
```

Ver [precios-dinamicos-uso.md](./precios-dinamicos-uso.md#ejemplos-de-integracion-sdk) para ejemplos de SDK.

---

## Overview de Testing

El sistema incluye cobertura de tests integral:

```
✅ Tipos Compartidos: 57 tests unitarios
   - Validación de pricing Tier 1-5
   - Tests de serialización
   - Verificación de escalado de costos
   - Tests de escenarios realistas (censo, votación, verificación edad)

✅ Tests de Integración: 4 tests
   - Validación de precio on-chain
   - Rechazo de jobs con precio bajo
   - Escalado de conteo de provers

✅ Tests E2E: 7 tests (en test-dynamic-pricing-e2e.sh)
   - Estimación de costos API
   - Integración con base de datos
   - Flujo end-to-end de job

✅ Calculadora ROI: 4 tests
   - Cálculos de rentabilidad
   - Cómputo de precio mínimo
   - Manejo de overhead operacional

TOTAL: 72 tests pasando
```

Ejecutar todos los tests:
```bash
cd /home/deploy/experimental/zyberlink-demo
bash test-dynamic-pricing-e2e.sh
```

---

## Referencia Rápida de Documentos

### Documentación Principal (modelo-precios-dinamicos.md)

**Lee esto para:** Overview de alto nivel, arquitectura, por qué importa el pricing dinámico

**Secciones clave:**
- Overview y declaración del problema
- 5 tiers de complejidad con ejemplos de pricing
- Casos de uso del mundo real (4 ejemplos)
- Checklist de estado de implementación
- Overview de testing
- Guía de inicio
- Garantías de seguridad

### Implementación Técnica (precios-dinamicos-tecnico.md)

**Lee esto para:** Detalles de algoritmos, estructuras de datos, flujos de validación

**Secciones clave:**
- Jerarquía completa de componentes (4 capas)
- Estructuras de datos principales (OperationCostConfig, FheOperation)
- Algoritmo de pricing detallado por tier
- Flujos de validación on-chain y off-chain
- Puntos de integración con código
- Casos límite y limitaciones
- Análisis de rendimiento
- Estrategia de testing

### Guía de Uso (precios-dinamicos-uso.md)

**Lee esto para:** Ejemplos de código, integración práctica, troubleshooting

**Secciones clave:**
- Guía de creadores de jobs con ejemplos
- Evaluación de rentabilidad para provers
- Ejemplos de integración SDK (Rust)
- Ejemplos de integración API (JS/TS, Python)
- Integración frontend (Svelte)
- Checklist de mejores prácticas
- Guía de troubleshooting
- FAQ (15+ preguntas)

---

## Datos Clave

### Fórmula de Pricing por Tier

| Tier | Fórmula | Fórmula de Timeout |
|------|---------|-----------------|
| **1** | Base = 0.001 SOL | 60s |
| **2** | 0.001 + (n × 0.0001) SOL | 60 + (n × 2)s |
| **3** | 0.005 SOL | 300s |
| **4a** (Average) | 0.001 + (n × 0.0002) SOL | 60 + (n × 3)s |
| **4b** (CountIf) | 0.005 + (n × 0.0003) SOL | 300 + (n × 5)s |
| **5** | 0.1 + (0.02 × m^1.5) SOL | 600 + (m × 100)s |

### La Validación Ocurre En

1. **API Backend** - Antes de firmar (UX: rechazo temprano)
2. **Programa Solana** - Durante creación del job (Seguridad: nivel de protocolo)
3. **Nodo Prover** - Antes de reclamar (Economía: verificación ROI)

### Archivos a Conocer

| Archivo | Propósito | Estado |
|------|---------|--------|
| `/shared/types/src/fhe.rs` | Lógica central de pricing | ✅ Implementado (8 operaciones) |
| `/programs/zyberlink/src/processor/create_job.rs` | Validación on-chain | ✅ Implementado (líneas 97-144) |
| `/blink-server/src/api_handlers.rs` | API de estimación de costos | ✅ Implementado (líneas 342-430) |
| `/prover-node/src/roi_calculator.rs` | Calculadora de rentabilidad | ✅ Implementado |
| `/frontend/src/lib/components/CreateJob.svelte` | UI Frontend | ✅ Integrado |
| `/test-dynamic-pricing-e2e.sh` | Suite de tests E2E | ✅ 7 tests pasando |

---

## Preguntas Frecuentes

**P: ¿Qué pasa si el precio de SOL cambia?**
R: El pricing está en lamports (moneda on-chain), no USD. Si el precio de SOL se duplica, el costo en lamports permanece igual, pero el costo en USD se duplica.

**P: ¿Puedo pagar más que el mínimo?**
R: ¡Sí! Pagos mayores pueden incentivar respuesta más rápida del prover y actuar como tarifas de prioridad.

**P: ¿Puedo usar un timeout personalizado?**
R: Sí, pero debe ser >= al timeout dinámico del config de costos de la operación.

**P: ¿Los provers negocian precios?**
R: Actualmente no. Los precios son determinísticos basados en complejidad. Una mejora futura podría agregar sistema de ofertas.

**P: ¿Hay descuentos por volumen?**
R: Actualmente no. Cada job paga el mismo costo por prover. El futuro podría implementar descuentos por volumen.

Ver [precios-dinamicos-uso.md#faq](./precios-dinamicos-uso.md#faq) para 15+ preguntas más.

---

## Diagrama de Arquitectura

```
Interfaz de Usuario (Frontend Svelte)
    │
    ├──> Estima Costo
    │    POST /api/estimate-cost
    │    ↓
    │    API Backend (Actix-web)
    │    ├─> Parsea parámetros de operación
    │    ├─> Llama FheOperation::get_cost_config()
    │    └─> Retorna: {tier, price, timeout, compute_ms}
    │
    ├──> Crea Job
    │    Firma transacción
    │    ↓
    │    Blockchain Solana
    │    ├─> Recibe instrucción create_job
    │    ├─> Valida: price >= cost_config.min × provers
    │    ├─> Rechaza si precio bajo
    │    └─> Crea cuenta de job si válido
    │
Nodo Prover
    ├─> Ve job disponible
    ├─> Evalúa con ROICalculator
    ├─> Verifica: roi_percentage >= min_roi_threshold
    ├─> Acepta si rentable
    └─> Ejecuta cómputo FHE
```

---

## Tips de Navegación

1. **¿Perdido?** Comienza con [modelo-precios-dinamicos.md](./modelo-precios-dinamicos.md)
2. **¿Necesitas ejemplos?** Salta a [precios-dinamicos-uso.md](./precios-dinamicos-uso.md)
3. **¿Quieres detalles?** Lee [precios-dinamicos-tecnico.md](./precios-dinamicos-tecnico.md)
4. **¿Tienes preguntas?** Revisa [precios-dinamicos-uso.md#faq](./precios-dinamicos-uso.md#faq)

---

## Estado del Sistema

| Componente | Tests | Estado |
|-----------|-------|--------|
| Tipos Centrales | 57 | ✅ Pasando |
| Integración | 4 | ✅ Pasando |
| E2E | 7 | ✅ Pasando |
| Calculadora ROI | 4 | ✅ Pasando |
| **TOTAL** | **72** | **✅ Pasando** |

**Versión:** 1.0.0
**Última Actualización:** 2025-11-21
**Mantenido Por:** Equipo ZyberLink Core

---

## Contribuyendo

¿Encontraste un problema en la lógica de pricing o documentación?

1. Revisa la sección de documentación relevante
2. Verifica [precios-dinamicos-tecnico.md](./precios-dinamicos-tecnico.md#casos-limite-y-limitaciones) para limitaciones conocidas
3. Abre un issue en GitHub con información detallada

---

## Documentación Relacionada

- **[referencia-api-completa.md](./referencia-api-completa.md)** - Documentación completa de API
- **[optimizacion-histograma-fhe.md](../arquitectura/optimizacion-histograma-fhe.md)** - Deep dive de complejidad Tier 5
- **[SUMMARY.md](../SUMMARY.md)** - Overview del proyecto

---

**¿Listo para comenzar?**

- **Creadores:** Salta a [precios-dinamicos-uso.md - Para Creadores de Jobs](./precios-dinamicos-uso.md#para-creadores-de-jobs)
- **Provers:** Salta a [precios-dinamicos-uso.md - Para Provers](./precios-dinamicos-uso.md#para-provers)
- **Desarrolladores:** Salta a [precios-dinamicos-uso.md - Integración SDK](./precios-dinamicos-uso.md#ejemplos-de-integracion-sdk)
