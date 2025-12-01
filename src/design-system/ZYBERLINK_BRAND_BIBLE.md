# ZyberLink - Brand Bible

**Todo lo que necesitas saber para representar ZyberLink**

---

## PARTE 1: EL PROYECTO

### Que es ZyberLink

**En una frase:**
Marketplace descentralizado de computacion criptografica que preserva privacidad.

**Definicion completa:**
ZyberLink es una red de nodos independientes ("provers") que compiten para ejecutar operaciones criptograficas intensivas (FHE y ZK) sobre datos encriptados, con verificacion on-chain y pagos automaticos en Solana.

### El Problema

```
Las apps modernas necesitan criptografia avanzada (FHE, ZK) pero:

1. MOVILES NO PUEDEN
   FHE toma minutos, drena bateria, imposible en smartphones

2. SERVIDORES CENTRALES = RIESGO
   Punto unico de falla, censurable, requiere confianza

3. SIN VERIFICACION
   No hay forma economica de garantizar que el resultado es correcto
```

### La Solucion

```
MULTI-PROVER CONSENSUS

1. Cliente encripta datos → nunca salen en texto plano
2. Crea JOB en Solana con reward
3. MULTIPLES provers reclaman y ejecutan INDEPENDIENTEMENTE
4. Consenso on-chain verifica acuerdo (2-de-3 o 3-de-5)
5. Pagos automaticos a provers honestos
6. Cliente recupera resultado (sigue encriptado)
```

### Arquitectura Visual

```
┌──────────────────────────────────────────────────────────────┐
│                         CLIENTES                             │
│          Mobile | Web | Backend | CLI                        │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│                    SDK (Rust/TypeScript)                     │
│              Encriptacion + Creacion de Jobs                 │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│               SOLANA MARKETPLACE PROGRAM                     │
│       Queue | Registry | Consensus | Escrow                  │
└─────────────────────────┬────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
   ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ PROVER A │    │ PROVER B │    │ PROVER C │
   │ FHE/ZK   │    │ FHE/ZK   │    │ FHE/ZK   │
   └──────────┘    └──────────┘    └──────────┘
```

---

## PARTE 2: EL ETHOS

### Filosofia Cypherpunk

> "Privacy is necessary for an open society in the electronic age."
> - Eric Hughes, 1993

**Principios que guian el desarrollo:**

1. **Code is law** - Reglas enforced por smart contracts
2. **Don't trust, verify** - Consenso multi-prover elimina confianza
3. **Privacy by default** - Datos encriptados end-to-end siempre
4. **Permissionless** - Cualquiera puede participar
5. **Open source** - Codigo auditable y verificable

### Valores Core

| Valor | Significado |
|-------|-------------|
| **Privacidad Absoluta** | Datos encriptados siempre, post-quantum ready |
| **Descentralizacion Real** | Sin servidores centrales, sin puntos de control |
| **Verificabilidad** | Consenso on-chain, codigo open source |
| **Accesibilidad** | Documentacion clara, SDKs intuitivos |
| **Innovacion Responsable** | Seguridad primero, iteracion constante |

### Resistencia a la Censura

En un mundo donde gobiernos bloquean, corporaciones deniegan acceso, e intermediarios discriminan:

- Red sin punto central de control
- Provers anonimos y distribuidos
- Pagos crypto sin intermediarios
- Datos que ni los provers pueden leer

---

## PARTE 3: CASOS DE USO

### DeFi Privado
```
"Quiero hacer un swap sin revelar mi balance"

→ App encripta balance con FHE
→ ZyberLink computa: balance >= monto?
→ Resultado: true/false (sin revelar balance exacto)
```

### Votacion DAO Confidencial
```
"Quiero votar sin que sepan mi voto hasta el conteo"

→ Usuario encripta voto (1=Si, 0=No)
→ ZyberLink agrega votos encriptados
→ Al cierre: se desencripta solo el TOTAL
→ Votos individuales permanecen privados
```

### Analytics Privado
```
"Quiero calcular promedios sin revelar datos individuales"

→ Dataset encriptado subido
→ ZyberLink computa sum(), count(), avg()
→ Solo resultado agregado se desencripta
```

### Proof of Innocence
```
"Quiero probar que NO estoy en lista negra sin revelar mi identidad"

→ Usuario genera ZK proof de no-pertenencia
→ ZyberLink ejecuta verificacion
→ Resultado: "No esta en lista" (identidad privada)
```

### Wallet Multi-Chain
```
"Quiero manejar Solana, Starknet y Zcash desde un lugar"

→ Extension de browser unificada
→ Transacciones privadas cross-chain
→ wZEC bridging con privacidad
```

---

## PARTE 4: AUDIENCIA

### Desarrolladores de dApps
- Necesitan offloadear FHE/ZK de moviles
- Quieren privacidad sin sacrificar UX
- Valoran descentralizacion real

### Operadores de Provers
- Tienen hardware disponible
- Buscan ingresos pasivos en crypto
- Quieren contribuir a infraestructura privacy

### Usuarios Finales (indirectos)
- Usan apps que integran ZyberLink
- Se benefician de privacidad sin saberlo
- Experiencia transparente

---

## PARTE 5: PERSONALIDAD DE MARCA

### Arquetipo

**EL MAGO** (80%)
- Transforma lo imposible en realidad
- Tecnologia avanzada que parece magia
- Empowerment a traves de capacidades nuevas

**EL EXPLORADOR** (20%)
- Descubre territorios desconocidos
- Aventura en el futuro de la privacidad
- Libertad y autonomia

### Tono de Voz

**SOMOS:**
- Techno-cripto-punk: Descentralizacion sin compromiso
- Post-quantum ready: Mirando al futuro
- Privacy-first builders: Privacidad como derecho fundamental

**NO SOMOS:**
- Web2 corporativo
- Simplista o reductivo
- Conservador o tradicional

**HABLAMOS COMO:**
- Hacker etico en un terminal nocturno
- Cypherpunk con claridad didactica

### Ejemplos de Copy

```
EN VEZ DE: "Easy privacy solutions"
USAMOS: "Fully Homomorphic Encryption. Decentralized. Post-Quantum Ready."

EN VEZ DE: "Fast and secure"
USAMOS: "Multi-prover consensus. Sub-second finality. Byzantine fault tolerant."

EN VEZ DE: "Join us today!"
USAMOS: "Run a prover node. Earn rewards. Secure the network."
```

---

## PARTE 6: MENSAJES CLAVE

### Tagline Principal
**"Private Compute, Decentralized Trust"**

### Variantes por Contexto

**Para Desarrolladores:**
> "Offload FHE/ZK to a decentralized prover network. Build privacy-first apps without the infrastructure burden."

**Para Provers:**
> "Run a node, earn rewards, secure the network. Your hardware powers privacy."

**Para Usuarios Finales:**
> "Your data stays encrypted. Always. Even during computation."

### Elevator Pitch (30 segundos)

> "Las apps modernas necesitan criptografia avanzada para privacidad, pero es imposible correrla en moviles. ZyberLink es un marketplace donde multiples provers independientes compiten para ejecutar estas operaciones sobre datos encriptados. El consenso on-chain garantiza correctitud sin requerir confianza. Es como AWS Lambda, pero descentralizado, privado, y verificable."

---

## PARTE 7: IDENTIDAD VISUAL

### Concepto: "Encrypted Light"

Datos fluyen a traves de canales encriptados, iluminados por operaciones criptograficas, creando glows en la oscuridad de la privacidad.

### Paleta de Colores

```
PRIMARIOS
─────────────────────────────────────
Quantum Violet    #8B5CF6    Encriptacion, magia, quantum
Cyber Cyan        #06B6D4    Energia, blockchain, tech

SEMANTICOS
─────────────────────────────────────
Success           #10B981    Operaciones exitosas
Warning           #F59E0B    Alerts, pending
Error             #EF4444    Failures, critical

BACKGROUNDS (Dark Mode First)
─────────────────────────────────────
Base              #0A0A0F    Casi negro con tinte violeta
Secondary         #13131A    Fondo secundario
Tertiary          #1C1C26    Cards/panels
Elevated          #25253C    Modals/dropdowns

TEXTO
─────────────────────────────────────
Primary           #F9FAFB    Blanco puro
Secondary         #D1D5DB    Gris claro
Tertiary          #9CA3AF    Gris medio
```

### Tipografia

```
HEADINGS - Space Grotesk
─────────────────────────────────────
Geometrica moderna, tech-forward
Weight: 700 (Bold)
Uso: Titulos, hero text, CTAs

BODY - Inter
─────────────────────────────────────
Optimizada para UI, legibilidad superior
Weight: 400 (Regular)
Uso: Parrafos, labels, descripciones

CODE - JetBrains Mono
─────────────────────────────────────
Ligaduras, distincion clara de caracteres
Weight: 400 (Regular)
Uso: Code snippets, hashes, direcciones
```

### Logo

**Concepto: "Bloque Encriptado"**

```
╔═══════════════════════════════╗
║  ▓▓▓▓                         ║
║  ▓▓▓  ZyberLink               ║
║  ▓▓▓  ENCRYPTED COMPUTE       ║
╚═══════════════════════════════╝

Elementos:
- "Z" formada con bloques pixelados
- Gradient violeta → cyan
- Glow effect alrededor
- Marco glassmorphic
```

**Variantes necesarias:**
1. Logo completo (icon + text + tagline)
2. Solo icono (favicon, app icon)
3. Solo wordmark (horizontal)
4. Monocromatico (blanco, negro)

### Estilo Visual

**Glassmorphism First**
```css
background: rgba(28, 28, 38, 0.6);
backdrop-filter: blur(20px);
border: 1px solid rgba(139, 92, 246, 0.2);
```

**Glow over Shadow**
```css
box-shadow: 0 0 20px rgba(139, 92, 246, 0.4);
```

**Dark Mode Primary**
- Fondos oscuros siempre
- Glows para profundidad
- Colores vibrantes sobre negro

---

## PARTE 8: ASSETS A CREAR

### Prioridad ALTA (necesarios ya)

**Logos**
- [ ] `logo-horizontal-dark.svg` - Principal
- [ ] `logo-horizontal-light.svg` - Alternativo
- [ ] `logo-icon.svg` - Solo icono
- [ ] `logo-wordmark.svg` - Solo texto
- [ ] PNGs en 1200x300, 600x150, 512x512, 256x256, 128x128, 64x64

**Favicons**
- [ ] `favicon.ico` (16x16, 32x32, 48x48)
- [ ] `favicon-16x16.png`
- [ ] `favicon-32x32.png`
- [ ] `apple-touch-icon.png` (180x180)
- [ ] `android-chrome-192x192.png`
- [ ] `android-chrome-512x512.png`

**Social/OG**
- [ ] `og-image.png` (1200x630) - Para links compartidos
- [ ] `twitter-card.png` (1200x600)

### Prioridad MEDIA

**Extension Icons**
- [ ] `extension-icon-16.png`
- [ ] `extension-icon-32.png`
- [ ] `extension-icon-48.png`
- [ ] `extension-icon-128.png`

**Empty States**
- [ ] `empty-jobs.svg` - Dashboard sin jobs
- [ ] `empty-wallet.svg` - Wallet no conectada
- [ ] `empty-analytics.svg` - Sin datos

**Social Banners**
- [ ] Twitter header (1500x500)
- [ ] GitHub banner (1280x640)
- [ ] README banner (1200x400)

### Prioridad BAJA

**Ilustraciones**
- [ ] `illustration-fhe.svg` - Concepto FHE
- [ ] `illustration-consensus.svg` - Multi-prover network
- [ ] `illustration-privacy.svg` - Data privacy

**Error States**
- [ ] `error-404.svg`
- [ ] `error-generic.svg`

**Loading/Progress**
- [ ] `spinner.svg` (animated)
- [ ] `computing-bar.svg`

---

## PARTE 9: ESTRUCTURA DE ARCHIVOS

```
design-system/
├── ZYBERLINK_BRAND_BIBLE.md    # Este archivo
├── tokens.css                   # Variables CSS
├── components.css               # Componentes
├── demo.html                    # Demo interactivo
└── assets/
    ├── logos/
    │   ├── svg/
    │   │   ├── logo-horizontal-dark.svg
    │   │   ├── logo-horizontal-light.svg
    │   │   ├── logo-icon.svg
    │   │   └── logo-wordmark.svg
    │   └── png/
    │       └── [varios tamaños]
    ├── favicons/
    │   ├── favicon.ico
    │   ├── favicon-16x16.png
    │   └── ...
    ├── social/
    │   ├── og-image.png
    │   ├── twitter-card.png
    │   └── banners/
    ├── illustrations/
    │   └── [SVGs]
    └── icons/
        └── extension/
```

---

## PARTE 10: REFERENCIAS RAPIDAS

### Colores (copiar)
```
#8B5CF6  Violet (primary)
#06B6D4  Cyan (accent)
#10B981  Green (success)
#F59E0B  Amber (warning)
#EF4444  Red (error)
#0A0A0F  Background
#F9FAFB  Text
```

### Fonts (Google Fonts)
```html
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;700&family=Inter:wght@400;500;600&family=JetBrains+Mono&display=swap" rel="stylesheet">
```

### CSS Variables
```css
:root {
  --zyber-primary: #8B5CF6;
  --zyber-accent: #06B6D4;
  --zyber-success: #10B981;
  --zyber-warning: #F59E0B;
  --zyber-error: #EF4444;
  --zyber-bg: #0A0A0F;
  --zyber-text: #F9FAFB;
  --font-heading: 'Space Grotesk', sans-serif;
  --font-body: 'Inter', sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
}
```

### Gradient Principal
```css
background: linear-gradient(135deg, #8B5CF6 0%, #06B6D4 100%);
```

### Glow Effect
```css
box-shadow: 0 0 20px rgba(139, 92, 246, 0.4);
```

---

## RESUMEN EJECUTIVO

**ZyberLink es:**
- Marketplace descentralizado de FHE/ZK
- Multi-prover consensus para verificacion
- Privacy by default, post-quantum ready

**Representa:**
- Privacidad como derecho
- Descentralizacion real
- Innovacion cypherpunk

**Visualmente es:**
- Cyberpunk elegante
- Glassmorphism + glows
- Dark mode first
- Violeta + cyan sobre negro

**Habla como:**
- Tecnico pero accesible
- Confiado sin arrogancia
- Cypherpunk con claridad

---

**ZyberLink: Private Compute, Decentralized Trust.**
