# ZyberLink

**Red Descentralizada Multi-Prover para Computaciones que Preservan Privacidad**

## Vision

ZyberLink es un marketplace descentralizado para computaciones FHE (Fully Homomorphic Encryption) y ZK (Zero-Knowledge) en Solana. Habilitamos infraestructura que preserva privacidad para cualquier aplicación al distribuir la confianza a través de una red de provers independientes que compiten para ejecutar computaciones criptográficas.

Nuestra mision es hacer las computaciones que preservan privacidad accesibles, asequibles y resistentes a la censura mediante consenso descentralizado multi-prover.

## El Problema

Las aplicaciones modernas requieren computaciones criptográficas intensivas (FHE, pruebas ZK) que son:

- **Demasiado lentas en dispositivos moviles** - Minutos de computación, alto consumo de batería
- **Centralizadas cuando se externalizan** - Confianza en servidores únicos, riesgo de censura
- **Costosas de verificar** - No hay forma económica de asegurar la corrección

## Nuestra Solucion

ZyberLink resuelve esto mediante **consenso multi-prover**:

1. **Cliente** encripta datos sensibles y publica un trabajo de computación en Solana
2. **Multiples provers** compiten para ejecutar la computación independientemente
3. **Algoritmo de consenso** asegura corrección (acuerdo 2-de-3 o 3-de-5)
4. **Pagos automatizados** recompensan provers honestos, penalizan deshonestos
5. **Descentralizacion** asegura que no hay punto único de falla o censura

## Caracteristicas Clave

- **Consenso Multi-Prover** - Confianza distribuida entre provers independientes
- **Soporte FHE & ZK** - Encriptación totalmente homomórfica (TFHE-rs) + circuitos zero-knowledge
- **Privacidad Preservada** - Datos de witness encriptados, provers nunca ven texto plano
- **Resistente a Censura** - Red sin permisos, sin autoridad central
- **Verificacion On-Chain** - Distribución de pagos sin confianza vía consenso
- **Preparado Post-Cuantico** - Intercambio de claves ML-KEM para privacidad a prueba de futuro

## Casos de Uso

### Aplicaciones Potenciadas por FHE
- **DeFi Privado** - Intercambios de balances encriptados sin revelar montos
- **Votacion Confidencial** - Gobernanza privada de DAOs con resultados verificables
- **Analitica Privada** - Computación sobre datasets encriptados sin desencriptar
- **Computacion Segura Multi-Parte** - Computación distribuida sin confianza

### Infraestructura ZK
- **Pruebas para Wallets Moviles** - Externalizar generación de pruebas de teléfonos a la red
- **Agregacion de Pruebas** - Procesar múltiples pruebas eficientemente por lotes
- **Privacidad Cross-Chain** - Puente de pruebas ZK entre blockchains

## Como Funciona

```
┌──────────────┐
│   Cliente    │ (App móvil, web app, etc.)
└──────┬───────┘
       │ 1. Encriptar witness + crear trabajo
       ▼
┌─────────────────────────┐
│  Marketplace Solana     │ (Coordinación de trabajos on-chain)
└────┬────────────────────┘
     │ 2. Difusión de trabajo
     ▼
┌────────────────────────────────────────────┐
│          Red de Provers                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Prover A │  │ Prover B │  │ Prover C │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
└───────│─────────────│─────────────│────────┘
        │ 3. Ejecutar │             │
        │    (async)  │             │
        ▼             ▼             ▼
┌────────────────────────────────────────────┐
│   Consenso + Verificacion On-Chain         │
│   A: hash_abc  B: hash_abc (OK) C: hash_def (Fallo)│
│   -> Pagar A & B, penalizar C              │
└────────────────────────────────────────────┘
```

## Stack Tecnologico

- **Blockchain:** Solana (alto rendimiento, bajas comisiones)
- **Motor FHE:** TFHE-rs (Zama)
- **Circuitos ZK:** Halo2
- **Nodo Prover:** Rust
- **Encriptacion:** ML-KEM (post-cuántica)
- **UI:** Terminal User Interface (TUI) con ratatui

## Estado del Proyecto

**En Desarrollo** - Zypherpunk Hackathon (10 Nov - 1 Dic, 2025)

**Caracteristicas Actuales:**
- Marketplace multi-prover en Solana
- Motor de computación FHE (TFHE-rs)
- Algoritmo de consenso on-chain
- Interfaz de terminal para monitoreo de provers
- Asistente de configuración interactivo
- 14/14 tests E2E de FHE pasando

## Enlaces Rapidos

- [Primeros Pasos](primeros-pasos/inicio-rapido.md) - Ejecuta tu primer trabajo
- [Arquitectura](arquitectura/vision-general.md) - Análisis técnico profundo
- [Guias](guias/) - Guías y tutoriales para desarrolladores

## Participa

ZyberLink está actualmente en desarrollo de hackathon. Después del 1 de diciembre de 2025, abriremos para:

- **Beta Testers** - Ejecutar un nodo prover y ganar recompensas
- **Socios de Integracion** - Construir aplicaciones que preservan privacidad
- **Contribuidores** - Ayudar a construir la infraestructura de privacidad descentralizada

---

**Construido con privacidad, potenciado por descentralización.**

## Siguiente Paso

[Primeros Pasos](primeros-pasos/README.md) - Comienza tu viaje con ZyberLink.