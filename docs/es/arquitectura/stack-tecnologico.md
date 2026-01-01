# Stack Tecnológico

Visión general de tecnologías, librerías y herramientas de ZyberLink.

## Backend / Provers
- **Rust** para lógica core.
- **TFHE-rs** para cómputo FHE.
- **Halo2** para circuitos ZK (planificado).
- **Rayon** para paralelismo en provers.

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
