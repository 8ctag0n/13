# FHE Histogram: Diseño y Optimizaciones Futuras

## Versión: 2.0 (Optimizada - No Implementada)

### Resumen Ejecutivo

Este documento describe el diseño optimizado de operaciones Histogram sobre datos cifrados con FHE (Fully Homomorphic Encryption), para implementación futura en ZyberLink. Documenta estrategias de optimización, benchmarks objetivos, y trade-offs arquitectónicos.

---

## 1. Fundamentos Matemáticos

### 1.1 Operación Base

```
FHE_Histogram: [Enc(v₁), ..., Enc(vₙ)], bins → [Enc(count₁), ..., Enc(countₘ)]

Donde:
- vᵢ ∈ {0..255} (valores encriptados FheUint8)
- bins = [(min₁,max₁), ..., (minₘ,maxₘ)] (definición de rangos)
- countⱼ = |{i : minⱼ ≤ vᵢ ≤ maxⱼ}| (conteo por bin)
```

### 1.2 Implementación Naive

```rust
fn compute_histogram_naive(
    encrypted_values: Vec<FheUint8>,
    bins: &[HistogramBin]
) -> Result<Vec<FheUint8>> {
    let mut counts = Vec::new();

    for bin in bins {
        let mut count = FheUint8::try_encrypt_trivial(0)?;

        for value in &encrypted_values {
            // Para cada valor, verificar si está en el bin
            let in_range = check_range(value, bin.min, bin.max)?;
            count = count + in_range; // in_range es FheBool → FheUint8
        }

        counts.push(count);
    }

    Ok(counts)
}

fn check_range(value: &FheUint8, min: u8, max: u8) -> Result<FheUint8> {
    let min_ct = FheUint8::try_encrypt_trivial(min)?;
    let max_ct = FheUint8::try_encrypt_trivial(max)?;

    let ge_min = value.ge(&min_ct); // value >= min
    let le_max = value.le(&max_ct); // value <= max

    let in_range = ge_min & le_max;

    // Convertir FheBool a FheUint8 (0 o 1)
    Ok(in_range.if_then_else(&FheUint8::try_encrypt_trivial(1)?,
                              &FheUint8::try_encrypt_trivial(0)?))
}
```

**Complejidad:** O(n × m × c)
- n = número de valores
- m = número de bins
- c = costo de comparación FHE (~2-3 operaciones bootstrap)

**Performance Estimada (v1.0):**
- 100 valores, 5 bins: ~20-30 segundos
- 1000 valores, 10 bins: ~300-500 segundos

---

## 2. Estrategias de Optimización

### 2.1 Paralelización por Bins

**Concepto:** Cada bin se procesa independientemente en hilos separados.

```rust
fn compute_histogram_parallel(
    encrypted_values: Vec<FheUint8>,
    bins: &[HistogramBin]
) -> Result<Vec<FheUint8>> {
    use rayon::prelude::*;

    bins.par_iter()
        .map(|bin| count_values_in_bin(&encrypted_values, bin))
        .collect()
}

fn count_values_in_bin(
    values: &[FheUint8],
    bin: &HistogramBin
) -> Result<FheUint8> {
    values.iter()
        .try_fold(FheUint8::try_encrypt_trivial(0)?, |acc, val| {
            let in_range = check_range(val, bin.min, bin.max)?;
            Ok(acc + in_range)
        })
}
```

**Ganancia Esperada:** Speedup lineal en número de bins (5x para 5 bins)
**Trade-off:** Mayor uso de RAM (m × tamaño_ciphertext)

### 2.2 Batch Range Checks

**Concepto:** Reducir número de comparaciones mediante pre-sorting conceptual.

```rust
// Optimización: si bins están ordenados y sin overlap
// podemos usar búsqueda binaria conceptual

fn optimized_range_check(
    value: &FheUint8,
    sorted_bins: &[HistogramBin]
) -> Result<FheUint8> {
    // Para bins ordenados: [0-10], [11-20], [21-30], ...
    // Un valor solo puede estar en un bin
    // Podemos crear árbol de decisión más eficiente

    // Ejemplo con 4 bins:
    // Comparar con punto medio (bin[1].max)
    // Si value <= mid → buscar en bins[0..2]
    // Si value > mid → buscar en bins[2..4]

    // Esto reduce de 4 comparaciones a log₂(4) = 2 en promedio

    todo!("Implementar binary search tree en FHE")
}
```

**Ganancia Esperada:** O(n × log m × c) vs O(n × m × c)
**Restricción:** Requiere bins disjuntos y ordenados

### 2.3 Compresión de Comparaciones

**Concepto:** Reutilizar comparaciones intermedias.

```rust
// Si tenemos bins: [0-10], [11-20], [21-30]
// Podemos calcular:
// bin1 = (value >= 0) & (value <= 10)
// bin2 = (value >= 11) & (value <= 20) = NOT(bin1) & (value <= 20)
// bin3 = (value >= 21) & (value <= 30) = NOT(bin1 | bin2) & (value <= 30)

fn compressed_histogram(
    values: Vec<FheUint8>,
    bins: &[HistogramBin]
) -> Result<Vec<FheUint8>> {
    // Cachear comparaciones comunes
    let mut comparison_cache = HashMap::new();

    for value in &values {
        // Reutilizar comparaciones ya calculadas
        let ge_11 = comparison_cache.entry("ge_11")
            .or_insert_with(|| value.ge(&FheUint8::try_encrypt_trivial(11)?));

        // ... aplicar a bins
    }

    todo!("Implementar cache de comparaciones")
}
```

**Ganancia Esperada:** 30-40% menos operaciones FHE
**Complejidad:** Difícil para bins arbitrarios

### 2.4 Approximation Schemes

**Concepto:** Trade off precisión por velocidad.

```rust
// Opción 1: Discrete Bins (ya implementado)
// Bins explícitos: [0-10], [11-20], ...

// Opción 2: Power-of-Two Bins (más eficiente)
// Usar máscara de bits: bin_index = value >> 3 (divide por 8)
fn power_of_two_histogram(
    values: Vec<FheUint8>,
    bin_shift: u8 // Ejemplo: 3 para bins de tamaño 8
) -> Result<Vec<FheUint8>> {
    // Bins automáticos: [0-7], [8-15], [16-23], ...
    let num_bins = 256 >> bin_shift;
    let mut counts = vec![FheUint8::try_encrypt_trivial(0)?; num_bins];

    for value in values {
        // Shift es más barato que comparaciones
        let bin_index = value >> bin_shift; // FHE shift operation

        // Incrementar el bin correcto
        // Problema: indexación dinámica en FHE es costosa
        // Solución: usar one-hot encoding + select

        todo!("Implementar indexación eficiente")
    }

    Ok(counts)
}
```

**Ganancia Esperada:** 5-10x speedup
**Trade-off:** Bins fijos, no arbitrarios

---

## 3. Casos de Uso y Requisitos

### 3.1 Votación DAO (Caso Primario)

```rust
// Escenario: 1000 miembros votan por 5 opciones
// Cada voto es FheUint8 con valor 0-4

let votes: Vec<FheUint8> = collect_encrypted_votes()?;

let bins = vec![
    HistogramBin::new(0, 0, "Opción A"),
    HistogramBin::new(1, 1, "Opción B"),
    HistogramBin::new(2, 2, "Opción C"),
    HistogramBin::new(3, 3, "Opción D"),
    HistogramBin::new(4, 4, "Opción E"),
];

let tallies = compute_histogram_parallel(votes, &bins)?;

// Resultado: [Enc(200), Enc(350), Enc(180), Enc(150), Enc(120)]
// Privacidad: votos individuales nunca revelados
```

**Requisitos:**
- Performance: 1000 votos, 5 opciones → < 60 segundos
- Bins disjuntos (cada voto cuenta una vez)
- Verificable: sum(tallies) = total_votes

### 3.2 Demografía Distribuida (Caso Secundario)

```rust
// Escenario: Distribución de edades sin revelar individuos
// 500 miembros, edades 0-100

let ages: Vec<FheUint8> = collect_encrypted_ages()?;

let bins = vec![
    HistogramBin::new(0, 17, "Menores"),
    HistogramBin::new(18, 35, "Jóvenes adultos"),
    HistogramBin::new(36, 55, "Adultos"),
    HistogramBin::new(56, 100, "Adultos mayores"),
];

let distribution = compute_histogram(ages, &bins)?;
```

**Requisitos:**
- Performance: 500 valores, 4 bins → < 40 segundos
- Bins contiguos (sin gaps)
- Extensible a más bins

### 3.3 Geo-distribución (Caso Avanzado)

```rust
// Escenario: Mapear usuarios a regiones
// Región codificada como u8: 0=Asia, 1=Europa, 2=Americas, etc.

let regions: Vec<FheUint8> = collect_encrypted_regions()?;

let bins = vec![
    HistogramBin::new(0, 0, "Asia"),
    HistogramBin::new(1, 1, "Europa"),
    HistogramBin::new(2, 2, "Americas"),
    HistogramBin::new(3, 3, "África"),
    HistogramBin::new(4, 4, "Oceanía"),
];

let distribution = compute_histogram(regions, &bins)?;

// Output: "40% Asia, 30% Europa, 20% Americas, 8% África, 2% Oceanía"
```

---

## 4. Benchmarks Objetivos

### 4.1 Targets v2.0 (Optimizado)

| Escenario | Valores | Bins | Target (v2.0) | Baseline (v1.0) | Speedup |
|-----------|---------|------|---------------|-----------------|---------|
| Votación pequeña | 100 | 5 | 3s | 20s | 6.6x |
| Votación grande | 1000 | 5 | 25s | 300s | 12x |
| Demografía | 500 | 4 | 15s | 160s | 10.6x |
| Geo-distribución | 1000 | 10 | 40s | 600s | 15x |

### 4.2 Métricas de Calidad

- **Throughput:** > 30 valores/segundo (v2.0 target)
- **Memory:** < 5MB por 1000 valores (incluye ciphertexts)
- **Latency p99:** < 1.5x p50 (consistencia)
- **Accuracy:** 100% (sin aproximaciones lossy)

### 4.3 Profiling Esperado

```
Histogram(1000 valores, 5 bins) - Breakdown:

Deserialización inputs:     2.0s  (8%)
Range checks (n×m):        18.0s (72%)  ← Principal bottleneck
Acumulación counts:         3.0s (12%)
Serialización outputs:      2.0s  (8%)
                          ------
Total:                    25.0s
```

**Optimización prioritaria:** Reducir costo de range checks

---

## 5. Plan de Implementación Futura

### 5.1 Fase 1: Baseline (Día 5)

**Implementar versión naive funcional:**

```rust
// prover-node/src/circuits/voting.rs

pub fn compute_histogram(
    encrypted_inputs: Vec<&[u8]>,
    bins: &[HistogramBin],
) -> Result<Vec<Vec<u8>>> {
    let values: Vec<FheUint8> = encrypted_inputs
        .iter()
        .map(|bytes| bincode::deserialize(bytes))
        .collect::<Result<Vec<_>, _>>()?;

    let mut counts = Vec::new();

    for bin in bins {
        let count = count_in_range(&values, bin.min, bin.max)?;
        counts.push(bincode::serialize(&count)?);
    }

    Ok(counts)
}
```

**Objetivo:** Funcionalidad correcta, benchmarks baseline

### 5.2 Fase 2: Paralelización (Post-MVP)

**Implementar Optimización 2.1:**

```rust
use rayon::prelude::*;

pub fn compute_histogram_v2(
    encrypted_inputs: Vec<&[u8]>,
    bins: &[HistogramBin],
) -> Result<Vec<Vec<u8>>> {
    let values: Arc<Vec<FheUint8>> = Arc::new(deserialize_all(encrypted_inputs)?);

    bins.par_iter()
        .map(|bin| {
            let count = count_in_range(&values, bin.min, bin.max)?;
            bincode::serialize(&count)
        })
        .collect()
}
```

**Ganancia esperada:** 4-6x speedup en sistemas multi-core

### 5.3 Fase 3: Smart Range Checks (Investigación)

**Requerirá investigación en:**
- Tfhe-rs: ¿Soporta optimizaciones custom para comparaciones batch?
- CUDA backend: ¿GPU acceleration disponible?
- Lookup tables: ¿Implementación eficiente de LUTs en Tfhe?

**Posible implementación:**

```rust
// Usar TFHE programmable bootstrapping con LUT custom
fn optimized_range_check_lut(
    value: &FheUint8,
    bins: &[HistogramBin]
) -> Result<Vec<FheUint8>> {
    // Crear LUT: input=valor → output=bin_index
    // Un solo bootstrap devuelve el bin

    let lut = create_histogram_lut(bins)?;
    let bin_index = value.apply_lookup_table(&lut)?;

    // Convertir bin_index a one-hot encoding
    // [0,0,1,0,0] para bin_index=2

    bins.iter()
        .enumerate()
        .map(|(i, _)| bin_index.eq(&FheUint8::try_encrypt_trivial(i as u8)?))
        .collect()
}
```

**Ganancia teórica:** 10-20x speedup
**Riesgo:** Alta complejidad, dependencia de features experimentales

---

## 6. Consideraciones de Arquitectura

### 6.1 API Surface

```rust
// Flexible API para diferentes optimizaciones

pub enum HistogramStrategy {
    Naive,           // O(n×m), simple
    Parallel,        // O(n×m/cores), usa rayon
    BatchOptimized,  // O(n×log m), requiere bins ordenados
    Approximate,     // O(n), bins power-of-two
}

pub fn compute_histogram_with_strategy(
    inputs: Vec<&[u8]>,
    bins: &[HistogramBin],
    strategy: HistogramStrategy,
) -> Result<Vec<Vec<u8>>> {
    match strategy {
        HistogramStrategy::Naive => compute_histogram_naive(inputs, bins),
        HistogramStrategy::Parallel => compute_histogram_parallel(inputs, bins),
        // ...
    }
}
```

### 6.2 Validación de Inputs

```rust
fn validate_bins(bins: &[HistogramBin]) -> Result<()> {
    // Check 1: Bins no vacíos
    ensure!(!bins.is_empty(), "Bins cannot be empty");
    ensure!(bins.len() <= 32, "Maximum 32 bins supported");

    // Check 2: Rangos válidos
    for bin in bins {
        ensure!(bin.min <= bin.max, "Invalid bin range: {} > {}", bin.min, bin.max);
    }

    // Check 3: Sin overlap (opcional, dependiendo de semántica)
    for i in 0..bins.len() {
        for j in (i+1)..bins.len() {
            let overlap = bins[i].max >= bins[j].min && bins[j].max >= bins[i].min;
            if overlap {
                warn!("Bins overlap detected: {} and {}", i, j);
                // Podemos permitir overlap, pero un valor se cuenta en múltiples bins
            }
        }
    }

    Ok(())
}
```

### 6.3 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum HistogramError {
    #[error("Invalid bins configuration: {0}")]
    InvalidBins(String),

    #[error("Deserialization failed: {0}")]
    DeserializationError(#[from] bincode::Error),

    #[error("FHE operation failed: {0}")]
    FheError(String),

    #[error("Too many values: {count} exceeds maximum {max}")]
    TooManyValues { count: usize, max: usize },
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_single_bin() {
        // 100 valores en rango [0-10], todos deben contarse
        let values = vec![5u8; 100];
        let encrypted = encrypt_values(&values);

        let bins = vec![HistogramBin::new(0, 10, "All")];
        let result = compute_histogram(encrypted, &bins).unwrap();

        let count = decrypt(&result[0]);
        assert_eq!(count, 100);
    }

    #[test]
    fn test_histogram_disjoint_bins() {
        // Valores: [5,5,5,15,15,25,25,25,25]
        // Bins: [0-10], [11-20], [21-30]
        // Expected: [3, 2, 4]

        let values = vec![5,5,5,15,15,25,25,25,25];
        let encrypted = encrypt_values(&values);

        let bins = vec![
            HistogramBin::new(0, 10, "Low"),
            HistogramBin::new(11, 20, "Mid"),
            HistogramBin::new(21, 30, "High"),
        ];

        let result = compute_histogram(encrypted, &bins).unwrap();
        let counts: Vec<u8> = result.iter().map(|r| decrypt(r)).collect();

        assert_eq!(counts, vec![3, 2, 4]);
    }

    #[test]
    fn test_histogram_overlapping_bins() {
        // Valores: [5, 10, 15]
        // Bins: [0-10], [10-20] (overlap en 10)
        // Expected: [2, 2] (10 se cuenta en ambos)

        // Test documenta comportamiento con overlap
    }
}
```

### 7.2 Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn histogram_sum_equals_count(
        values in prop::collection::vec(any::<u8>(), 1..100),
        num_bins in 1usize..10
    ) {
        // Propiedad: suma de todos los bins = número de valores
        // (Solo si bins cubren todo el rango sin overlap)

        let encrypted = encrypt_values(&values);
        let bins = generate_disjoint_bins(num_bins);

        let result = compute_histogram(encrypted, &bins)?;
        let total: u8 = result.iter().map(|r| decrypt(r)).sum();

        prop_assert_eq!(total, values.len() as u8);
    }
}
```

### 7.3 Performance Regression Tests

```rust
#[test]
#[ignore] // Solo correr en CI con tiempo suficiente
fn bench_histogram_1000_values_5_bins() {
    let values = vec![42u8; 1000];
    let encrypted = encrypt_values(&values);

    let bins = create_voting_bins(5);

    let start = Instant::now();
    let _result = compute_histogram(encrypted, &bins).unwrap();
    let duration = start.elapsed();

    // v2.0 target: < 25s
    assert!(duration.as_secs() < 25,
            "Regression: took {:?}, expected < 25s", duration);
}
```

---

## 8. Dependencias y Limitaciones

### 8.1 Tfhe-rs Capabilities

**Actualmente soportado:**
- FheUint8 comparisons: `ge()`, `le()`, `eq()`
- Boolean operations: `&`, `|`, `^`
- Conditional: `if_then_else()`
- Arithmetic: `+`, `-`, `*`

**No soportado (aún):**
- Indexación dinámica eficiente de arrays
- Custom LUTs para multiples outputs
- GPU acceleration nativo

**Implicaciones:**
- Estrategia 2.4 (Power-of-Two Bins) difícil de implementar eficientemente
- Estrategia 2.3 (Compression) requiere lógica manual compleja
- Estrategia 2.1 (Parallelization) es la más viable a corto plazo

### 8.2 Hardware Requirements

**Para benchmarks target v2.0:**
- CPU: 8+ cores (para paralelización)
- RAM: 16GB+ (1000 FheUint8 ≈ 4-8MB)
- Disco: SSD recomendado (I/O de ciphertexts)

**Escalamiento:**
- 10,000 valores: ~80GB RAM estimado
- Considerar almacenamiento en disco + streaming

---

## 9. Roadmap Técnico

### Q1 2026: MVP (v1.0)
- [x] Implementación naive funcional
- [ ] Tests unitarios completos
- [ ] Benchmark baseline establecido
- [ ] Documentación de API

### Q2 2026: Optimización (v2.0)
- [ ] Paralelización con Rayon
- [ ] Benchmarks mejorados (6-12x speedup)
- [ ] Property-based tests
- [ ] Performance regression suite

### Q3 2026: Investigación Avanzada (v3.0)
- [ ] Explorar GPU acceleration
- [ ] Implementar custom LUTs si disponibles
- [ ] Algoritmos aproximados para casos de uso específicos
- [ ] Paper académico sobre optimizaciones

### Q4 2026: Producción (v3.x)
- [ ] Auto-selección de estrategia basada en inputs
- [ ] Monitoreo y alertas de performance
- [ ] Optimizaciones específicas por caso de uso
- [ ] A/B testing de diferentes estrategias

---

## 10. Referencias

### Papers Académicos
- **TFHE:** "Faster Fully Homomorphic Encryption: Bootstrapping in less than 0.1 seconds" (Ducas & Micciancio, 2016)
- **Histogram Computation:** "Privacy-Preserving Histogram Computation over Encrypted Data" (Chen et al., 2019)
- **Optimization Techniques:** "Efficient FHE Evaluation of Circuit Families" (Halevi & Shoup, 2020)

### Documentación Técnica
- Tfhe-rs API: https://docs.rs/tfhe/latest/tfhe/
- ZyberLink FHE Engine: `/prover-node/src/fhe_engine.rs`
- Track 2 Plan: `/TRACK2_PLAN.md`

### Benchmarks Comparativos
- Microsoft SEAL: Histogram sobre 1000 valores ≈ 15-20s (BFV scheme)
- Zama Concrete: Similar performance esperado (TFHE scheme)
- OpenFHE: 25-30s (BGV scheme)

**Nota:** Benchmarks externos son aproximados, diferentes parámetros de seguridad y hardware.

---

## Conclusiones

El histogram FHE es computacionalmente costoso (O(n×m)) pero optimizable. La estrategia más viable a corto plazo es **paralelización por bins**, con potencial de 6-12x speedup. Optimizaciones avanzadas (LUTs, GPU) requieren investigación adicional pero podrían lograr 10-20x mejoras.

**Prioridades para implementación inicial:**
1. Versión naive funcional (correctitud > velocidad)
2. Tests exhaustivos (confianza en resultados)
3. Benchmarks baseline (medir para mejorar)
4. Paralelización (quick win con Rayon)

Este documento se actualizará conforme se implementen optimizaciones y se descubran nuevas técnicas.

---

**Última actualización:** 2025-11-20
**Autor:** Equipo ZyberLink
**Estado:** Diseño para implementación futura
