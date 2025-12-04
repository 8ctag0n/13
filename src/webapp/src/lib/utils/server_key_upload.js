/**
 * Server Key Pre-Upload Utility
 *
 * Uploads large TFHE server keys (~117MB) separately from job creation
 * to avoid timeout issues with the main validate-and-build endpoint.
 *
 * Usage:
 * 1. Parse witness.bin to extract serverKey bytes
 * 2. Call uploadServerKey() with raw bytes and progress callback
 * 3. Use returned server_key_hash in job creation request
 */

/**
 * Upload a TFHE server key to the backend
 *
 * @param {Uint8Array} serverKeyBytes - Raw server key bytes (bincode-serialized tfhe::ServerKey)
 * @param {string} apiBaseUrl - API base URL (e.g., 'https://demo.zyberlink.fun')
 * @param {Function} [onProgress] - Progress callback: (progress: {loaded, total, percent}) => void
 * @returns {Promise<{server_key_hash: string, size_bytes: number}>}
 */
export async function uploadServerKey(serverKeyBytes, apiBaseUrl, onProgress = () => {}) {
  if (!serverKeyBytes || serverKeyBytes.length === 0) {
    throw new Error('Server key bytes are empty');
  }

  const sizeMB = serverKeyBytes.length / (1024 * 1024);
  console.log(`[ServerKey Upload] Starting upload of ${sizeMB.toFixed(2)} MB`);

  // Validate size (must be between 40MB and 120MB)
  const MIN_SIZE = 40 * 1024 * 1024;
  const MAX_SIZE = 120 * 1024 * 1024;

  if (serverKeyBytes.length < MIN_SIZE) {
    throw new Error(`Server key too small: ${sizeMB.toFixed(2)} MB (min: 40 MB)`);
  }

  if (serverKeyBytes.length > MAX_SIZE) {
    throw new Error(`Server key too large: ${sizeMB.toFixed(2)} MB (max: 120 MB)`);
  }

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    const url = `${apiBaseUrl}/api/server-key/upload`;

    // Track timing for speed calculation
    const startTime = Date.now();
    let lastLoaded = 0;
    let lastTime = startTime;

    xhr.open('POST', url, true);
    xhr.setRequestHeader('Content-Type', 'application/octet-stream');

    // Track upload progress with speed calculation
    xhr.upload.onprogress = (event) => {
      if (event.lengthComputable) {
        const now = Date.now();
        const percent = Math.round((event.loaded / event.total) * 100);

        // Calculate speed (bytes per second) using rolling average
        const elapsedTotal = (now - startTime) / 1000; // seconds
        const avgSpeed = elapsedTotal > 0 ? event.loaded / elapsedTotal : 0;

        // Calculate instant speed (last interval)
        const intervalTime = (now - lastTime) / 1000;
        const intervalBytes = event.loaded - lastLoaded;
        const instantSpeed = intervalTime > 0.1 ? intervalBytes / intervalTime : avgSpeed;

        // Use weighted average for smoother display
        const displaySpeed = avgSpeed * 0.7 + instantSpeed * 0.3;

        // Estimate remaining time
        const remaining = event.total - event.loaded;
        const etaSeconds = displaySpeed > 0 ? remaining / displaySpeed : 0;

        // Format values
        const loadedMB = event.loaded / (1024 * 1024);
        const totalMB = event.total / (1024 * 1024);
        const speedKBps = displaySpeed / 1024;
        const speedMBps = displaySpeed / (1024 * 1024);

        // Update for next interval
        lastLoaded = event.loaded;
        lastTime = now;

        onProgress({
          loaded: event.loaded,
          total: event.total,
          percent,
          phase: 'uploading',
          // New detailed info
          loadedMB: loadedMB.toFixed(1),
          totalMB: totalMB.toFixed(1),
          speedKBps: speedKBps.toFixed(0),
          speedMBps: speedMBps.toFixed(2),
          etaSeconds: Math.round(etaSeconds),
          etaFormatted: formatEta(etaSeconds)
        });

        console.log(`[ServerKey Upload] ${percent}% | ${loadedMB.toFixed(1)}/${totalMB.toFixed(1)} MB | ${speedKBps.toFixed(0)} KB/s | ETA: ${formatEta(etaSeconds)}`);
      }
    };

    function formatEta(seconds) {
      if (seconds <= 0 || !isFinite(seconds)) return '--:--';
      const mins = Math.floor(seconds / 60);
      const secs = Math.round(seconds % 60);
      return `${mins}:${secs.toString().padStart(2, '0')}`;
    }

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const response = JSON.parse(xhr.responseText);
          console.log('[ServerKey Upload] Success:', response);
          onProgress({ percent: 100, phase: 'complete' });
          resolve(response);
        } catch (e) {
          reject(new Error(`Failed to parse server response: ${e.message}`));
        }
      } else {
        let errorMsg = `Upload failed with status ${xhr.status}`;
        try {
          const errorData = JSON.parse(xhr.responseText);
          errorMsg = errorData.error || errorMsg;
        } catch (e) {
          // Use status text if JSON parse fails
          errorMsg = xhr.statusText || errorMsg;
        }
        console.error('[ServerKey Upload] Error:', errorMsg);
        reject(new Error(errorMsg));
      }
    };

    xhr.onerror = () => {
      console.error('[ServerKey Upload] Network error');
      reject(new Error('Network error during server key upload'));
    };

    xhr.ontimeout = () => {
      console.error('[ServerKey Upload] Timeout');
      reject(new Error('Server key upload timed out'));
    };

    // Set very generous timeout for large uploads (20 minutes)
    // Users with slow connections (~100KB/s) need ~20min for 117MB
    xhr.timeout = 1200000;

    // Send raw bytes
    xhr.send(serverKeyBytes);
  });
}

/**
 * Check if a server key already exists on the backend
 *
 * @param {string} serverKeyHash - Hex-encoded Blake2s256 hash
 * @param {string} apiBaseUrl - API base URL
 * @returns {Promise<boolean>}
 */
export async function checkServerKeyExists(serverKeyHash, apiBaseUrl) {
  try {
    const response = await fetch(`${apiBaseUrl}/api/server-key/${serverKeyHash}/exists`);
    if (!response.ok) {
      return false;
    }
    const data = await response.json();
    return data.exists === true;
  } catch (e) {
    console.warn('[ServerKey] Error checking existence:', e);
    return false;
  }
}

/**
 * Compute Blake2s256 hash of server key bytes (client-side)
 * This allows checking if upload is needed before actually uploading
 *
 * @param {Uint8Array} serverKeyBytes - Raw server key bytes
 * @returns {Promise<string>} - Hex-encoded hash
 */
export async function computeServerKeyHash(serverKeyBytes) {
  // Use SubtleCrypto if available, otherwise fall back to a simple implementation
  // Note: SubtleCrypto doesn't support Blake2s, so we'll need to compute on server
  // For now, just return null to indicate hash computation should happen server-side
  console.log('[ServerKey] Hash computation delegated to server');
  return null;
}
