# Guía de Circuitos ZK

## Descripción General

ZyberLink utiliza circuitos Zero-Knowledge (ZK) para proporcionar computaciones verificables y privadas. Estos circuitos permiten a los usuarios probar propiedades de sus datos (ej. membresía en una lista, umbrales de patrimonio neto) sin revelar los datos en sí.

## Categorías de Circuitos

El protocolo soporta varios circuitos ZK especializados ubicados en `zyb-circuits/`:

### 1. Core e Identidad (`poi/`)
- **Proof of Innocence (POI)**: Prueba que un usuario NO está presente en una lista negra específica (ej. lista de sanciones).
- **Membresía**: Prueba que un usuario pertenece a un grupo específico sin revelar su identidad.

### 2. Gobernanza (`vote/`)
- **Voto Privado**: Asegura el secreto del voto permitiendo la verificación pública del conteo.
- **Voto con POI**: Combina verificación de identidad con voto anónimo para prevenir que entidades sancionadas participen en la gobernanza.

### 3. Mercado y DeFi (`market/`, `blind/`)
- **Apuesta de Mercado**: Valida que una apuesta se realice dentro de los límites permitidos sin revelar la estrategia exacta.
- **Apuestas Ciegas**: Lógica de juego encriptada donde los resultados se verifican vía ZK.
- **Balance Privado**: Maneja transferencias de tokens encriptadas y actualizaciones de estado de balance.

---

## Stack Tecnológico

- **Lenguaje**: Circom 2.1
- **Sistema de Pruebas**: Groth16 (vía SnarkJS)
- **Curva**: BN128
- **Verificación Backend**: `ark-groth16` (Rust)

---

## Flujo de Trabajo de Desarrollo

### 1. Diseño del Circuito
Los circuitos se definen en archivos `.circom`.

```text
template Example() {
    signal input in;
    signal output out;
    out <== in * in;
}
```

### 2. Compilación
Usa los scripts proporcionados para compilar circuitos:

```bash
cd zyb-circuits/poi
./scripts/compile.sh
```

### 3. Setup de Confianza (Trusted Setup)
El protocolo utiliza un setup de confianza por circuito (Powers of Tau).

### 4. Generación de Pruebas (Prover)
Los provers generan un archivo `.json` de prueba y un archivo `public.json` de entradas.

---

## Verificación On-Chain

Las pruebas ZK se verifican en Solana vía el programa `zk-generator`. El protocolo compara el hash de la prueba enviado por el prover con los requisitos definidos en el trabajo.

| ID Circuito | Estado de Implementación |
|-------------|--------------------------|
| 10 (POI)    | ✅ Listo para Producción |
| 20 (Voto)   | ✅ Listo para Producción |
| 30 (DeFi)   | ⚠️ Beta (Pruebas)        |
| 40 (Port)   | ⚠️ Alpha (Investigación) |
