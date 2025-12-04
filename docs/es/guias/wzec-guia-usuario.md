# Guía de Usuario de Pagos wZEC

Una guía completa para usar wrapped Zcash (wZEC) para pagos enfocados en privacidad en el marketplace ZyberLink.

## Tabla de Contenidos

- [¿Qué es wZEC?](#qué-es-wzec)
- [¿Por qué usar wZEC?](#por-qué-usar-wzec)
- [Comenzando](#comenzando)
- [Haciendo tu primer pago wZEC](#haciendo-tu-primer-pago-wzec)
- [Entendiendo las Cuentas de Tokens](#entendiendo-las-cuentas-de-tokens)
- [Costos de Transacción](#costos-de-transacción)
- [Solución de Problemas](#solución-de-problemas)
- [Preguntas Frecuentes](#preguntas-frecuentes)

## ¿Qué es wZEC?

**wZEC (Wrapped Zcash)** es un token SPL en Solana que representa Zcash (ZEC), la criptomoneda enfocada en privacidad. Cada token wZEC está respaldado 1:1 por ZEC real mantenido en reserva.

### Propiedades Clave

- **Dirección del Token Mint**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
- **Decimales**: 8 (1 wZEC = 100,000,000 zatoshis)
- **Estándar**: SPL Token (Solana Program Library)
- **Bridge**: Solución de custodia multi-firma

## ¿Por qué usar wZEC?

### Beneficios

1. **Alineación con Privacidad**: Las características de privacidad de Zcash se alinean con la misión de computación confidencial de ZyberLink
2. **Pago Diversificado**: Alternativa a SOL para usuarios que poseen Zcash
3. **Integración Cross-Chain**: Habilita participación del ecosistema Zcash en computaciones FHE basadas en Solana
4. **Preparado para el Futuro**: Prepara infraestructura para pagos protegidos y características avanzadas de privacidad

### Trade-offs

| Característica | Pago SOL | Pago wZEC |
|----------------|----------|-----------|
| Velocidad de Transacción | Instantánea | Instantánea |
| Configuración Requerida | Ninguna | Cuenta de tokens necesaria |
| Privacidad | Estándar | Mejorada (respaldada por Zcash) |
| Comisiones | Más bajas | Ligeramente más altas (transferencia de tokens) |
| Disponibilidad | Nativa | Requiere compra de tokens |

## Comenzando

### Prerrequisitos

1. **Wallet Solana**: Phantom, Solflare, o cualquier wallet compatible con Solana
2. **Tokens wZEC**: Comprar desde exchanges soportados
3. **SOL para Comisiones**: ~0.01 SOL para comisiones de transacción

### Adquiriendo wZEC

**Opción 1: Swap en DEX**
```bash
# Usar Raydium, Orca, o Jupiter para intercambiar SOL por wZEC
# Buscar token: 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
```

**Opción 2: Bridge desde Zcash**
```bash
# Usar el bridge oficial wZEC (si está disponible)
# Depositar ZEC → Recibir wZEC en Solana
```

**Opción 3: Exchange Centralizado**
```bash
# Algunos exchanges pueden ofrecer retiros directos de wZEC a Solana
# Revisar redes soportadas al retirar
```

### Verificando tu Balance

```bash
# Usando Solana CLI
solana balance --token 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf

# Usando SPL Token CLI
spl-token balance 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
```

## Haciendo tu Primer Pago wZEC

### Paso 1: Acceder al Marketplace ZyberLink

Navegar a la aplicación web ZyberLink:
```
https://marketplace.zyberlink.io
```

### Paso 2: Conectar tu Wallet

1. Hacer clic en **"Conectar Wallet"** en la parte superior derecha
2. Seleccionar tu proveedor de wallet (Phantom, Solflare, etc.)
3. Aprobar la solicitud de conexión

### Paso 3: Crear un Job

1. Hacer clic en **"Crear Nuevo Job"**
2. Llenar los parámetros del job:
   - **Operación**: Elegir tipo de computación (Add, Multiply, etc.)
   - **Datos Encriptados**: Subir tu entrada encriptada con FHE
   - **Server Key**: Proporcionar la server key TFHE
   - **Precio**: Establecer monto de pago (en zatoshis)

### Paso 4: Seleccionar Método de Pago

**¡Aquí es donde entra wZEC!**

```
┌─────────────────────────────────────────────────────────┐
│  PAYMENT_METHOD: wZEC_SELECTED                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ○ SOL                          ● wZEC                 │
│    Solana nativa                  Wrapped Zcash        │
│    Rápido y bajas comisiones      Pagos privados       │
│    [RECOMENDADO]                                        │
│                                                         │
│  💡 PAGANDO_CON_TOKEN_wZEC                             │
│  Requiere cuenta de token wZEC. El sistema verificará  │
│  y creará si es necesario.                              │
└─────────────────────────────────────────────────────────┘
```

- Seleccionar el botón de radio **wZEC**
- El sistema verifica automáticamente si tienes una cuenta de tokens
- Si falta, se creará durante la transacción

### Paso 5: Revisar Transacción

```
┌─────────────────────────────────────────────────────────┐
│  Resumen de Transacción                                 │
├─────────────────────────────────────────────────────────┤
│  Método de Pago:  wZEC                                  │
│  Monto:           5 wZEC (500,000,000 zatoshis)        │
│  Comisión Red:    ~0.001 SOL                           │
│  Destinatario:    Escrow PDA (liberación automática)   │
│                                                         │
│  Cuentas:                                               │
│    - Tu wallet (firmante)                               │
│    - Tu cuenta de token wZEC (débito)                  │
│    - Cuenta de escrow (crédito)                        │
│    - Job PDA (metadata del job)                        │
└─────────────────────────────────────────────────────────┘
```

### Paso 6: Firmar y Confirmar

1. Hacer clic en **"Crear Job"**
2. Tu wallet solicitará aprobación
3. Revisar cuidadosamente los detalles de la transacción
4. Hacer clic en **"Aprobar"** en tu wallet
5. Esperar confirmación (~400ms en Solana)

### Paso 7: Rastrear tu Job

```
┌─────────────────────────────────────────────────────────┐
│  Job #12345 - Estado: PENDIENTE                        │
├─────────────────────────────────────────────────────────┤
│  Pago:        ✅ 5 wZEC bloqueado en escrow            │
│  Provers:     ⏳ 1/3 reclamados                        │
│  Consenso:    ⏳ Esperando resultados                  │
│  Estimado:    ~30 segundos                              │
└─────────────────────────────────────────────────────────┘
```

Una vez que se alcanza el consenso, wZEC se distribuye automáticamente a los provers, ¡y recibes tu resultado encriptado!

## Entendiendo las Cuentas de Tokens

### ¿Qué es una Cuenta de Tokens?

En el sistema de tokens SPL de Solana, cada usuario necesita una **Associated Token Account (ATA)** dedicada para cada tipo de token que posee.

```
┌───────────────────────────────────────────────────────────┐
│  Tu Wallet                                               │
│  HxL4...7Zf (clave pública base)                        │
│                                                           │
│  ├── Balance SOL: 2.5 SOL (nativo)                      │
│  │                                                        │
│  ├── Cuenta Token wZEC: 8kJ2...3mP                      │
│  │   └── Balance: 10 wZEC                               │
│  │                                                        │
│  └── Otras Cuentas de Tokens: ...                       │
└───────────────────────────────────────────────────────────┘
```

### Creación Automática

**Buenas noticias:** ¡ZyberLink maneja automáticamente la creación de cuentas de tokens!

**Cuando pagas con wZEC:**
1. El sistema verifica si tienes una cuenta de tokens wZEC
2. Si falta, incluye una instrucción de creación en la transacción
3. El rent de cuenta (~0.002 SOL) se deduce de tu balance SOL
4. El pago procede normalmente

**Creación Manual (Opcional):**
```bash
# Crear cuenta de tokens wZEC manualmente
spl-token create-account 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf

# Verificar dirección de tu cuenta de tokens
spl-token accounts
```

### Rent de Cuenta

Las cuentas de tokens requieren balance exento de rent:
- **Costo**: ~0.002 SOL (una vez, reembolsable si se cierra la cuenta)
- **Propósito**: Previene spam al requerir stake
- **Reembolso**: Cerrar cuenta para reclamar rent

## Costos de Transacción

### Desglose

```
Pago con wZEC:
├── Precio del Job:      5.0 wZEC  (establecido por ti)
├── Comisión de Red:     ~0.001 SOL (transacción Solana)
├── Rent Cuenta Token:   ~0.002 SOL (una vez, si es necesario)
└── Comisión Plataforma: 10% del precio del job (0.5 wZEC)

Costo Total:
  - 5.0 wZEC (de tu balance wZEC)
  - ~0.003 SOL (de tu balance SOL para comisiones)
```

### Comparación de Costos

| Escenario | Pago SOL | Pago wZEC |
|-----------|----------|-----------|
| Precio Job | 5 SOL | 5 wZEC |
| Comisión Red | ~0.0005 SOL | ~0.001 SOL |
| Cuenta Token | N/A | ~0.002 SOL (primera vez) |
| Comisión Plataforma | 0.5 SOL (10%) | 0.5 wZEC (10%) |
| **Total** | **5.0005 SOL** | **5 wZEC + 0.003 SOL** |

## Solución de Problemas

### Problema: "Balance wZEC Insuficiente"

**Problema:** La transacción falla con error de fondos insuficientes.

**Soluciones:**
1. Verificar tu balance wZEC:
   ```bash
   spl-token balance 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
   ```
2. Verificar que la cuenta de tokens existe:
   ```bash
   spl-token accounts
   ```
3. Comprar más wZEC desde un DEX o exchange

### Problema: "Falló Creación de Cuenta de Tokens"

**Problema:** El sistema no puede crear tu cuenta de tokens wZEC.

**Soluciones:**
1. Asegurar que tienes al menos 0.005 SOL para rent + comisiones
2. Verificar permisos de wallet (aprobar creación de cuenta de tokens)
3. Crear cuenta manualmente:
   ```bash
   spl-token create-account 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
   ```

### Problema: "Timeout de Transacción"

**Problema:** La transacción no se confirma en el tiempo esperado.

**Soluciones:**
1. Verificar estado de red Solana: https://status.solana.com
2. Aumentar comisión de prioridad de transacción (usuarios avanzados)
3. Reintentar transacción después de 30 segundos
4. Cambiar a pago SOL si es urgente

### Problema: "Token Mint Incorrecto"

**Problema:** Enviaste tokens a la dirección incorrecta.

**Soluciones:**
1. **Verificar dirección del mint**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
2. **Verificar en Solana Explorer**: Buscar el mint y confirmar que es wZEC
3. **No usar tokens aleatorios**: Solo se acepta wZEC oficial

### Problema: "Escrow No Libera Fondos"

**Problema:** El job se completó pero los provers no recibieron pago.

**Soluciones:**
1. Verificar estado del job on-chain
2. Verificar que se alcanzó consenso (2 de 3 provers coincidieron)
3. Contactar soporte con ID del job
4. Verificar firma de transacción en Solana Explorer

## Preguntas Frecuentes

### P: ¿Es el pago wZEC más privado que SOL?

**R:** En Solana, todas las transacciones son públicas independientemente del tipo de token. Sin embargo, wZEC representa Zcash, que tiene características fuertes de privacidad. Integraciones futuras pueden aprovechar los pools protegidos de Zcash para privacidad mejorada.

### P: ¿Puedo recuperar mi wZEC si cancelo un job?

**R:** ¡Sí! Si cancelas un job pendiente (antes de que los provers lo reclamen), tu wZEC se devuelve desde el escrow a tu cuenta de tokens menos las comisiones de red.

### P: ¿Qué pasa si los provers no están de acuerdo en el resultado?

**R:** El mecanismo de consenso requiere 2 de 3 (o threshold configurado) resultados coincidentes. Si el consenso falla:
1. El job se marca como fallido
2. Tu wZEC se reembolsa desde el escrow
3. Los provers deshonestos pueden ser penalizados

### P: ¿Puedo pagar parcialmente en SOL y parcialmente en wZEC?

**R:** Actualmente no. Cada job debe usar un único método de pago. Sin embargo, puedes crear múltiples jobs con diferentes métodos de pago.

### P: ¿Cómo convierto wZEC de vuelta a ZEC?

**R:** Usar el bridge oficial wZEC para desenvolver tus tokens:
1. Enviar wZEC al contrato del bridge
2. Proporcionar tu dirección receptora ZEC
3. Esperar confirmación del bridge (varía según el bridge)
4. Recibir ZEC en tu wallet Zcash

### P: ¿Hay un monto mínimo de wZEC para jobs?

**R:** No hay mínimo estricto, pero considera:
- Las comisiones de red (~0.001 SOL) no escalan con el tamaño del pago
- Jobs muy pequeños pueden no atraer provers
- Mínimo recomendado: 0.1 wZEC (~$5-10)

### P: ¿Puedo usar wZEC de testnet?

**R:** ¡Sí! Para testing:
- **Mint Devnet**: Usar faucet para obtener wZEC de prueba
- **Explorer testnet**: https://explorer.solana.com?cluster=devnet
- **Sin valor real**: Los tokens de testnet no tienen valor de mercado

### P: ¿Qué pasa si la dirección del mint wZEC cambia?

**R:** La dirección del mint está fija en el protocolo. Si cambia:
1. Los administradores del sistema anunciarán la migración
2. La documentación se actualizará
3. Puede ser necesario intercambiar tokens antiguos
4. Seguir canales oficiales para actualizaciones

## Próximos Pasos

- **[Guía de Desarrollador wZEC](wzec-guia-desarrollador.md)** - Integrar pagos wZEC en tu aplicación
- **[Referencia API wZEC](wzec-referencia-api.md)** - Documentación completa de API
- **[Arquitectura wZEC](../arquitectura/wzec-arquitectura.md)** - Inmersión técnica profunda
- **[Guía de Testing](wzec-guia-testing.md)** - Ejecutar tests E2E localmente

## Soporte

¿Necesitas ayuda con pagos wZEC?

- **Documentación**: [docs.zyberlink.io](https://docs.zyberlink.io)
- **GitHub Issues**: [Reportar bugs](https://github.com/zyberlink/zyberlink/issues)
- **Discord**: [Unirse a la comunidad](https://discord.gg/zyberlink)
- **Email**: support@zyberlink.io

---

**Última Actualización**: 2025-11-21
**Versión**: 1.0.0
**Mint wZEC**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
