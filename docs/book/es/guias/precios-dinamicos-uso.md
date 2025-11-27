# Sistema de Precios Dinámicos - Guía de Uso

**Guía de Integración Práctica con Ejemplos del Mundo Real**

---

## Tabla de Contenidos

1. [Para Creadores de Jobs](#para-creadores-de-jobs)
2. [Para Provers](#para-provers)
3. [Ejemplos de Integración con SDK](#ejemplos-de-integración-con-sdk)
4. [Ejemplos de Integración con API](#ejemplos-de-integración-con-api)
5. [Integración Frontend](#integración-frontend)
6. [Mejores Prácticas](#mejores-prácticas)
7. [Solución de Problemas](#solución-de-problemas)
8. [Preguntas Frecuentes](#preguntas-frecuentes)

---

## Para Creadores de Jobs

### Inicio Rápido: Estimar Costos de Jobs

Antes de crear un job, siempre estima el costo:

```bash
# Ejemplo: Verificación de edad (Tier 3)
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

**Respuesta:**
```json
{
  "operation": "Threshold",
  "complexity_tier": 3,
  "min_payment_lamports": 5000000,
  "min_payment_sol": 0.005,
  "total_min_payment_lamports": 15000000,
  "total_min_payment_sol": 0.015,
  "timeout_seconds": 300,
  "estimated_compute_ms": 300
}
```

### Ejemplos de Casos de Uso

#### 1. Verificación de Edad para ZK-Passport

**Escenario:** Verificar que un usuario es mayor de 18 años sin revelar su edad real.

**Operación:** `Threshold`

```rust
use zyberlink_types::fhe::{FheOperation, FheConsensusConfig};
use zyberlink_sdk::instructions::InstructionBuilder;

// Configuración
let operation = FheOperation::Threshold {
    threshold: 18,
    greater_or_equal: true,
};

// Obtener precios
let cost_config = operation.get_cost_config();
println!("Operation: {}", operation.name());
println!("Tier: {}", cost_config.complexity_tier);
println!("Cost per prover: {} SOL", cost_config.min_payment_lamports as f64 / 1_000_000_000.0);
println!("Timeout: {}s", cost_config.timeout_seconds);

// Crear configuración FHE
let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

// Calcular precio total
let total_price = cost_config.min_payment_lamports * 3; // 3 provers = 15,000,000 lamports (0.015 SOL)

// Construir instrucción
let builder = InstructionBuilder::new(program_id);
let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_age_data,
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;

// Firmar y enviar transacción
```

**Costo Esperado:** 0.015 SOL (0.005 SOL × 3 provers)
**Tiempo Esperado:** ~5 minutos

#### 2. Conteo Censal Privado (Tier 2)

**Escenario:** Contar el número de personas en una región sin revelar datos individuales.

**Operación:** `Sum`

```rust
// Para 1,000 personas
let operation = FheOperation::Sum {
    expected_count: 1000,
};

let cost_config = operation.get_cost_config();

// Cálculo de costo:
// Base: 0.001 SOL
// Por ítem: 1000 × 0.0001 SOL = 0.1 SOL
// Total por prover: 0.101 SOL
// 3 provers: 0.303 SOL

let total_price = cost_config.min_payment_lamports * 3; // 303,000,000 lamports

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds, // 2,060 segundos
    operation: operation.clone(),
};

let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_census_data, // Array de 1,000 valores encriptados
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;
```

**Costo Esperado:** 0.303 SOL
**Tiempo Esperado:** ~34 minutos

#### 3. Votación Privada (Tier 5)

**Escenario:** Realizar una votación entre 5 candidatos con boletas encriptadas.

**Operación:** `Histogram`

```rust
use zyberlink_types::fhe::HistogramBin;

// Definir 5 candidatos
let operation = FheOperation::Histogram {
    bins: vec![
        HistogramBin::new(0, 0, "Candidate A"),
        HistogramBin::new(1, 1, "Candidate B"),
        HistogramBin::new(2, 2, "Candidate C"),
        HistogramBin::new(3, 3, "Candidate D"),
        HistogramBin::new(4, 4, "Candidate E"),
    ],
};

let cost_config = operation.get_cost_config();

// Cálculo de costo:
// Base: 0.1 SOL
// Exponencial: 0.02 × 5^1.5 ≈ 0.224 SOL
// Total por prover: ~0.324 SOL
// 3 provers: ~0.972 SOL

let total_price = cost_config.min_payment_lamports * 3; // ~972,000,000 lamports

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds, // 1,100 segundos
    operation: operation.clone(),
};

let ix = builder.create_fhe_job(
    creator_pubkey,
    job_id,
    &encrypted_votes, // Opciones de voto encriptadas (0-4)
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;
```

**Costo Esperado:** ~0.972 SOL
**Tiempo Esperado:** ~18 minutos

**Formato de Resultado:**
```
[
  encrypt(count_A),  // Número de votos para Candidato A
  encrypt(count_B),  // Número de votos para Candidato B
  encrypt(count_C),
  encrypt(count_D),
  encrypt(count_E)
]
```

#### 4. Ejemplo de Pago con Token (wZEC)

**Escenario:** Pagar por un job FHE usando tokens wZEC en lugar de SOL.

```rust
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;

let operation = FheOperation::Add(5);
let cost_config = operation.get_cost_config();

// Convertir precio de SOL a wZEC (1 wZEC = ~$25, 1 SOL = ~$100)
// wZEC tiene 8 decimales (como Bitcoin)
let sol_price = cost_config.min_payment_lamports;
let wzec_price = (sol_price * 4) / 1; // Ratio aproximado 4:1, ajustar según mercado real

let wzec_mint = Pubkey::from_str("ZECpv6hqVqwz4c3c9RMz8N5SLKFx9Qz...")
    .expect("Valid wZEC mint");

let creator_token_account = get_associated_token_address(
    &creator_pubkey,
    &wzec_mint,
);

let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

let ix = builder.create_fhe_job_with_token(
    creator_pubkey,
    job_id,
    &encrypted_data,
    fhe_config,
    wzec_price * 3, // Precio en zatoshis (unidades base de wZEC)
    cost_config.timeout_seconds,
    wzec_mint,
    creator_token_account,
)?;
```

---

## Para Provers

### Inicio Rápido: Evaluar Rentabilidad de Jobs

Antes de reclamar un job, verifica si es rentable:

```rust
use zyberlink_prover::roi_calculator::ROICalculator;

// Configurar tus requerimientos de ROI
let calculator = ROICalculator::new(
    15.0,   // 15% ROI mínimo
    1.2     // 20% de gastos operacionales (electricidad, desgaste de hardware)
);

// Evaluar un job
let job_roi = calculator.evaluate_job(
    &job.circuit_type,
    job.price_lamports,
    job.fhe_config.required_provers,
);

if job_roi.is_profitable {
    println!("✅ JOB RENTABLE");
    println!("   Ingreso por prover: {} SOL", job_roi.revenue_per_prover as f64 / 1e9);
    println!("   Costo estimado: {} SOL", job_roi.estimated_cost as f64 / 1e9);
    println!("   Ganancia: {} SOL", job_roi.profit as f64 / 1e9);
    println!("   ROI: {:.1}%", job_roi.roi_percentage);
    println!("   Tier: {}", job_roi.complexity_tier);

    // Reclamar el job
    claim_job(&job)?;
} else {
    println!("❌ NO RENTABLE");
    println!("   ROI esperado: {:.1}% (min: 15%)", job_roi.roi_percentage);
    println!("   Omitir este job");
}
```

### Ejemplos de Configuración de Prover

#### 1. Prover Conservador (Alto Margen)

```rust
// Solo aceptar jobs muy rentables
let conservative_calculator = ROICalculator::new(
    50.0,   // 50% ROI mínimo
    2.0     // 100% gastos generales (margen de seguridad 2x)
);

// Este prover solo aceptará jobs que paguen al menos 3x el costo base
```

**Caso de Uso:** Operaciones pequeñas, baja tolerancia al riesgo, costos de electricidad altos

#### 2. Prover Agresivo (Bajo Margen)

```rust
// Aceptar la mayoría de jobs por encima del costo
let aggressive_calculator = ROICalculator::new(
    10.0,   // 10% ROI mínimo
    1.1     // 10% gastos generales (márgenes ajustados)
);

// Este prover acepta más jobs pero requiere operaciones eficientes
```

**Caso de Uso:** Hardware de alto rendimiento, costos de electricidad bajos, estrategia de volumen

#### 3. Estrategia Específica por Tier

```rust
// Aceptar diferentes márgenes para diferentes tiers
fn should_accept_job(job: &Job) -> bool {
    let base_calculator = ROICalculator::new(20.0, 1.2);
    let roi = base_calculator.evaluate_job(
        &job.circuit_type,
        job.price_lamports,
        job.fhe_config.required_provers,
    );

    match roi.complexity_tier {
        1 | 2 => {
            // Aceptar Tier 1-2 si es rentable en absoluto
            roi.roi_percentage > 5.0
        }
        3 | 4 => {
            // Margen estándar para Tier 3-4
            roi.roi_percentage > 15.0
        }
        5 => {
            // Mayor margen para Tier 5 costoso
            roi.roi_percentage > 30.0
        }
        _ => false,
    }
}
```

### Cálculo de Precio Mínimo

Obtén el precio mínimo absoluto que aceptarías:

```rust
let calculator = ROICalculator::new(20.0, 1.5);

let min_acceptable = calculator.get_minimum_price(
    &CircuitType::FheComputation(FheOperation::Threshold {
        threshold: 18,
        greater_or_equal: true,
    }),
    3, // 3 provers
);

println!("Precio mínimo aceptable: {} SOL", min_acceptable as f64 / 1e9);
// Salida: Precio mínimo aceptable: 0.027 SOL
// (0.005 × 1.5 × 1.2 × 3 = 0.027)
```

---

## Ejemplos de Integración con SDK

### Ejemplo 1: Job de Adición Simple

```rust
use zyberlink_sdk::instructions::InstructionBuilder;
use zyberlink_types::fhe::{FheOperation, FheConsensusConfig};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;

// Configuración
let rpc_url = "https://api.devnet.solana.com";
let client = RpcClient::new(rpc_url);
let program_id = Pubkey::from_str("YOUR_PROGRAM_ID")?;
let creator = Keypair::new();
let builder = InstructionBuilder::new(program_id);

// Definir operación
let operation = FheOperation::Add(10);
let cost_config = operation.get_cost_config();

// Crear configuración FHE
let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: cost_config.timeout_seconds,
    operation: operation.clone(),
};

// Preparar datos encriptados (esto vendría de la encriptación tfhe-rs)
let encrypted_data = vec![/* valor encriptado */];

// Calcular precio
let total_price = cost_config.min_payment_lamports * 3;

// Construir instrucción
let ix = builder.create_fhe_job(
    creator.pubkey(),
    1, // job_id
    &encrypted_data,
    fhe_config,
    total_price,
    cost_config.timeout_seconds,
)?;

// Crear y enviar transacción
let recent_blockhash = client.get_latest_blockhash()?;
let tx = Transaction::new_signed_with_payer(
    &[ix],
    Some(&creator.pubkey()),
    &[&creator],
    recent_blockhash,
);

let signature = client.send_and_confirm_transaction(&tx)?;
println!("Job creado: {}", signature);
```

### Ejemplo 2: Creación de Jobs en Lote

```rust
// Crear múltiples jobs con diferentes operaciones
let operations = vec![
    FheOperation::Add(5),
    FheOperation::Multiply(3),
    FheOperation::Threshold { threshold: 18, greater_or_equal: true },
];

for (idx, operation) in operations.iter().enumerate() {
    let cost_config = operation.get_cost_config();
    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: cost_config.timeout_seconds,
        operation: operation.clone(),
    };

    let total_price = cost_config.min_payment_lamports * 3;

    let ix = builder.create_fhe_job(
        creator.pubkey(),
        idx as u64,
        &encrypted_data[idx],
        fhe_config,
        total_price,
        cost_config.timeout_seconds,
    )?;

    // Crear y enviar transacción
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&creator.pubkey()),
        &[&creator],
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx)?;
    println!("Job {} creado ({}): {}", idx, operation.name(), sig);
}
```

---

## Ejemplos de Integración con API

### Ejemplo 1: Frontend JavaScript/TypeScript

```typescript
// estimate-cost.ts
interface EstimateCostRequest {
  operation: string;
  operation_value: number;
  expected_count?: number;
  bins?: number;
  required_provers: number;
}

interface EstimateCostResponse {
  operation: string;
  complexity_tier: number;
  min_payment_lamports: number;
  min_payment_sol: number;
  total_min_payment_lamports: number;
  total_min_payment_sol: number;
  timeout_seconds: number;
  estimated_compute_ms: number;
}

async function estimateJobCost(
  operation: string,
  params: Partial<EstimateCostRequest>
): Promise<EstimateCostResponse> {
  const request: EstimateCostRequest = {
    operation,
    operation_value: params.operation_value || 0,
    required_provers: params.required_provers || 3,
    ...params,
  };

  const response = await fetch('http://localhost:8080/api/estimate-cost', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Cost estimation failed: ${response.statusText}`);
  }

  return await response.json();
}

// Ejemplos de uso
async function main() {
  // Ejemplo 1: Adición simple
  const addCost = await estimateJobCost('add', {
    operation_value: 5,
    required_provers: 3,
  });
  console.log(`Add operation: ${addCost.total_min_payment_sol} SOL`);

  // Ejemplo 2: Histogram
  const histCost = await estimateJobCost('histogram', {
    bins: 5,
    required_provers: 3,
  });
  console.log(`Histogram (5 bins): ${histCost.total_min_payment_sol} SOL`);
  console.log(`Estimated time: ${histCost.timeout_seconds}s`);

  // Ejemplo 3: Suma censal
  const sumCost = await estimateJobCost('sum', {
    expected_count: 10000,
    required_provers: 5,
  });
  console.log(`Sum (10k items, 5 provers): ${sumCost.total_min_payment_sol} SOL`);
}
```

### Ejemplo 2: Backend Python

```python
import requests
from typing import Optional, Dict

class ZyberLinkPricingClient:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url

    def estimate_cost(
        self,
        operation: str,
        operation_value: int = 0,
        expected_count: Optional[int] = None,
        bins: Optional[int] = None,
        required_provers: int = 3
    ) -> Dict:
        """Estimar costo para operación FHE."""
        payload = {
            "operation": operation,
            "operation_value": operation_value,
            "required_provers": required_provers,
        }

        if expected_count is not None:
            payload["expected_count"] = expected_count
        if bins is not None:
            payload["bins"] = bins

        response = requests.post(
            f"{self.base_url}/api/estimate-cost",
            json=payload
        )
        response.raise_for_status()
        return response.json()

    def calculate_job_budget(
        self,
        operations: list[tuple[str, dict]]
    ) -> Dict:
        """Calcular presupuesto total para múltiples operaciones."""
        total_lamports = 0
        job_estimates = []

        for op_name, params in operations:
            estimate = self.estimate_cost(op_name, **params)
            total_lamports += estimate["total_min_payment_lamports"]
            job_estimates.append(estimate)

        return {
            "total_lamports": total_lamports,
            "total_sol": total_lamports / 1e9,
            "jobs": job_estimates,
        }

# Uso
if __name__ == "__main__":
    client = ZyberLinkPricingClient()

    # Estimación de un solo job
    cost = client.estimate_cost(
        operation="threshold",
        operation_value=18,
        required_provers=3
    )
    print(f"Costo verificación de edad: {cost['total_min_payment_sol']} SOL")

    # Presupuesto para múltiples jobs
    budget = client.calculate_job_budget([
        ("add", {"operation_value": 5, "required_provers": 3}),
        ("multiply", {"operation_value": 10, "required_provers": 3}),
        ("histogram", {"bins": 5, "required_provers": 3}),
    ])
    print(f"Presupuesto total: {budget['total_sol']} SOL")
```

---

## Integración Frontend

### Ejemplo de Componente Svelte

```svelte
<!-- CreateJobForm.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';

  type Operation = 'add' | 'multiply' | 'sum' | 'threshold' | 'histogram';

  let selectedOperation: Operation = 'add';
  let operationValue = 5;
  let expectedCount = 100;
  let bins = 5;
  let requiredProvers = 3;

  let costEstimate: any = null;
  let isEstimating = false;

  async function estimateCost() {
    isEstimating = true;

    const params: any = {
      operation: selectedOperation,
      operation_value: operationValue,
      required_provers: requiredProvers,
    };

    if (['sum', 'average', 'count_if'].includes(selectedOperation)) {
      params.expected_count = expectedCount;
    }

    if (selectedOperation === 'histogram') {
      params.bins = bins;
    }

    try {
      const response = await fetch('/api/estimate-cost', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
      });

      costEstimate = await response.json();
    } catch (error) {
      console.error('Cost estimation failed:', error);
    } finally {
      isEstimating = false;
    }
  }

  // Re-estimar al cambiar parámetros
  $: if (selectedOperation || operationValue || expectedCount || bins || requiredProvers) {
    estimateCost();
  }
</script>

<div class="job-form">
  <h2>Crear Job FHE</h2>

  <label>
    Operación:
    <select bind:value={selectedOperation}>
      <option value="add">Add (Tier 1)</option>
      <option value="multiply">Multiply (Tier 1)</option>
      <option value="sum">Sum (Tier 2)</option>
      <option value="threshold">Threshold (Tier 3)</option>
      <option value="histogram">Histogram (Tier 5)</option>
    </select>
  </label>

  {#if ['add', 'multiply', 'threshold'].includes(selectedOperation)}
    <label>
      Valor:
      <input type="number" bind:value={operationValue} min="0" max="255" />
    </label>
  {/if}

  {#if ['sum', 'average'].includes(selectedOperation)}
    <label>
      Cantidad Esperada:
      <input type="number" bind:value={expectedCount} min="1" max="10000" />
    </label>
  {/if}

  {#if selectedOperation === 'histogram'}
    <label>
      Número de Bins:
      <input type="number" bind:value={bins} min="2" max="50" />
      {#if bins > 10}
        <span class="warning">⚠️ Alto número de bins = operación costosa</span>
      {/if}
    </label>
  {/if}

  <label>
    Provers Requeridos:
    <input type="range" bind:value={requiredProvers} min="2" max="10" />
    <span>{requiredProvers} provers</span>
  </label>

  {#if costEstimate && !isEstimating}
    <div class="cost-display">
      <h3>Estimación de Costo</h3>
      <div class="cost-details">
        <div class="tier">Tier {costEstimate.complexity_tier}</div>
        <div class="price">
          <strong>{costEstimate.total_min_payment_sol.toFixed(6)} SOL</strong>
          <small>({costEstimate.total_min_payment_lamports.toLocaleString()} lamports)</small>
        </div>
        <div class="breakdown">
          {costEstimate.min_payment_sol.toFixed(6)} SOL × {requiredProvers} provers
        </div>
        <div class="timeout">
          Timeout: {costEstimate.timeout_seconds}s
          ({Math.floor(costEstimate.timeout_seconds / 60)} min)
        </div>
      </div>

      {#if costEstimate.total_min_payment_sol > 1.0}
        <div class="warning">
          ⚠️ Esta es una operación costosa. Considera reducir provers o bins.
        </div>
      {/if}
    </div>
  {/if}

  <button on:click={createJob} disabled={isEstimating || !costEstimate}>
    Crear Job
  </button>
</div>

<style>
  .warning { color: orange; font-size: 0.9em; }
  .cost-display {
    background: #f5f5f5;
    padding: 1rem;
    border-radius: 8px;
    margin: 1rem 0;
  }
  .tier {
    display: inline-block;
    background: #007bff;
    color: white;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8em;
  }
  .price strong { font-size: 1.5em; }
  .breakdown { color: #666; font-size: 0.9em; }
</style>
```

---

## Mejores Prácticas

### 1. Siempre Estimar Antes de Crear Jobs

```rust
// ❌ NO HACER: Crear job sin verificar precio
let ix = builder.create_fhe_job(creator, job_id, data, config, 1_000_000, 60)?;
// Esto probablemente fallará on-chain

// ✅ HACER: Calcular precio mínimo primero
let operation = FheOperation::Threshold { threshold: 18, greater_or_equal: true };
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;
let ix = builder.create_fhe_job(creator, job_id, data, config, min_price, cost_config.timeout_seconds)?;
```

### 2. Usar Timeouts Dinámicos

```rust
// ❌ NO HACER: Usar timeouts hardcodeados
let ix = builder.create_fhe_job(creator, job_id, data, config, price, 300)?;

// ✅ HACER: Usar timeout dinámico de cost config
let cost_config = operation.get_cost_config();
let ix = builder.create_fhe_job(creator, job_id, data, config, price, cost_config.timeout_seconds)?;
```

### 3. Advertir a Usuarios Sobre Operaciones Costosas

```typescript
// En tu frontend
if (costEstimate.complexity_tier >= 5) {
  alert(`Advertencia: Las operaciones Tier ${costEstimate.complexity_tier} son costosas. ` +
        `Este job costará ${costEstimate.total_min_payment_sol} SOL.`);
}
```

### 4. Agrupar Operaciones Similares en Lotes

```rust
// ❌ NO HACER: Crear jobs separados para operaciones similares
for value in vec![5, 10, 15, 20] {
    create_job(FheOperation::Add(value))?; // 4 jobs = costo 4×
}

// ✅ HACER: Procesar múltiples valores en un job si es posible
// O al menos advertir sobre el costo total
let total_cost = estimate_cost(Add(5)) * 4;
println!("Crear 4 jobs costará {} SOL", total_cost);
```

### 5. Manejar Cambios en Cantidad de Provers

```typescript
// Recalcular costo cuando cambia la cantidad de provers
proverCountSlider.addEventListener('change', async (e) => {
  const newProverCount = e.target.value;
  const newEstimate = await estimateCost({
    ...currentParams,
    required_provers: newProverCount
  });
  updateCostDisplay(newEstimate);
});
```

---

## Solución de Problemas

### Error: InvalidPrice en Creación de Job

**Síntoma:**
```
Transaction failed: InvalidPrice
Price too low for FHE operation 'Threshold' (tier 3): 5000000 < 15000000
```

**Causa:** El precio del job está por debajo del mínimo requerido para la operación y número de provers.

**Solución:**
```rust
// Obtener precio mínimo de cost config
let cost_config = operation.get_cost_config();
let min_price = cost_config.min_payment_lamports * required_provers;

// Usar mínimo o mayor
let ix = builder.create_fhe_job(..., min_price, ...)?;
```

### Estimación de Costo Devuelve Error

**Síntoma:**
```json
{"error": "Unknown operation: threshhold"}
```

**Causa:** Error tipográfico en nombre de operación u operación no soportada.

**Solución:** Usar nombres de operación exactos:
- `"add"`, `"multiply"`, `"sum"`, `"threshold"`, `"range_check"`, `"average"`, `"count_if"`, `"histogram"`

### Job Expira Antes de Completarse

**Síntoma:** El job expira antes de que los provers puedan enviar resultados.

**Causa:** Usar timeout hardcodeado que es demasiado corto para la operación.

**Solución:**
```rust
// Usar timeout dinámico
let cost_config = operation.get_cost_config();
let ix = builder.create_fhe_job(
    creator,
    job_id,
    data,
    config,
    price,
    cost_config.timeout_seconds  // ✅ Usar esto
)?;
```

### Prover Rechaza Todos los Jobs

**Síntoma:** Los logs del nodo prover muestran todos los jobs como "no rentables".

**Causa:** Umbrales de ROI demasiado altos o multiplicador de costo operacional demasiado alto.

**Solución:**
```rust
// Ajustar parámetros de ROI calculator
let calculator = ROICalculator::new(
    10.0,  // Bajar ROI mínimo (era 50%)
    1.2    // Bajar overhead (era 2.0)
);
```

---

## Preguntas Frecuentes

### Q: ¿Puedo pagar más que el precio mínimo?

**A:** ¡Sí! Pagar por encima del mínimo puede:
- Incentivar respuesta más rápida del prover
- Actuar como tarifa prioritaria
- Compensar por fluctuaciones de mercado

```rust
let min_price = cost_config.min_payment_lamports * 3;
let premium_price = min_price + 5_000_000; // +0.005 SOL de bonificación
let ix = builder.create_fhe_job(..., premium_price, ...)?;
```

### Q: ¿Qué sucede si el precio de SOL cambia?

**A:** Los precios dinámicos están denominados en lamports (moneda on-chain), no USD. Si el precio de SOL se duplica:
- La operación aún cuesta los mismos lamports
- Pero el costo en USD se duplica
- Futuro: Considerar implementar precios vinculados a stablecoin USD

### Q: ¿Puedo usar valores de timeout personalizados?

**A:** Sí, pero deben ser >= timeout dinámico:

```rust
let cost_config = operation.get_cost_config();
let custom_timeout = cost_config.timeout_seconds * 2; // Buffer 2x
let ix = builder.create_fhe_job(..., price, custom_timeout)?;
```

### Q: ¿Cómo sé en qué tier está una operación?

**A:**
```rust
let operation = FheOperation::Histogram { bins: vec![...] };
let config = operation.get_cost_config();
println!("Tier: {}", config.complexity_tier);
```

O consulta la [documentación principal](./DYNAMIC_PRICING.md#complexity-tiers).

### Q: ¿Hay descuentos por volumen para múltiples jobs?

**A:** Actualmente no. Cada job paga el costo por prover. Una mejora futura podría implementar:
- Descuentos por volumen
- Modelos de suscripción
- Descuentos basados en stake

### Q: ¿Pueden los provers negociar precios?

**A:** No en la implementación actual. Los precios son determinísticos basados en la complejidad de la operación. El futuro podría implementar:
- Sistema de licitación de provers
- Niveles premium/económicos
- Mecánicas de subasta holandesa

---

## Próximos Pasos

- **[Análisis Técnico Profundo](./DYNAMIC_PRICING_TECHNICAL.md)** - Entender los algoritmos
- **[Referencia API](./API_REFERENCE.md)** - Documentación completa de API
- **[Documentación Principal](./DYNAMIC_PRICING.md)** - Vista general del sistema

---

**Última Actualización:** 2025-11-21
**¿Preguntas?** Abre un issue en GitHub o pregunta en Discord
