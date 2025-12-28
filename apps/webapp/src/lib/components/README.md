# ZyberLink Frontend Components

## Payment Components

### PaymentMethodSelector
Selector visual de métodos de pago para jobs FHE.

**Props:**
- `selected` (string): Método de pago actual ('sol' | 'wzec')
- `disabled` (boolean): Deshabilitar selector

**Events:**
- `change`: Se emite cuando cambia la selección
  - `event.detail.payment_method`: Nuevo método seleccionado

**Ejemplo:**
```svelte
<PaymentMethodSelector
  selected={jobData.paymentMethod}
  on:change={handlePaymentMethodChange}
/>
```

**Features:**
- Radio buttons visuales con estilo TUI
- Tooltips explicativos
- Animaciones suaves
- Accesibilidad WCAG AA
- Responsive design

---

## UX Components

### Toast
Sistema de notificaciones tipo toast.

**Props:**
- `message` (string): Mensaje a mostrar
- `type` (string): Tipo de notificación ('success' | 'error' | 'warning' | 'info')
- `duration` (number): Duración en ms (default: 3000)
- `onClose` (function): Callback al cerrar

**Ejemplo:**
```svelte
<Toast
  message="Job created successfully!"
  type="success"
  duration={5000}
/>
```

**Toast Store:**
```javascript
import { toastStore } from '$lib/stores/toast';

toastStore.show('Transaction confirmed!', 'success');
toastStore.error('Failed to create job');
toastStore.warning('Token account needed');
toastStore.info('Checking wallet...');
```

---

### Tooltip
Tooltips informativos con posicionamiento flexible.

**Props:**
- `text` (string): Texto del tooltip
- `position` (string): Posición ('top' | 'bottom' | 'left' | 'right')
- `maxWidth` (string): Ancho máximo (default: '200px')

**Ejemplo:**
```svelte
<Tooltip text="This is helpful info" position="top">
  <button>Hover me</button>
</Tooltip>
```

**Features:**
- Auto-posicionamiento
- Animaciones fade
- Estilo TUI coherente
- Accesible por teclado

---

### Loading
Indicador de carga con spinner animado.

**Props:**
- `message` (string): Mensaje de estado
- `size` (string): Tamaño del spinner ('sm' | 'md' | 'lg')
- `inline` (boolean): Mostrar en línea

**Ejemplo:**
```svelte
<Loading message="Creating job..." size="md" />

<!-- Inline -->
<Loading message="Processing" size="sm" inline />
```

---

## Utility Modules

### tokenAccountManager.js
Gestión de SPL Token Accounts para pagos wZEC.

**Funciones principales:**

#### checkTokenAccount
Verifica si existe una token account.
```javascript
const { exists, address } = await checkTokenAccount(
  connection,
  ownerPublicKey,
  mintPublicKey
);
```

#### ensureTokenAccount
Asegura que existe token account, crea si no existe.
```javascript
const tokenAddress = await ensureTokenAccount(
  connection,
  wallet,
  WZEC_MINT,
  (progress) => {
    console.log(progress.step, progress.message);
  }
);
```

#### hasWzecAccount
Helper rápido para verificar wZEC account.
```javascript
const hasAccount = await hasWzecAccount(connection, walletPublicKey);
```

**Constantes:**
- `WZEC_MINT`: PublicKey del mint de wZEC

---

## Mock API

### api.js (mocks)
Cliente de API mock para desarrollo independiente.

**Uso:**
```javascript
import { mockApi, WZEC_MINT } from '$lib/mocks/api';

// Crear job
const response = await mockApi.createJob({
  payment_method: 'wzec',
  price_lamports: 2000000,
  // ... otros campos
});

// Verificar token account
const accountStatus = await mockApi.checkTokenAccount(
  walletPubkey,
  WZEC_MINT
);
```

**Responses:**
- `mockCreateJobResponse`: Diferentes scenarios de creación
- `mockCheckTokenAccountResponse`: Estados de token account

---

## Styling Guidelines

Todos los componentes siguen el sistema de diseño TUI:

- **Colores:** Variables CSS de `tui-system.css`
- **Tipografía:** Monospace para elementos técnicos
- **Animaciones:** Transiciones suaves, respetan `prefers-reduced-motion`
- **Accesibilidad:** ARIA labels, keyboard navigation, color contrast

**Variables principales:**
```css
--zyber-quantum-violet: #8B5CF6
--zyber-cyber-cyan: #06B6D4
--zyber-success: #10B981
--zyber-error: #EF4444
--zyber-warning: #F59E0B
```

---

## Testing

Tests disponibles en `/tests`:
- `paymentSelector.test.js`: Tests del selector de pago
- `tokenAccountManager.test.js`: Tests del token manager

**Ejecutar tests:**
```bash
npm test
```

---

## Integration Checklist

Para integrar con backend real de Agent A:

- [ ] Reemplazar `mockApi` con llamadas fetch reales
- [ ] Configurar Connection con RPC endpoint correcto
- [ ] Integrar wallet adapter (@solana/wallet-adapter-react)
- [ ] Actualizar WZEC_MINT si difiere en mainnet
- [ ] Manejar errores de red apropiadamente
- [ ] Agregar retry logic para transacciones

---

## Next Steps

1. **Backend Integration:** Conectar con API de Agent A cuando esté listo
2. **E2E Testing:** Tests end-to-end con backend real
3. **Performance:** Optimizar re-renders y bundle size
4. **Features:** Modal de confirmación de token account creation
