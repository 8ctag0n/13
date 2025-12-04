<script lang="ts">
  import { onMount } from 'svelte';
  import QRCode from 'qrcode';

  export let value: string = '';
  export let size: number = 150;
  export let color: string = '#00ff9f';
  export let bgColor: string = '#0a0e14';

  let canvas: HTMLCanvasElement;
  let error = false;

  $: if (value && canvas) {
    generateQR();
  }

  async function generateQR() {
    if (!value || !canvas) return;

    try {
      error = false;
      await QRCode.toCanvas(canvas, value, {
        width: size,
        margin: 2,
        color: {
          dark: color,
          light: bgColor
        },
        errorCorrectionLevel: 'M'
      });
    } catch (e) {
      console.error('[QRCode] Generation failed:', e);
      error = true;
    }
  }

  onMount(() => {
    if (value) generateQR();
  });
</script>

<div class="qr-container" style="--size: {size}px; --color: {color}">
  {#if error}
    <div class="qr-error">
      <span class="error-icon">[!]</span>
      <span class="error-text">QR Error</span>
    </div>
  {:else if !value}
    <div class="qr-placeholder">
      <span class="placeholder-text">[NO DATA]</span>
    </div>
  {:else}
    <canvas bind:this={canvas} class="qr-canvas"></canvas>
  {/if}

  <div class="qr-corners">
    <span class="corner tl"></span>
    <span class="corner tr"></span>
    <span class="corner bl"></span>
    <span class="corner br"></span>
  </div>
</div>

<style>
  .qr-container {
    position: relative;
    width: var(--size);
    height: var(--size);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--cyber-bg);
  }

  .qr-canvas {
    display: block;
    image-rendering: pixelated;
  }

  .qr-placeholder,
  .qr-error {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--cyber-border);
    gap: 8px;
  }

  .qr-error {
    border-color: var(--cyber-error, #ff4444);
  }

  .error-icon {
    font-size: 24px;
    color: var(--cyber-error, #ff4444);
  }

  .error-text {
    font-size: 10px;
    color: var(--cyber-error, #ff4444);
  }

  .placeholder-text {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }

  /* Cyberpunk corner decorations */
  .qr-corners {
    position: absolute;
    inset: -4px;
    pointer-events: none;
  }

  .corner {
    position: absolute;
    width: 12px;
    height: 12px;
    border-color: var(--color);
    border-style: solid;
    border-width: 0;
  }

  .corner.tl {
    top: 0;
    left: 0;
    border-top-width: 2px;
    border-left-width: 2px;
  }

  .corner.tr {
    top: 0;
    right: 0;
    border-top-width: 2px;
    border-right-width: 2px;
  }

  .corner.bl {
    bottom: 0;
    left: 0;
    border-bottom-width: 2px;
    border-left-width: 2px;
  }

  .corner.br {
    bottom: 0;
    right: 0;
    border-bottom-width: 2px;
    border-right-width: 2px;
  }
</style>
