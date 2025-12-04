# Arquitectura

Aprende sobre la arquitectura técnica de ZyberLink y decisiones de diseño.

## En Esta Sección

* [Visión General](vision-general.md) - Arquitectura de alto nivel y componentes
* [Diseño FHE](diseno-fhe.md) - Implementación de Fully Homomorphic Encryption
* [Stack Tecnológico](stack-tecnologico.md) - Tecnologías, librerías y herramientas

## Conceptos Clave

### Consenso Multi-Prover

ZyberLink usa un mecanismo de consenso novedoso donde múltiples provers independientes ejecutan la misma computación y envían resultados. El sistema solo acepta resultados cuando un umbral de provers (ej. 2-de-3) están de acuerdo.

### Witness Encriptado

Todos los datos sensibles se encriptan antes de ser publicados on-chain. Los provers ejecutan computaciones sobre datos encriptados sin nunca ver el texto plano.

### Verificación On-Chain

El programa de Solana verifica el consenso y automáticamente distribuye pagos a provers honestos mientras penaliza a los deshonestos.

---

Explora cada sección para entender cómo ZyberLink logra computación descentralizada que preserva privacidad.
