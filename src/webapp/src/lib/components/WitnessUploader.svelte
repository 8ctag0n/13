<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let compact = false; // Modo compacto para páginas simplificadas

  // State
  let witnessFile = null;
  let witnessParseError = null;
  let isParsingWitness = false;
  let isDragging = false;

  // Parsed data
  let serverKey = null;
  let encryptedData = null;
  let fileName = null;

  /**
   * Convert Uint8Array to base64 using streaming chunks (memory efficient)
   * Processes in chunks of 3 bytes aligned (base64 encodes 3 bytes -> 4 chars)
   */
  async function arrayBufferToBase64Async(bytes, onProgress = null) {
    // Use 48KB chunks (divisible by 3 for clean base64 encoding)
    const chunkSize = 48 * 1024; // 48KB = 49152 bytes, divisible by 3
    const base64Chunks = [];
    const totalBytes = bytes.length;
    let processed = 0;

    for (let i = 0; i < totalBytes; i += chunkSize) {
      const end = Math.min(i + chunkSize, totalBytes);
      const chunk = bytes.subarray(i, end);

      // Convert chunk to binary string
      let binary = '';
      for (let j = 0; j < chunk.length; j++) {
        binary += String.fromCharCode(chunk[j]);
      }

      // Encode this chunk to base64 directly
      base64Chunks.push(btoa(binary));
      processed = end;

      // Yield to event loop every few chunks to keep UI responsive
      if (base64Chunks.length % 5 === 0) {
        if (onProgress) {
          onProgress(Math.round((processed / totalBytes) * 100));
        }
        await new Promise(r => setTimeout(r, 0));
      }
    }

    if (onProgress) {
      onProgress(100);
    }

    // Join base64 chunks (much smaller than raw binary)
    return base64Chunks.join('');
  }

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      handleWitnessUpload(files[0]);
    }
  }

  function handleFileInput(e) {
    const files = e.target.files;
    if (files.length > 0) {
      handleWitnessUpload(files[0]);
    }
  }

  /**
   * Parse witness.bin format:
   * [server_key_len (8 bytes LE)][server_key bytes][encrypted_data bytes]
   */
  async function handleWitnessUpload(file) {
    witnessFile = file;
    witnessParseError = null;
    fileName = file.name;
    isParsingWitness = true;

    // Reset parsed data
    serverKey = null;
    encryptedData = null;

    try {
      // Use setTimeout to let UI update before heavy processing
      await new Promise(r => setTimeout(r, 50));

      const buffer = await file.arrayBuffer();
      const view = new DataView(buffer);

      // Read server_key length (8 bytes, little-endian u64)
      const serverKeyLen = Number(view.getBigUint64(0, true));

      if (serverKeyLen <= 0 || serverKeyLen > buffer.byteLength - 8) {
        throw new Error(`Invalid server_key length: ${serverKeyLen}`);
      }

      // Extract server_key bytes - KEEP raw bytes for pre-upload
      const serverKeyBytes = new Uint8Array(buffer, 8, serverKeyLen);

      // Copy to a new Uint8Array to avoid issues with buffer views
      const serverKeyBytesCopy = new Uint8Array(serverKeyBytes);

      // Extract encrypted_data bytes (rest of the file)
      const encryptedDataStart = 8 + serverKeyLen;
      const encryptedDataBytes = new Uint8Array(buffer, encryptedDataStart);

      if (encryptedDataBytes.length === 0) {
        throw new Error('No encrypted data found in witness');
      }

      // We no longer need to encode server_key to base64 (will use pre-upload)
      // Just encode encrypted_data which is small (~KB)
      serverKey = null; // Will use serverKeyBytes instead

      // Small delay between large operations
      await new Promise(r => setTimeout(r, 10));

      encryptedData = await arrayBufferToBase64Async(encryptedDataBytes, (progress) => {
        console.log(`Encrypted data encoding: ${progress}%`);
      });

      console.log(`Parsed witness: server_key=${(serverKeyLen / 1024 / 1024).toFixed(1)}MB (raw bytes), encrypted_data=${(encryptedDataBytes.length / 1024).toFixed(1)}KB`);

      // Dispatch success event with parsed data
      // Include raw serverKeyBytes for pre-upload instead of base64
      dispatch('witnessParsed', {
        serverKeyBytes: serverKeyBytesCopy,  // Raw bytes for pre-upload
        serverKey: null,                      // Legacy base64 (no longer used)
        encryptedData,
        fileName: file.name,
        fileSize: file.size,
        serverKeySize: serverKeyLen,
        encryptedDataSize: encryptedDataBytes.length
      });

    } catch (error) {
      console.error('Failed to parse witness file:', error);
      witnessParseError = error.message || 'Invalid witness file format';
      serverKey = null;
      encryptedData = null;

      // Dispatch error event
      dispatch('witnessError', {
        error: witnessParseError,
        fileName: file.name
      });
    } finally {
      isParsingWitness = false;
    }
  }

  // Reset function (can be called from parent)
  export function reset() {
    witnessFile = null;
    witnessParseError = null;
    isParsingWitness = false;
    isDragging = false;
    serverKey = null;
    encryptedData = null;
    fileName = null;
  }

  // Track serverKeyBytes instead of serverKey
  let serverKeyBytesStored = null;

  // Expose parsed state - now based on encryptedData since serverKeyBytes is handled separately
  $: isParsed = encryptedData && !witnessParseError;
</script>

<div
  class="witness-uploader"
  class:compact
  class:dragging={isDragging}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if !compact}
    <div class="upload-help-header text-mono text-sm mb-4">
      <span class="text-cyan">[REQUIRED]</span> Upload the <strong>witness.bin</strong> file generated by zyb:
    </div>
  {/if}

  <!-- File Input -->
  <div class="file-input-wrapper" data-testid="witness-upload-wrapper">
    <label class="file-label" class:file-label-success={isParsed} class:file-label-error={witnessParseError}>
      <span class="label-header">
        <span class="label-title text-mono">WITNESS.BIN</span>
        <span class="label-format text-xs text-muted">Contains server key + encrypted data</span>
      </span>
      <input
        type="file"
        accept=".bin"
        on:change={handleFileInput}
        data-testid="witness-input"
        class="file-input"
        aria-label="Upload witness.bin file"
      />
      <span
        class="file-status text-mono text-sm"
        class:file-selected={isParsed}
        class:file-pending={!fileName}
        class:file-processing={isParsingWitness}
        class:file-error={witnessParseError}
        data-testid="witness-status"
        aria-live="polite"
      >
        {#if isParsingWitness}
          <span class="text-cyan">[...]</span> Processing witness file...
        {:else if witnessParseError}
          <span class="text-error">[!]</span> {witnessParseError}
        {:else if isParsed}
          <span class="text-success">[OK]</span> {fileName}
          <span class="text-muted text-xs">({witnessFile?.size ? (witnessFile.size / (1024 * 1024)).toFixed(1) : '0'} MB)</span>
        {:else}
          <span class="text-warning">[...]</span> Click or drop witness.bin here
        {/if}
      </span>
    </label>
  </div>

  <!-- Upload Status Indicator -->
  <div class="upload-status-container" data-testid="upload-status">
    {#if isParsingWitness}
      <div class="upload-processing-indicator text-mono text-sm text-cyan">
        <span>[~]</span> Processing file, please wait...
      </div>
    {:else if witnessParseError}
      <div class="upload-error-indicator text-mono text-sm text-error">
        <span>[X]</span> Invalid witness file. Generate it with: zyb fhe encrypt -p ./output
      </div>
    {:else if isParsed}
      <div class="upload-complete-indicator text-mono text-sm text-success">
        <span>[OK]</span> Witness parsed successfully. Ready to proceed.
      </div>
    {:else}
      <div class="upload-pending-indicator text-mono text-sm text-muted">
        <span>[...]</span> Upload witness.bin to continue
      </div>
    {/if}
  </div>
</div>

<style>
  .witness-uploader {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-6);
    transition: all var(--transition-base);
  }

  .witness-uploader.dragging {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
  }

  .witness-uploader.compact {
    padding: var(--space-4);
  }

  .upload-help-header {
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .file-input-wrapper {
    position: relative;
    margin-bottom: var(--space-4);
  }

  .file-label {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    background: rgba(100, 116, 139, 0.05);
    border: 2px solid var(--zyber-border-primary);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .file-label:hover {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
  }

  .file-label-success {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.05);
  }

  .file-label-error {
    border-color: var(--zyber-error);
    background: rgba(239, 68, 68, 0.05);
  }

  .label-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .label-title {
    font-weight: 600;
    color: var(--zyber-text-primary);
  }

  .label-format {
    padding: 2px 8px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: var(--radius-sm);
  }

  .file-input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .file-status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.3);
    min-height: 32px;
  }

  .file-status.file-selected {
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .file-status.file-pending {
    background: rgba(245, 158, 11, 0.05);
    border: 1px solid rgba(245, 158, 11, 0.2);
  }

  .file-status.file-error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .file-status.file-processing {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid rgba(6, 182, 212, 0.4);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .upload-status-container {
    margin-top: var(--space-3);
  }

  .upload-complete-indicator {
    padding: var(--space-3);
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-pending-indicator {
    padding: var(--space-3);
    background: rgba(245, 158, 11, 0.05);
    border: 1px solid rgba(245, 158, 11, 0.2);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-error-indicator {
    padding: var(--space-3);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-processing-indicator {
    padding: var(--space-3);
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid rgba(6, 182, 212, 0.4);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    animation: pulse 1.5s ease-in-out infinite;
  }

  /* Text utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-success { color: var(--zyber-success); }
  .text-error { color: var(--zyber-error); }
  .text-warning { color: var(--zyber-warning); }
  .text-muted { color: var(--zyber-text-muted); }
  .mb-4 { margin-bottom: var(--space-4); }
</style>
