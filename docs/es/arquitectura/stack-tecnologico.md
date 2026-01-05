# Stack Tecnológico

Visión general de tecnologías, librerías y herramientas de ZyberLink.

## Backend / Provers
- **Rust** para lógica core.
- **TFHE-rs** para cómputo FHE.
- **Halo2** para circuitos ZK (planificado).
- **Rayon** para paralelismo en provers.

## Servicios Backend (Arquitectura de 3 Capas)

ZyberLink utiliza una arquitectura de microservicios por niveles para seguridad y escalabilidad:

1.  **Public API (Gateway)**: Punto de entrada basado en Axum. Maneja rate limiting y validación inicial.
2.  **x402 Server (Middleware)**: Capa de anti-spam y verificación de pagos.
3.  **Blink Server (Core)**: Servicio Actix-web que gestiona el estado de la DB, lógica de atestación y sincronización con la cadena.

## Blockchain
- **Solana Program** (Rust, BPF).
- **CLI Solana 2.1+** para despliegue y gestión.

## CLI y Utilidades
- **zyb** (`zyb fhe encrypt`, `zyb fhe decrypt`) para generar `witness.bin` y claves.
- **Scripts** de demo y pruebas E2E (`./scripts`, `demo/`).

## Frontend
- **Webapp** (Svelte) para crear jobs, subir `witness.bin` y monitorear estados.
- **TUI** (ratatui) para monitorear provers y stats.

## Observabilidad y Ops
- **systemd** recomendado para servicios de provers.
- **Logs** accesibles vía `journalctl` o archivos locales.

## Requisitos de entorno
- **Rust 1.75+**, **Solana CLI 2.1+**.
- Provers: 16GB RAM / 8+ cores recomendado; conexión estable.

Para detalles de despliegue, ver [Despliegue](../guias/despliegue.md) y para el CLI ver [Zyb CLI (FHE)](../guias/fhe-cli.md).
