# Inicio Rápido

Pon ZyberLink en funcionamiento en 5 minutos.

## Prerequisitos

Antes de comenzar, asegúrate de tener:

- **Rust 1.75+** - [Instalar Rust](https://rustup.rs/)
- **Solana CLI 2.1+** - [Instalar Solana](https://docs.solana.com/cli/install-solana-cli-tools)
- **Entorno tipo Unix** - Linux, macOS, o WSL2

## Configuración Rápida

### 1. Clonar el Repositorio

```bash
git clone https://github.com/yourusername/zyberlink.git
cd zyberlink
```

### 2. Compilar el Proyecto

```bash
cargo build --release
```

Esto compilará todos los componentes: el programa de Solana, SDK, nodo prover y utilidades.

**Tiempo esperado:** 3-5 minutos en la primera compilación.

### 3. Iniciar Validador Local

Abre una nueva terminal e inicia un validador local de Solana:

```bash
solana-test-validator --reset
```

Déjalo corriendo en segundo plano.

### 4. Desplegar el Programa

En tu terminal principal, despliega el programa marketplace de ZyberLink:

```bash
# Compilar el programa de Solana
cargo build-sbf

# Desplegar al validador local
solana program deploy target/deploy/cypherlink.so
```

Guarda el **Program ID** que aparece después del despliegue - lo necesitarás después.

### 5. Ejecutar el Demo Interactivo

La forma más rápida de ver ZyberLink en acción:

```bash
cd demo
./run-demo.sh
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

### 6. Detener el Demo

Presiona `q` en el TUI para apagar el nodo prover correctamente.

## ¿Qué Sigue?

Ahora que tienes ZyberLink funcionando:

- **[Guía del Demo](demo.md)** - Explora el demo interactivo en detalle
- **[Arquitectura](../arquitectura/vision-general.md)** - Entiende cómo funciona ZyberLink
- **[Guías](../guias/)** - Despliega a producción

## Solución de Problemas

### Falla la compilación con errores de linking

Asegúrate de tener la última versión de Rust:
```bash
rustup update
```

### Solana CLI no encontrado

Agrega Solana a tu PATH:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### Falla el despliegue del programa

Verifica que el validador local esté corriendo:
```bash
solana cluster-version
```

Debería mostrar información de versión si el validador es accesible.

### ¿Necesitas ayuda?

- Consulta los [GitHub Issues](https://github.com/yourusername/zyberlink/issues)
- Únete a nuestras discusiones comunitarias

## Configuración Avanzada

### Ejecutar Tu Propio Nodo Prover

En vez de usar el script del demo, puedes ejecutar un nodo prover manualmente:

```bash
cd prover-node
cargo run --release -- wizard
```

El asistente interactivo te guiará a través de:
- Configuración de wallet
- Configuración de RPC de Solana
- Configuración de Program ID
- Registro de prover

### Ejecutar Tests

Verifica que todo funcione correctamente:

```bash
# Tests unitarios
cargo test

# Tests de integración
cargo test --test integration_tests

# Tests E2E (requiere validador + prover corriendo)
./scripts/test-e2e.sh
```

## Lo Que Construiste

Ahora tienes un entorno de desarrollo completo de ZyberLink con:

- ✅ Validador local de Solana
- ✅ Programa marketplace de ZyberLink desplegado
- ✅ Nodo prover listo para procesar trabajos
- ✅ Infraestructura de demo para testing

¿Listo para profundizar? Consulta la [Guía del Demo](demo.md) para explorar todas las funcionalidades.
