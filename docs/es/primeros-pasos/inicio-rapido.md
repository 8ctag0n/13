# Inicio Rápido

Pon ZyberLink en funcionamiento en 5 minutos.

## Prerequisitos

Antes de comenzar, asegúrate de tener:

- **Rust 1.75+** - [Instalar Rust](https://rustup.rs/)
- **Solana CLI 2.1+** - [Instalar Solana](https://docs.solana.com/cli/install-solana-cli-tools)
- **Docker + Docker Compose** - Requerido para el stack local
- **Entorno tipo Unix** - Linux, macOS, o WSL2

## Configuración Rápida

### 1. Clonar el Repositorio de Plataforma

```bash
git clone https://github.com/8ctagon/zyb-platform.git
cd zyb-platform
```

### 2. Configurar el Entorno Local

```bash
cp zyb.example.toml zyb.toml
cp .env.example .env
```

### 3. Levantar el Stack Local

Usa Docker Compose:

```bash
docker-compose up -d
```

O usa el Makefile (recomendado):

```bash
make start
```

### 4. Ejecutar el Demo Interactivo

La forma más rápida de ver ZyberLink en acción:

```bash
./demo.sh
```

Este script:
1. Configura el entorno local
2. Inicia un nodo prover con interfaz TUI
3. Genera trabajos de computación FHE de ejemplo
4. Muestra procesamiento en tiempo real

**Lo que verás:**

- Interfaz de terminal mostrando estado del prover
- Trabajos siendo reclamados y procesados
- Computaciones FHE completándose
- Ganancias acumulándose
- Estadísticas del sistema en vivo

### 5. Detener el Demo

Presiona `q` en el TUI para apagar el nodo prover correctamente, luego detén el stack:

```bash
make stop
# o: docker-compose down
```

## ¿Qué Sigue?

Ahora que tienes ZyberLink funcionando:

- **[Guía del Demo](demo.md)** - Explora el demo interactivo en detalle
- **[Arquitectura](../arquitectura/vision-general.md)** - Entiende cómo funciona ZyberLink
- **[Guías](../guias/)** - Despliega a producción

## Solución de Problemas

### Docker Compose no inicia

Verifica estado y logs:
```bash
docker-compose ps
docker-compose logs -f
```

### Solana CLI no encontrado

Agrega Solana a tu PATH:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### ¿Necesitas ayuda?

- Revisa el [Mapa de Repositorios](repositorios.md) para el issue tracker correcto
- Únete a nuestras discusiones comunitarias

## Configuración Avanzada

### Ejecutar Tu Propio Nodo Prover

En vez de usar el script del demo, puedes ejecutar un nodo prover manualmente desde `zyb-compute`.
Sigue la [Guía de Configuración de Prover](../guias/configuracion-prover.md).

### Ejecutar Tests

Los tests están organizados por repositorio. Empieza por `zyb-platform/tests` y los README de cada repo.

## Lo Que Construiste

Ahora tienes un entorno de desarrollo completo de ZyberLink con:

- ✅ Validador local de Solana
- ✅ Programa marketplace de ZyberLink desplegado
- ✅ Nodo prover listo para procesar trabajos
- ✅ Infraestructura de demo para testing

¿Listo para profundizar? Consulta la [Guía del Demo](demo.md) para explorar todas las funcionalidades.
