# Mapa de Repositorios

ZyberLink esta dividido en multiples repositorios. Este GitBook se construye desde `zyb-platform/docs`, por lo que algunas guias apuntan a otros repos segun el componente.

Usa esta pagina para elegir el repo correcto cuando una guia dice "clona el repositorio".

## Repositorios Principales

| Repo | Proposito | Archivos | URL |
| --- | --- | --- | --- |
| **zyb-platform** | Orquestacion, deployment, docker-compose | Makefile, scripts | https://github.com/8ctag0n/13 |
| **zyb-chain** | Contratos on-chain (Solana, Starknet, Aptos) + SDKs | 269 | https://github.com/8ctag0n/21 |
| **zyb-kernel** | Core libs (crypto, fhe, types, jobs) | 19 | https://github.com/8ctag0n/5 |
| **zyb-circuits** | Circuitos ZK (Halo2, Circom) | 30 | https://github.com/8ctag0n/8 |
| **zyb-cli** | CLI principal (`zyb`) | 24 | https://github.com/8ctag0n/144 |
| **zyb-compute** | Prover nodes y computacion | 59 | https://github.com/8ctag0n/34 |
| **zyb-sdks** | SDKs multi-lenguaje (Rust, TS, Go) | 52 | https://github.com/8ctag0n/89 |
| **zyb-services** | Backend services (API, sync, gateway) | 61 | https://github.com/8ctag0n/55 |
| **zyb-apps** | Frontend apps (wallet, webapp, design-system) | 40 | https://github.com/8ctag0n/233 |

Otros repos en este checkout:
- `verticals`: Ejemplos e integraciones por vertical.

## Ejemplos de Clonado

Organizacion: `8ctagon`

```bash
git clone https://github.com/8ctagon/zyb-platform.git
git clone https://github.com/8ctagon/zyb-chain.git
git clone https://github.com/8ctagon/zyb-compute.git
git clone https://github.com/8ctagon/zyb-cli.git
git clone https://github.com/8ctagon/zyb-apps.git
git clone https://github.com/8ctagon/zyb-services.git
git clone https://github.com/8ctagon/zyb-sdks.git
git clone https://github.com/8ctagon/zyb-kernel.git
git clone https://github.com/8ctagon/zyb-circuits.git
```

## Reporte de Issues

Abre issues en el repo dueño del componente. Si no estas seguro, arranca con `zyb-platform` y linkea la guia correspondiente.

## Changelogs

Los changelogs viven en cada repo. Arranca por:

- https://github.com/8ctag0n/13/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/21/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/5/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/8/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/144/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/34/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/89/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/55/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/233/blob/main/CHANGELOG.md
