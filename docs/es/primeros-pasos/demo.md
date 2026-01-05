# Demo Interactiva

Experimenta el consenso multi-prover de ZyberLink en acción.

## Resumen

La demo de ZyberLink muestra un flujo completo de computación FHE multi-prover:

1. **Cliente** crea trabajos de computación encriptados
2. **Múltiples provers** compiten para procesar trabajos
3. **Consenso** asegura corrección
4. **Pagos** se distribuyen automáticamente

**Duración:** A tu propio ritmo (típicamente 5-10 minutos)

## Prerequisitos

Completa la guía de [Inicio Rápido](inicio-rapido.md) primero para tener:
- Validador local de Solana corriendo
- Programa ZyberLink desplegado
- Proyecto compilado exitosamente

## Ejecutar la Demo

### Opción 1: Demo Automatizada (Recomendada)

La forma más fácil de ver todo en acción:

```bash
cd zyb-platform
./demo.sh
```

Este script automáticamente:
- Configura el entorno
- Inicia 3 nodos prover
- Lanza la interfaz TUI
- Comienza a generar trabajos FHE
- Muestra procesamiento en tiempo real

**Lo que verás:**

```
┌─────────────────────────────────────────────────────────────┐
│ Nodo Prover ZyberLink - Demo Multi-Prover                  │
├─────────────────────────────────────────────────────────────┤
│ Estado: ACTIVO      Trabajos: 12     Ganancias: 0.024 SOL  │
│                                                             │
│ Trabajos Recientes:                                         │
│  Trabajo #234abc  Reclamado   Computando...  [████░░░░░] 45%│
│  Trabajo #233def  Completo    Verificado ✓   Ganado: 0.002 │
│  Trabajo #232ghi  Completo    Verificado ✓   Ganado: 0.002 │
│                                                             │
│ Red de Provers:                                             │
│  Prover A (Tú):    12 trabajos    97% uptime   ⭐ 4.9      │
│  Prover B:         11 trabajos    98% uptime   ⭐ 5.0      │
│  Prover C:         12 trabajos    95% uptime   ⭐ 4.8      │
└─────────────────────────────────────────────────────────────┘
Presiona 'q' para salir
```

### Opción 2: Control Manual

Para más control, ejecuta los componentes por separado:

#### Terminal 1: Iniciar Nodo Prover

```bash
cd prover-node
cargo run --release
```

El TUI se lanzará mostrando tu prover esperando por trabajos.

#### Terminal 2: Generar Trabajos

En una nueva terminal:

```bash
cd demo
./generate-jobs.sh
```

Esto crea trabajos de computación FHE y los publica en el marketplace.

#### Terminal 3: Monitorear Blockchain (Opcional)

Observa la actividad on-chain:

```bash
solana logs
```

Verás transacciones para:
- Creación de trabajos
- Reclamaciones de provers
- Envío de resultados
- Distribución de pagos

## Entendiendo la Demo

### Qué Está Pasando

1. **Creación de Trabajos**
   - El script de demo genera solicitudes de computación FHE encriptadas
   - Los trabajos se publican en el marketplace de Solana
   - Cada trabajo incluye datos de witness encriptados

2. **Competencia de Provers**
   - Múltiples nodos prover monitorean por nuevos trabajos
   - Los provers reclaman trabajos que pueden manejar
   - Los reclamadores más rápidos obtienen el trabajo

3. **Computación FHE**
   - Los provers ejecutan computaciones usando TFHE-rs
   - Las operaciones corren en datos encriptados
   - Los resultados permanecen encriptados hasta la finalización

4. **Verificación de Consenso**
   - Múltiples provers envían resultados
   - El programa on-chain verifica consenso 2-de-3
   - Los resultados coincidentes son aceptados
   - Los provers deshonestos son penalizados

5. **Distribución de Pagos**
   - Los provers honestos reciben pago automáticamente
   - Las puntuaciones de reputación se actualizan
   - El trabajo se marca como completado

### Interfaz TUI Explicada

La Interfaz de Usuario de Terminal muestra:

**Encabezado**
- `Estado`: IDLE (esperando), ACTIVE (procesando), o ERROR
- `Trabajos`: Total de trabajos procesados desde el inicio
- `Ganancias`: SOL ganados de trabajos completados

**Sección de Trabajos Recientes**
- ID del trabajo y estado actual
- Barras de progreso para computaciones en curso
- Estado de finalización y ganancias por trabajo

**Sección de Red de Provers**
- Todos los provers activos en la red
- Sus estadísticas: trabajos completados, uptime, reputación
- Muestra tu posición en la red

**Controles**
- `q` - Salir elegantemente
- `r` - Actualizar pantalla
- `h` - Mostrar ayuda

## Escenarios de Demo

### Escenario 1: Suma Simple

Operación FHE más simple - sumar dos números encriptados:

```bash
./demo/scenarios/single-addition.sh
```

**Salida esperada:**
- Trabajo publicado en ~1 segundo
- 3 provers reclaman el trabajo
- La computación completa en ~3 segundos
- Se alcanza consenso (acuerdo 3/3)
- Pago distribuido

### Escenario 2: Operaciones por Lotes

Múltiples operaciones FHE en paralelo:

```bash
./demo/scenarios/batch-operations.sh
```

**Salida esperada:**
- 10 trabajos publicados simultáneamente
- Los provers distribuyen el trabajo automáticamente
- Los trabajos completan en ~5-10 segundos
- Demostración de alto rendimiento

### Escenario 3: Prover Deshonesto

Simula un prover malicioso enviando resultados incorrectos:

```bash
./demo/scenarios/dishonest-prover.sh
```

**Salida esperada:**
- Un prover envía resultado incorrecto
- El consenso detecta desajuste
- El prover deshonesto NO es pagado
- Los provers honestos dividen la recompensa
- El sistema de reputación penaliza al mal actor

## Solución de Problemas

### El TUI no aparece

Verifica que el nodo prover esté corriendo:
```bash
ps aux | grep prover-node
```

### Los trabajos no están siendo reclamados

Verifica que el prover esté registrado:
```bash
solana account <YOUR_PROVER_PUBKEY>
```

### "No hay trabajos disponibles"

Asegúrate de que el generador de trabajos esté corriendo:
```bash
./demo/generate-jobs.sh
```

### Fallos de consenso

Verifica que tengas al menos 2 provers corriendo para consenso 2-de-3:
```bash
./demo/run-demo.sh  # Inicia 3 provers automáticamente
```

## Qué Sigue

Ahora que has visto ZyberLink en acción:

- **[Visión General de Arquitectura](../arquitectura/vision-general.md)** - Entiende el diseño técnico
- **[Diseño FHE](../arquitectura/diseno-fhe.md)** - Análisis profundo de encriptación
- **[Guía de Despliegue](../guias/despliegue.md)** - Ejecutar en producción

---

Disfruta explorando ZyberLink! La demo muestra la propuesta de valor central: **computación descentralizada, verificable y que preserva privacidad a escala**.
