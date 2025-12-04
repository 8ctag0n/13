# Diseño FHE

Cómo ZyberLink usa FHE (Fully Homomorphic Encryption) con consenso multi-prover.

## Fundamentos

- **Librería:** TFHE-rs
- **Tipo de dato:** `FheUint8`
- **Operaciones soportadas:** suma, resta, multiplicación, comparaciones y booleanos.

El cliente encripta los datos antes de publicarlos on-chain; los provers nunca ven texto plano.

## Flujo de Consenso FHE

1. **Claim múltiple:** 3 provers reclaman el mismo job.
2. **Ejecución independiente:** cada prover computa el resultado sobre datos encriptados.
3. **Subida de resultados:** se envía hash/resultado encriptado on-chain.
4. **Umbral:** si `consensus_threshold` provers coinciden (p.ej. 2 de 3), se paga a quienes coinciden.
5. **Penalización:** los provers en desacuerdo no cobran (y pueden perder reputación en versiones futuras).

## Qué ve cada rol

- **Cliente:** conoce claves de cliente y puede desencriptar el resultado.
- **Provers:** solo ven `witness.bin` (datos encriptados) y `server_key.bin`.
- **On-chain:** almacena hashes/resultados encriptados y gestiona pagos; no ve datos en claro.

## Seguridad

- **Sin necesidad de confiar en un único prover:** se requiere corrupción simultánea de 2+ provers.
- **Datos siempre encriptados:** tanto en tránsito como en cómputo.
- **Limitaciones:** no tolera mayoría bizantina ni ataques de canal lateral en hardware de provers.

## Rendimiento

- El cómputo FHE es costoso; los tiempos dependen de la complejidad y del número de bins/operaciones.
- Optimización rápida: correr varios provers en paralelo y usar precios mínimos adecuados por operación.
- Para histogramas y operaciones pesadas, ver [Optimización Histograma FHE](histograma-fhe-optimizaciones.md).
