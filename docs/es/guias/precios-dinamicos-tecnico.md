# Sistema de Precios Dinámicos - Guía de Implementación Técnica

**Inmersión Profunda en Arquitectura, Algoritmos y Estructuras de Datos**

---

## Tabla de Contenidos

1. [Arquitectura del Sistema](#arquitectura-del-sistema)
2. [Estructuras de Datos Core](#estructuras-de-datos-core)
3. [Algoritmo de Precios](#algoritmo-de-precios)
4. [Flujo de Validación](#flujo-de-validación)
5. [Puntos de Integración](#puntos-de-integración)
6. [Casos Extremos y Limitaciones](#casos-extremos-y-limitaciones)
7. [Consideraciones de Rendimiento](#consideraciones-de-rendimiento)

---

## Arquitectura del Sistema

### Jerarquía de Componentes

```
┌─────────────────────────────────────────────────────────────────┐
│                      CAPA 1: TIPOS COMPARTIDOS                  │
│                 (/shared/types/src/fhe.rs)                      │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ enum FheOperation                                       │    │
│  │  - Add(u8)                                              │    │
│  │  - Multiply(u8)                                         │    │
│  │  - Sum { expected_count }                               │    │
│  │  - Threshold { threshold, greater_or_equal }            │    │
│  │  - RangeCheck { min, max }                              │    │
│  │  - Average { expected_count }                           │    │
│  │  - CountIf { predicate, expected_count }                │    │
│  │  - Histogram { bins }                                   │    │
│  │                                                          │    │
│  │ impl FheOperation {                                      │    │
│  │     fn get_cost_config(&self) -> OperationCostConfig    │    │
│  │     fn name(&self) -> &str                              │    │
│  │     fn estimated_compute_time_ms(&self) -> u32          │    │
│  │ }                                                        │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ struct OperationCostConfig                              │    │
│  │  - min_payment_lamports: u64                            │    │
│  │  - timeout_seconds: i64                                 │    │
│  │  - complexity_tier: u8                                  │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Importar
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   CAPA 2: PROGRAMA SOLANA                       │
│         (/programs/zyberlink/src/processor/)                   │
│                                                                  │
│  create_job.rs (líneas 98-144):                                 │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ match circuit_type {                                    │    │
│  │   CircuitType::FheComputation(fhe_op) => {              │    │
│  │     let cost_config = fhe_op.get_cost_config();         │    │
│  │     let min = cost_config.min_payment_lamports;         │    │
│  │     let total = min * required_provers;                 │    │
│  │                                                          │    │
│  │     if price_lamports < total {                         │    │
│  │       return Err(InvalidPrice);                         │    │
│  │     }                                                    │    │
│  │   }                                                      │    │
│  │ }                                                        │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Paralelo
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CAPA 3: API BACKEND                          │
│              (/blink-server/src/api_handlers.rs)                │
│                                                                  │
│  POST /api/estimate-cost (líneas 342-432):                      │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ 1. Parsear operación de JSON de solicitud               │    │
│  │ 2. Construir variante enum FheOperation                 │    │
│  │ 3. Llamar operation.get_cost_config()                   │    │
│  │ 4. Calcular total = min × required_provers              │    │
│  │ 5. Retornar respuesta JSON con detalles de costo        │    │
│  └────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ API HTTP
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CAPA 4: UI FRONTEND                          │
│         (/frontend/src/lib/components/CreateJob.svelte)         │
│                                                                  │
│  - Selector dropdown de operación                               │
│  - Entradas dinámicas de parámetros (bins, thresholds, etc.)    │
│  - Slider de conteo de provers                                  │
│  - Visualización de estimación de costo en tiempo real          │
│  - Validación de envío                                          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Estructuras de Datos Core

### 1. OperationCostConfig

**Ubicación:** `/shared/types/src/fhe.rs` (líneas 5-17)

```rust
#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct OperationCostConfig {
    /// Pago mínimo requerido en lamports POR PROVER
    pub min_payment_lamports: u64,

    /// Timeout para esta operación en segundos
    pub timeout_seconds: i64,

    /// Nivel de complejidad (1-5, donde 5 es más complejo)
    pub complexity_tier: u8,
}
```

**Propiedades Clave:**
- **Serialización:** Soporta tanto Borsh (on-chain) como Serde (off-chain)
- **Tamaño:** 21 bytes (8 + 8 + 1 + padding)
- **Inmutabilidad:** Retornado por valor desde `get_cost_config()`, previniendo modificación

### 2. Enum FheOperation

**Ubicación:** `/shared/types/src/fhe.rs` (líneas 62-106)

```rust
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FheOperation {
    Add(u8),
    Multiply(u8),
    Sum { expected_count: u16 },
    Threshold { threshold: u8, greater_or_equal: bool },
    RangeCheck { min: u8, max: u8 },
    Average { expected_count: u16 },
    CountIf { predicate: FhePredicate, expected_count: u16 },
    Histogram { bins: Vec<HistogramBin> },
}
```

**Análisis de Tamaño de Variante:**

| Variante | Discriminante | Datos | Tamaño Total |
|---------|--------------|------|------------|
| Add | 1 byte | 1 byte | 2 bytes |
| Multiply | 1 byte | 1 byte | 2 bytes |
| Sum | 1 byte | 2 bytes | 3 bytes |
| Threshold | 1 byte | 2 bytes | 3 bytes |
| RangeCheck | 1 byte | 2 bytes | 3 bytes |
| Average | 1 byte | 2 bytes | 3 bytes |
| CountIf | 1 byte | predicate + 2 | ~5 bytes |
| Histogram | 1 byte | Vec header + bins | Variable |

**Importante:** Histogram es la única variante con tamaño no limitado debido a `Vec<HistogramBin>`.

---

## Algoritmo de Precios

### Nivel 1: Operaciones Aritméticas O(1)

**Operaciones:** `Add(u8)`, `Multiply(u8)`

**Algoritmo:**
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 155-159)
FheOperation::Add(_) | FheOperation::Multiply(_) => OperationCostConfig {
    min_payment_lamports: LAMPORTS_PER_SOL / 1000,  // 0.001 SOL
    timeout_seconds: 60,
    complexity_tier: 1,
}
```

**Justificación:**
- **Tiempo constante:** Sin bucles, número fijo de operaciones FHE
- **Bootstrapping:** Mínimo (1-2 operaciones bootstrap)
- **Memoria:** Tamaño ciphertext O(1)
- **Benchmark:** ~150-200ms en AMD Ryzen 7900X

### Nivel 2: Agregación Lineal O(n)

**Operaciones:** `Sum { expected_count }`

**Algoritmo:**
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 164-174)
FheOperation::Sum { expected_count } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 1000;        // 0.001 SOL
    let per_item_cost = LAMPORTS_PER_SOL / 10000;   // 0.0001 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 60 + (n as i64 * 2),
        complexity_tier: 2,
    }
}
```

**Fórmula de Costo:**
```
Costo(Sum, n) = 0.001 + (n × 0.0001) SOL
Timeout(Sum, n) = 60 + (n × 2) segundos
```

**Ejemplos:**
- `Sum { expected_count: 10 }` → 0.002 SOL, 80s
- `Sum { expected_count: 100 }` → 0.011 SOL, 260s
- `Sum { expected_count: 10000 }` → 1.001 SOL, 20060s

**Justificación:**
- Escalado de costo lineal con tamaño de entrada
- Costo base cubre setup + extracción de resultado
- Costo por item refleja adición homomórfica

### Nivel 3: Operaciones O(1) + Bootstrap

**Operaciones:** `Threshold`, `RangeCheck`

**Algoritmo:**
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 178-184)
FheOperation::Threshold { .. } | FheOperation::RangeCheck { .. } => {
    OperationCostConfig {
        min_payment_lamports: LAMPORTS_PER_SOL / 200,  // 0.005 SOL
        timeout_seconds: 300,  // 5 minutos
        complexity_tier: 3,
    }
}
```

**Justificación:**
- **Bootstrapping costoso:** Operaciones de comparación requieren múltiples bootstraps
- **Tamaño de entrada constante:** Threshold es comparación de valor único
- **Timeout mayor:** Operaciones bootstrap son más lentas (~300ms cada una)
- **Caso de uso:** Verificación de edad, verificación de límites de valor

### Nivel 4: O(n) + Predicados/División

**Operaciones:** `Average { expected_count }`, `CountIf { predicate, expected_count }`

#### Algoritmo Average:
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 187-197)
FheOperation::Average { expected_count } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 1000;        // 0.001 SOL
    let per_item_cost = LAMPORTS_PER_SOL / 5000;    // 0.0002 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 60 + (n as i64 * 3),
        complexity_tier: 4,
    }
}
```

**Fórmula de Costo:**
```
Costo(Average, n) = 0.001 + (n × 0.0002) SOL
Timeout(Average, n) = 60 + (n × 3) segundos
```

#### Algoritmo CountIf:
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 199-209)
FheOperation::CountIf { expected_count, .. } => {
    let n = *expected_count as u64;
    let base_cost = LAMPORTS_PER_SOL / 200;         // 0.005 SOL
    let per_item_cost = (LAMPORTS_PER_SOL * 3) / 10_000;  // 0.0003 SOL

    OperationCostConfig {
        min_payment_lamports: base_cost + (n * per_item_cost),
        timeout_seconds: 300 + (n as i64 * 5),
        complexity_tier: 4,
    }
}
```

**Fórmula de Costo:**
```
Costo(CountIf, n) = 0.005 + (n × 0.0003) SOL
Timeout(CountIf, n) = 300 + (n × 5) segundos
```

**Justificación:**
- Costo por item mayor que Nivel 2 debido a evaluación de predicados
- CountIf requiere comparación para cada elemento
- Average requiere división (aproximada en FHE)

### Nivel 5: Complejidad Exponencial O(n×m)

**Operación:** `Histogram { bins }`

**Algoritmo:**
```rust
// Ubicación: /shared/types/src/fhe.rs (líneas 215-232)
FheOperation::Histogram { bins } => {
    let m = bins.len() as u64;

    // Costo base para operación histogram
    let base_cost = LAMPORTS_PER_SOL / 10;  // 0.1 SOL

    // Costo exponencial escala con bins^1.5
    // Usando m^1.5 = sqrt(m^3) para matemática entera
    let m_cubed = m * m * m;
    let m_power_1_5 = (m_cubed as f64).sqrt() as u64;
    let exponential_cost = (LAMPORTS_PER_SOL / 50) * m_power_1_5;

    OperationCostConfig {
        min_payment_lamports: base_cost + exponential_cost,
        timeout_seconds: 600 + (m as i64 * 100),
        complexity_tier: 5,
    }
}
```

**Fórmula de Costo:**
```
Costo(Histogram, m) = 0.1 + (0.02 × m^1.5) SOL
Timeout(Histogram, m) = 600 + (m × 100) segundos
```

**Ejemplos:**
| Bins | Costo/Prover | Timeout | Justificación |
|------|-------------|---------|-----------|
| 3 | ~0.204 SOL | 900s | Votación pequeña (3 opciones) |
| 5 | ~0.324 SOL | 1100s | Votación mediana (5 candidatos) |
| 10 | ~0.732 SOL | 1600s | Distribución grande |
| 20 | ~2.032 SOL | 2600s | Rangos de edad censal |

**Justificación:**
- **Crecimiento exponencial:** Cada bin requiere comparación contra cada entrada
- **Complejidad real:** O(n×m) donde n=entradas, m=bins
- **Escalado sublineal:** m^1.5 en lugar de m^2 para balancear costo vs. utilidad
- **Benchmark real:** ~3 segundos por bin para 100 entradas

---

## Flujo de Validación

### Validación On-Chain (Programa Solana)

**Ubicación:** `/programs/zyberlink/src/processor/create_job.rs` (líneas 97-144)

```
                     Instrucción CreateJob
                              │
                              ▼
           ┌──────────────────────────────────┐
           │  Parsear circuit_type de datos IX│
           └──────────────────────────────────┘
                              │
                              ▼
           ┌──────────────────────────────────┐
           │  ¿Es FheComputation?             │
           └──────────────────────────────────┘
                      │              │
                   Sí│              │No (trabajo ZK)
                      ▼              ▼
           ┌─────────────────┐   Omitir validación
           │ Extraer FheOp   │
           └─────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ fhe_op.get_cost_config()         │
           │                                  │
           │ Devuelve:                        │
           │  - min_payment_lamports          │
           │  - timeout_seconds               │
           │  - complexity_tier               │
           └─────────────────────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ Calcular mínimo total:           │
           │                                  │
           │ total_min = min_payment          │
           │           × required_provers     │
           └─────────────────────────────────┘
                      │
                      ▼
           ┌─────────────────────────────────┐
           │ if price_lamports < total_min { │
           │   msg!("Precio muy bajo");       │
           │   return Err(InvalidPrice);      │
           │ }                                │
           └─────────────────────────────────┘
                      │
                   Éxito
                      ▼
           ┌─────────────────────────────────┐
           │ Usar timeout dinámico de config  │
           │ Crear trabajo con precio validado│
           └─────────────────────────────────┘
```

**Código Clave:**
```rust
// Líneas 109-128
let cost_config = fhe_op.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (fhe_consensus_config.required_provers as u64);

if price_lamports < total_min_price {
    msg!(
        "Precio muy bajo para operación FHE '{}' (nivel {}): {} < {} ({}×{} provers)",
        fhe_op.name(),
        cost_config.complexity_tier,
        price_lamports,
        total_min_price,
        min_price_per_prover,
        fhe_consensus_config.required_provers
    );
    return Err(ZyberLinkProgramError::InvalidPrice.into());
}
```

### Validación Off-Chain (API Backend)

**Ubicación:** `/blink-server/src/api_handlers.rs` (líneas 438-475)

```rust
// Validar precios contra modelo de costo dinámico
let cost_config = operation.get_cost_config();
let min_price_per_prover = cost_config.min_payment_lamports;
let total_min_price = min_price_per_prover * (validated.required_provers as u64);

if validated.price_lamports < total_min_price {
    return Err(anyhow::anyhow!(
        "Precio muy bajo para operación '{}' (nivel {}): {} < {} lamports",
        operation.name(),
        cost_config.complexity_tier,
        validated.price_lamports,
        total_min_price
    ));
}
```

**¿Por Qué Validar Dos Veces?**
1. **Validación backend** - Rechazo temprano, mejor UX (antes de firmar)
2. **Validación on-chain** - Garantía de seguridad (verdad canónica)

---

## Puntos de Integración

### 1. Integración SDK

**Ubicación:** `/sdk/src/instructions/marketplace.rs` (líneas 211-247)

```rust
pub fn create_fhe_job(
    &self,
    creator: Pubkey,
    job_id: u64,
    encrypted_input: &[u8],
    fhe_config: FheConsensusConfig,
    price_lamports: u64,
    timeout_seconds: i64,
) -> Result<Instruction> {
    // Nota: SDK NO valida precios
    // Validación ocurre on-chain por seguridad

    self.create_job(
        creator,
        job_id,
        CircuitType::FheComputation(fhe_config.operation.clone()),
        witness_commitment,
        encrypted_input.len() as u32,
        price_lamports,
        timeout_seconds,
        Some(fhe_config),
    )
}
```

**Mejor Práctica:**
```rust
// La aplicación debe calcular precio mínimo
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * 3; // 3 provers

// Usar mínimo o mayor
let ix = builder.create_fhe_job(
    creator,
    job_id,
    &encrypted_data,
    fhe_config,
    min_price,  // O mayor para prioridad
    cost_config.timeout_seconds
)?;
```

### 2. Integración Calculadora ROI

**Ubicación:** `/prover-node/src/roi_calculator.rs` (líneas 67-137)

```rust
pub fn evaluate_job(
    &self,
    circuit_type: &CircuitType,
    price_lamports: u64,
    required_provers: u8,
) -> JobROI {
    let fhe_operation = match circuit_type {
        CircuitType::FheComputation(op) => op,
        _ => return self.evaluate_simple_job(price_lamports, required_provers),
    };

    // Obtener config de costo de operación
    let cost_config = fhe_operation.get_cost_config();

    // Calcular ingreso por prover
    let revenue_per_prover = price_lamports / (required_provers as u64);

    // Calcular costo total incluyendo overhead operacional
    let base_cost = cost_config.min_payment_lamports;
    let total_cost = (base_cost as f64 * self.operational_cost_multiplier) as u64;

    // Calcular beneficio y ROI
    let profit = revenue_per_prover as i64 - total_cost as i64;
    let roi_percentage = if total_cost > 0 {
        (profit as f64 / total_cost as f64) * 100.0
    } else {
        0.0
    };

    let is_profitable = roi_percentage >= self.min_roi_percentage;

    JobROI {
        revenue_per_prover,
        estimated_cost: total_cost,
        profit,
        roi_percentage,
        complexity_tier: cost_config.complexity_tier,
        timeout_seconds: cost_config.timeout_seconds,
        is_profitable,
    }
}
```

**Configuración:**
```rust
// Por defecto: 20% ROI mín, 50% overhead
let calculator = ROICalculator::default();

// Personalizado: 15% ROI mín, 20% overhead
let calculator = ROICalculator::new(15.0, 1.2);
```

---

## Casos Extremos y Limitaciones

### 1. Tamaños Máximos de Entrada

**Problema:** Operaciones lineales (Sum, Average) escalan con `expected_count`, que es `u16` (máx 65,535).

**Impacto:**
```rust
// Costo máximo de Sum
let max_sum = FheOperation::Sum { expected_count: 65535 };
let config = max_sum.get_cost_config();
// Costo: 0.001 + (65535 × 0.0001) = 6.5545 SOL por prover
// Timeout: 60 + (65535 × 2) = 131,130 segundos (~36 horas)
```

**Mitigación:**
- Frontend debe advertir en conteos > 10,000
- Considerar dividir operaciones grandes en lotes
- Futuro: Implementar primitivas de procesamiento por lotes

### 2. Límites de Bins de Histogram

**Problema:** El costo de Histogram crece como O(m^1.5), haciendo los conteos grandes de bins costosos.

**Impacto:**
```rust
// Histogram de 100 bins
let large_hist = FheOperation::Histogram {
    bins: vec![HistogramBin::new(i, i+1, format!("bin{}", i)); 100],
};
let config = large_hist.get_cost_config();
// Costo: 0.1 + (0.02 × 100^1.5) = 0.1 + 20 = 20.1 SOL por prover
// Timeout: 600 + (100 × 100) = 10,600 segundos (~3 horas)
```

**Mitigación:**
- Frontend limita bins a 50 máximo
- Recomendar 5-10 bins para la mayoría de casos de uso
- Ver [optimizacion-histograma-fhe.md](./optimizacion-histograma-fhe.md) para alternativas

### 3. Front-running de Precios

**Problema:** Los precios mínimos son determinísticos y públicos.

**Ataque:** Actor malicioso podría crear trabajos al precio mínimo exacto para desplazar a usuarios legítimos.

**Mitigación:**
- Los usuarios pueden pagar por encima del mínimo para prioridad
- Futuro: Implementar mecanismo de tarifa de prioridad
- La selección de prover podría factorizar reputación histórica

### 4. Precisión de Timeout

**Problema:** Los timeouts dinámicos son estimaciones basadas en benchmarks, el tiempo real de ejecución puede variar.

**Impacto:** Los trabajos podrían expirar en hardware más lento.

**Mitigación:**
- Los timeouts son conservadores (incluyen buffer)
- Los provers deben hacer benchmarks y rechazar trabajos no rentables
- Futuro: Implementar niveles de clase de hardware

### 5. Operaciones de Costo Cero

**Problema:** Operaciones `expected_count: 0` solo tienen costo base.

```rust
let zero_sum = FheOperation::Sum { expected_count: 0 };
// Costo: 0.001 SOL (solo costo base)
```

**Impacto:** Puede usarse para spam, aunque aún requiere pago mínimo.

**Mitigación:**
- Frontend valida count > 0
- El programa debe validar expected_count > 0 (TODO)

---

## Consideraciones de Rendimiento

### 1. Rendimiento de Cálculo de Costo

**Benchmark:** Tiempo de ejecución de `get_cost_config()`

```rust
// Nivel 1-4: Cálculo O(1)
// Tiempo: < 100ns (despreciable)

// Nivel 5: O(1) con llamada sqrt()
// Tiempo: ~500ns (aún despreciable)
```

**Conclusión:** El cálculo de costo no es un cuello de botella de rendimiento.

### 2. Unidades de Cómputo On-Chain

**Presupuesto de Cómputo Solana:**

```rust
// Unidades de cómputo de instrucción CreateJob
Overhead base:       ~5,000 CU
Precios dinámicos:   ~500 CU
Creación escrow:     ~10,000 CU
Creación cuenta:     ~20,000 CU
──────────────────────────────
Total:               ~35,500 CU
```

**Límite:** 200,000 CU por transacción (bien dentro de límites)

### 3. Overhead de Serialización

**Serialización Borsh:**

| Tipo | Tamaño Serializado |
|------|-----------------|
| `OperationCostConfig` | 21 bytes |
| `FheOperation::Add` | 2 bytes |
| `FheOperation::Sum` | 3 bytes |
| `FheOperation::Histogram(10 bins)` | ~240 bytes |

**Impacto de Red:** Mínimo, Histogram es la variante más grande pero aún < 1KB.

### 4. Latencia API Backend

**Endpoint:** `/api/estimate-cost`

```
Parseo de solicitud:     ~50μs
Cálculo de costo:        ~1μs
Serialización JSON:      ~100μs
──────────────────────────────
Total:                   ~150μs
```

**Conclusión:** Latencia sub-milisegundo, despreciable para UX.

---

## Estrategia de Testing

### Tests Unitarios

**Ubicación:** `/shared/types/src/fhe.rs` (líneas 598-811)

**Cobertura:**
- Nivel 1: 2 tests (Add, Multiply)
- Nivel 2: 1 test (escalado Sum)
- Nivel 3: 2 tests (Threshold, RangeCheck)
- Nivel 4: 2 tests (Average, CountIf)
- Nivel 5: 2 tests (Histogram pequeño/grande)
- Cross-tier: 1 test (verificación de ordenamiento)
- Escenarios realistas: 1 test (censo, votación, verificación edad)

### Tests de Integración

**Ubicación:** `/programs/zyberlink/tests/dynamic_pricing_tests.rs`

**Casos de Test:**
1. `test_tier1_add_sufficient_price` - Aceptar precio válido Nivel 1
2. `test_tier1_add_insufficient_price` - Rechazar Nivel 1 con precio bajo
3. `test_tier3_threshold_higher_price` - Verificar Nivel 3 requiere más
4. `test_pricing_scales_with_provers` - Validación de escalado lineal

### Tests E2E

**Ubicación:** `/test-dynamic-pricing-e2e.sh`

**Flujo de Test:**
1. Estimación API Nivel 1 (Add)
2. Estimación API Nivel 3 (Threshold)
3. Estimación API Nivel 5 (Histogram)
4. Verificación de escalado de conteo de provers
5. Tests unitarios de tipos compartidos
6. Tests unitarios de calculadora ROI

---

## Mejoras Futuras

### 1. Ajuste Dinámico de Niveles

```rust
// Ajustar niveles basado en congestión de red
pub fn get_dynamic_cost_config(
    &self,
    network_load: f64  // 0.0 - 1.0
) -> OperationCostConfig {
    let base_config = self.get_cost_config();
    let multiplier = 1.0 + (network_load * 2.0);  // Hasta 3x durante pico

    OperationCostConfig {
        min_payment_lamports: (base_config.min_payment_lamports as f64 * multiplier) as u64,
        ..base_config
    }
}
```

### 2. Clases de Hardware de Provers

```rust
pub enum HardwareClass {
    Basic,      // 1x precio base
    Standard,   // 1.5x precio base, 0.7x timeout
    Premium,    // 2x precio base, 0.5x timeout
}

// Provers pujan con clase de hardware
// Los trabajos pueden especificar clase requerida
```

### 3. Análisis Histórico

```sql
CREATE TABLE pricing_history (
    operation TEXT,
    tier INT,
    avg_price_lamports BIGINT,
    avg_completion_time_secs INT,
    date DATE
);

-- Rastrear costos reales vs. estimaciones
-- Ajustar niveles trimestralmente basado en datos
```

---

## Conclusión

El Sistema de Precios Dinámicos proporciona:

1. **Compensación justa** para provers basada en costo computacional
2. **Precios transparentes** visibles para todos los participantes
3. **Aplicación a nivel de protocolo** previniendo precios bajos
4. **Escalabilidad** desde aritmética simple a distribuciones complejas
5. **Extensibilidad** para futuros tipos de operación y modelos de precios

**Próximos Pasos:**
- Leer [Guía de Uso](./precios-dinamicos-uso.md) para ejemplos de integración
- Revisar [Referencia API](./referencia-api-completa.md) para detalles de endpoints
- Ver [Optimización Histogram](./optimizacion-histograma-fhe.md) para inmersión profunda Nivel 5

---

**Última Actualización:** 2025-11-21
**Versión:** 1.0.0
**Mantenido Por:** Equipo Core ZyberLink
