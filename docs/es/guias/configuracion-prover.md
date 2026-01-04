# Guía de Configuración de Nodo Prover

Ejecuta un nodo prover de ZyberLink para procesar computaciones que preservan privacidad y ganar SOL.

## Descripción General

Un nodo prover de ZyberLink es un servicio que ejecuta computaciones FHE sobre datos encriptados. La implementación actual es un prototipo fundacional enfocado en demostrar el concepto central de marketplace de computación que preserva privacidad.

**Lo que hacen los provers:**
- Hacen polling del marketplace on-chain para trabajos pendientes
- Reclaman trabajos basados en verificaciones simples de rentabilidad
- Descargan datos de witness encriptados
- Ejecutan operaciones FHE sobre texto cifrado
- Envían resultados para verificación de consenso

**Estado Actual:** Prototipo temprano. Funcionalidad básica funcionando, características avanzadas en desarrollo.

## Requisitos Previos

### Requisitos de Hardware

**Mínimo:**
- 8 GB RAM
- 4 núcleos CPU
- 20 GB almacenamiento
- Conexión a Internet

**Recomendado:**
- 16 GB RAM
- 8 núcleos CPU
- 50 GB almacenamiento

**Nota:** Las operaciones FHE son intensivas en CPU. Más núcleos = mejor rendimiento.

### Requisitos de Software

- **Rust** 1.75+
- **Solana CLI** 1.18+
- **Git**
- Conocimiento básico de Linux/Unix

### Financiamiento Inicial

- ~0.1-0.2 SOL para registro y fees de transacción

## Inicio Rápido

### 1. Instala Dependencias

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev curl git

# Instala Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Instala Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### 2. Clona y Compila

Reemplaza `<ORG>` por tu organizacion de GitHub o mirror.

```bash
# Clona el repositorio
git clone https://github.com/<ORG>/zyb-compute.git
cd zyb-compute

# Compila prover (toma 5-10 minutos)
cargo build --release --bin zyberlink-prover

# Ubicación del binario: target/release/zyberlink-prover
```

### 3. Crea Keypair

```bash
# Genera keypair del prover
solana-keygen new --outfile ~/.config/solana/prover.json

# ¡Respalda este archivo - es tu identidad de prover!
cp ~/.config/solana/prover.json ~/prover-backup.json
```

### 4. Fondea la Wallet

**Para Devnet (pruebas):**
```bash
solana config set --url https://api.devnet.solana.com
solana airdrop 2 ~/.config/solana/prover.json
```

**Para Mainnet:**
```bash
solana config set --url https://api.mainnet-beta.solana.com
# Transfiere SOL desde tu wallet principal
solana transfer <PROVER_PUBKEY> 0.2 --from ~/.config/solana/id.json
```

### 5. Configura el Entorno

```bash
# Establece variables de entorno requeridas
export SOLANA_RPC_URL=https://api.devnet.solana.com
export ZYBERLINK_PROGRAM_ID=<PROGRAM_ID>  # Obtén del equipo/docs
export WITNESS_BACKEND_URL=http://localhost:8080
export RUST_LOG=info
```

### 6. Registra el Prover

Registra tu prover on-chain:

```bash
./target/release/zyberlink-prover register \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --stake-amount 100000000  # 0.1 SOL
```

**Nota:** Este es un registro único. Tu stake está bloqueado hasta que te des de baja.

### 7. Ejecuta el Prover

Inicia el nodo prover:

```bash
./target/release/zyberlink-prover \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --witness-backend-url $WITNESS_BACKEND_URL
```

**Salida esperada:**
```
[INFO] Iniciando Nodo Prover de ZyberLink
[INFO] Autoridad del Prover: 7vX8h9...
[INFO] ID del Programa: ZyberLink...
[INFO] Haciendo polling de trabajos cada 5 segundos...
[INFO] Encontrados 0 trabajos pendientes
[INFO] Encontrados 2 trabajos pendientes
[INFO] Procesando trabajo 1...
[INFO] [Trabajo 1] Reclamado exitosamente
[INFO] [Trabajo 1] Descargando witness...
[INFO] [Trabajo 1] Ejecutando operación FHE...
[INFO] [Trabajo 1] ¡Completado! ✅
```

## Opciones de Configuración

Argumentos de línea de comandos:

| Argumento | Descripción | Por Defecto |
|-----------|-------------|-------------|
| `--rpc-url` | Endpoint RPC de Solana | http://localhost:8899 |
| `--program-id` | ID del programa ZyberLink | Requerido |
| `--keypair` | Ruta al keypair del prover | ~/.config/solana/id.json |
| `--witness-backend-url` | URL del servidor de witness | http://localhost:8080 |
| `--poll-interval` | Intervalo de polling de trabajos (segundos) | 5 |
| `--min-roi` | ROI mínimo para aceptar trabajos (%) | 20.0 |
| `--max-concurrent-jobs` | Trabajos paralelos máximos | 3 |
| `--tui-mode` | Habilitar UI de terminal | false |

**Ejemplo con configuraciones personalizadas:**
```bash
./target/release/zyberlink-prover \
  --rpc-url https://api.devnet.solana.com \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --keypair ~/.config/solana/prover.json \
  --witness-backend-url http://localhost:8080 \
  --poll-interval 5 \
  --min-roi 15.0 \
  --max-concurrent-jobs 2
```

## Ejecutar como Servicio

Para operación continua, ejecuta el prover como servicio systemd:

### 1. Crea Archivo de Servicio

```bash
sudo nano /etc/systemd/system/zyberlink-prover.service
```

### 2. Agrega Configuración

```ini
[Unit]
Description=Nodo Prover de ZyberLink
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/zyb-compute
Environment="RUST_LOG=info"
Environment="SOLANA_RPC_URL=https://api.devnet.solana.com"
ExecStart=/home/YOUR_USERNAME/zyb-compute/target/release/zyberlink-prover \
  --program-id YOUR_PROGRAM_ID \
  --witness-backend-url http://localhost:8080
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### 3. Habilita e Inicia

```bash
# Recarga systemd
sudo systemctl daemon-reload

# Inicia servicio
sudo systemctl start zyberlink-prover

# Habilita al arrancar
sudo systemctl enable zyberlink-prover

# Verifica estado
sudo systemctl status zyberlink-prover

# Ve logs
sudo journalctl -u zyberlink-prover -f
```

## Monitoreo

### Verifica Estado del Prover

```bash
# Vía systemd
sudo systemctl status zyberlink-prover

# Ve logs recientes
sudo journalctl -u zyberlink-prover -n 50

# Verifica balance de la wallet
solana balance ~/.config/solana/prover.json
```

### Modo TUI (Terminal UI)

Para monitoreo en tiempo real con interfaz visual:

```bash
./target/release/zyberlink-prover --tui-mode \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --witness-backend-url $WITNESS_BACKEND_URL
```

El TUI muestra:
- Trabajos reclamados, completados, fallidos
- Ganancias (SOL)
- Historial de trabajos recientes
- Tiempo de actividad del sistema

Presiona `q` para salir.

## Solución de Problemas

### El Prover No Inicia

**Verifica logs en busca de errores:**
```bash
./target/release/zyberlink-prover --program-id $ZYBERLINK_PROGRAM_ID
# Busca mensajes de error
```

**Problemas comunes:**
- Falta `ZYBERLINK_PROGRAM_ID`
- Ruta de keypair inválida
- Balance SOL insuficiente
- No puede conectar a RPC

### No se Reclaman Trabajos

**Posibles razones:**
1. No hay trabajos disponibles en la red
2. Otros provers reclaman trabajos primero
3. Tu umbral de ROI es demasiado alto
4. No estás registrado como prover

**Soluciones:**
```bash
# Baja el umbral de ROI
--min-roi 10.0

# Verifica registro
solana account <PROVER_PDA>

# Verifica que tienes SOL para fees de transacción
solana balance ~/.config/solana/prover.json
```

### La Descarga de Witness Falla

**Error:** "Failed to download witness from backend"

**Verifica el backend de witness:**
```bash
# Prueba conectividad del backend
curl $WITNESS_BACKEND_URL/health

# Verifica que la URL sea correcta
echo $WITNESS_BACKEND_URL
```

### Errores de Computación FHE

**Error:** "Failed to deserialize server key" o "FHE computation failed"

**Posibles causas:**
- Datos de witness corruptos
- Memoria insuficiente (se necesitan 8GB+)
- Discrepancia de versión TFHE

**Intenta:**
```bash
# Verifica memoria disponible
free -h

# Reduce trabajos concurrentes
--max-concurrent-jobs 1

# Reinicia prover
sudo systemctl restart zyberlink-prover
```

## Limitaciones Actuales

La implementación actual del prover es un prototipo temprano con las siguientes limitaciones:

**Limitaciones conocidas:**
- Selección básica de trabajos (sin optimización avanzada de rentabilidad)
- Mecanismo simple de consenso (2-de-3 o 3-de-5)
- Recuperación limitada de errores
- Sin reinicio automático en fallos
- Logging y monitoreo básico

**En Desarrollo:**
- Cálculo avanzado de ROI
- Aceleración GPU para operaciones FHE
- Sistema de reputación de provers
- Failover automático y recuperación
- Dashboard de monitoreo mejorado

**Recomendado para:** Pruebas, desarrollo y early adopters dispuestos a ejecutar software experimental.

## Mejores Prácticas

### Seguridad

```bash
# Asegura tu keypair
chmod 600 ~/.config/solana/prover.json

# Respalda keypair offline
cp ~/.config/solana/prover.json /media/usb/backup/

# No expongas puertos RPC públicamente
# Usa firewall para restringir acceso
```

### Confiabilidad

```bash
# Ejecuta como servicio systemd (auto-reinicio)
sudo systemctl enable zyberlink-prover

# Monitorea logs regularmente
sudo journalctl -u zyberlink-prover --since "1 hour ago"

# Mantén balance SOL por encima de 0.05
# (para fees de transacción)
```

### Rendimiento

```bash
# Ajusta trabajos concurrentes basado en RAM
# 8GB RAM = 1-2 trabajos
# 16GB RAM = 2-3 trabajos

# Usa RPC local si es posible (menor latencia)
# Usa almacenamiento SSD (descargas de witness más rápidas)
```

## Para Desarrollo

### Configuración de Pruebas Local

```bash
# Inicia validador local
make c1

# Inicializa marketplace
make c2

# Inicia prover en modo desarrollo
RUST_LOG=debug ./target/release/zyberlink-prover \
  --rpc-url http://localhost:8899 \
  --program-id <LOCAL_PROGRAM_ID> \
  --witness-backend-url http://localhost:8080
```

### Ejecutar Pruebas E2E

```bash
# Prueba flujo de Proof of Innocence
make e2e-poi

# Prueba flujo de Census Sum
make e2e-sum
```

## Próximos Pasos

- **Documentación Completa del Prover** - Ver `zyb-compute/README.md` (via el [Mapa de Repositorios](../primeros-pasos/repositorios.md))
- **[Guía de Analytics](analytics-privado.md)** - Ve cómo lucen los trabajos
- **[Guía de Proof of Innocence](proof-of-innocence.md)** - Otro caso de uso

## Obtener Ayuda

Para preguntas sobre configuración del prover:

- **GitHub Issues:** ver el [Mapa de Repositorios](../primeros-pasos/repositorios.md)
- **Documentación:** https://docs.zyberlink.fun
- **Discord:** [Únete a la comunidad]

## Documentación Relacionada

- `zyb-compute/README.md` - Documentación técnica completa
- `zyb-compute/src/roi_calculator.rs` - Lógica de rentabilidad
- `zyb-compute/src/fhe_engine.rs` - Implementación de computación

---

**Ejecuta un Prover:** Procesa computaciones encriptadas. Gana SOL. Apoya la computación que preserva privacidad.
