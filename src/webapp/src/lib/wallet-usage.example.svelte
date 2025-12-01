<!--
  EJEMPLO: Cómo usar ZyberLink Wallet en componentes

  Este archivo muestra los patrones de uso del wallet store multi-chain.
  No es un componente funcional, es documentación con código.
-->

<script>
  import {
    walletStore,
    isConnected,
    activeChain,
    activeAddress,
    allAddresses,
    CHAIN_CONFIG,
    SUPPORTED_CHAINS
  } from './stores/wallet';

  // ====================================
  // EJEMPLO 1: Verificar conexión
  // ====================================

  // Usando el store directamente
  $: connected = $walletStore.connected;
  $: currentChain = $walletStore.activeChain;
  $: currentAddress = $walletStore.addresses[$walletStore.activeChain];

  // O usando derived stores (más limpio)
  // $isConnected, $activeChain, $activeAddress ya están disponibles


  // ====================================
  // EJEMPLO 2: Conectar wallet
  // ====================================

  async function connectSolana() {
    try {
      await walletStore.connect('solana');
      console.log('Connected to Solana:', $walletStore.addresses.solana);
    } catch (error) {
      console.error('Failed:', error.message);
    }
  }

  async function connectAllChains() {
    try {
      const addresses = await walletStore.connectAll();
      console.log('All addresses:', addresses);
    } catch (error) {
      console.error('Failed:', error.message);
    }
  }


  // ====================================
  // EJEMPLO 3: Firmar mensaje
  // ====================================

  async function signMessage() {
    if (!$isConnected) {
      alert('Connect wallet first');
      return;
    }

    try {
      const message = 'Hello ZyberLink!';
      const result = await walletStore.signMessage(message);
      console.log('Signature:', result.signature);
    } catch (error) {
      console.error('Sign failed:', error.message);
    }
  }


  // ====================================
  // EJEMPLO 4: Enviar transacción
  // ====================================

  async function sendTransaction() {
    if (!$isConnected) {
      alert('Connect wallet first');
      return;
    }

    try {
      // La transacción depende de la chain activa
      const tx = {
        // ... datos de la transacción
      };

      const result = await walletStore.signAndSendTransaction(tx);
      console.log('TX signature:', result.signature);
    } catch (error) {
      console.error('TX failed:', error.message);
    }
  }


  // ====================================
  // EJEMPLO 5: Cambiar chain activa
  // ====================================

  function switchToZcash() {
    walletStore.setActiveChain('zcash');
  }


  // ====================================
  // EJEMPLO 6: Obtener dirección específica
  // ====================================

  function getZcashAddress() {
    const zcashAddr = $walletStore.addresses.zcash;
    if (zcashAddr) {
      console.log('Zcash address:', zcashAddr);
    } else {
      console.log('Not connected to Zcash');
    }
  }


  // ====================================
  // EJEMPLO 7: Usar en CreateJob (pago)
  // ====================================

  async function createJobWithPayment(jobData) {
    if (!$isConnected) {
      throw new Error('Wallet not connected');
    }

    // 1. Obtener dirección según chain de pago
    const paymentChain = jobData.paymentChain || 'solana';
    const payerAddress = $walletStore.addresses[paymentChain];

    if (!payerAddress) {
      // Conectar a esa chain específica
      await walletStore.connect(paymentChain);
    }

    // 2. Cambiar a esa chain si no es la activa
    if ($walletStore.activeChain !== paymentChain) {
      walletStore.setActiveChain(paymentChain);
    }

    // 3. Crear y firmar transacción de pago
    const paymentTx = {
      to: jobData.escrowAddress,
      amount: jobData.price,
      // ... otros datos
    };

    const result = await walletStore.signAndSendTransaction(paymentTx);

    // 4. Crear job con el txid como prueba de pago
    return {
      ...jobData,
      paymentTxId: result.signature,
      payerAddress
    };
  }
</script>

<!--
  EJEMPLO DE UI: Mostrar estado del wallet
-->

{#if $isConnected}
  <div class="wallet-status">
    <div class="active-chain">
      <span style="color: {CHAIN_CONFIG[$activeChain].color}">
        {CHAIN_CONFIG[$activeChain].icon}
      </span>
      {CHAIN_CONFIG[$activeChain].name}
    </div>

    <div class="address">
      {$activeAddress?.slice(0, 8)}...{$activeAddress?.slice(-6)}
    </div>

    <!-- Mostrar todas las direcciones conectadas -->
    <div class="all-chains">
      {#each $walletStore.connectedChains as chain}
        <button
          class:active={chain === $activeChain}
          on:click={() => walletStore.setActiveChain(chain)}
        >
          {CHAIN_CONFIG[chain].icon} {CHAIN_CONFIG[chain].symbol}
        </button>
      {/each}
    </div>
  </div>
{:else}
  <button on:click={connectAllChains}>
    Connect ZyberLink
  </button>
{/if}

<!--
  EJEMPLO: Selector de chain para pagos
-->

<div class="payment-selector">
  <label>Pay with:</label>
  {#each SUPPORTED_CHAINS as chain}
    {@const config = CHAIN_CONFIG[chain]}
    {@const hasAddress = $walletStore.addresses[chain]}
    <button
      class="chain-option"
      class:available={hasAddress}
      disabled={!hasAddress}
      on:click={() => walletStore.setActiveChain(chain)}
    >
      <span style="color: {config.color}">{config.icon}</span>
      {config.symbol}
      {#if !hasAddress}
        (not connected)
      {/if}
    </button>
  {/each}
</div>

<style>
  /* Estilos de ejemplo */
  .wallet-status {
    padding: 1rem;
    border: 1px solid #333;
  }

  .all-chains {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .all-chains button.active {
    border-color: #00ff9f;
  }

  .payment-selector {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .chain-option:disabled {
    opacity: 0.5;
  }
</style>
