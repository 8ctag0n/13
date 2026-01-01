// Content script - Bridge between inject.ts and background service worker
// This script has access to chrome.runtime and acts as a message relay

const ZYBERLINK_CHANNEL = 'zyberlink-wallet';

// Inject the provider script into the page context
function injectScript() {
  const script = document.createElement('script');
  script.src = chrome.runtime.getURL('content/inject.js');
  script.type = 'module';
  (document.head || document.documentElement).appendChild(script);
  script.onload = () => script.remove();
}

// Listen for messages from the injected script (page context)
window.addEventListener('message', async (event) => {
  // Only accept messages from the same window
  if (event.source !== window) return;

  // Check for our specific channel
  if (event.data?.channel !== ZYBERLINK_CHANNEL) return;
  if (event.data?.direction !== 'to-extension') return;

  const { id, type, payload } = event.data;

  console.log('[ZyberLink Content] Received from page:', type, id);

  try {
    // Forward to background service worker
    const response = await chrome.runtime.sendMessage({
      type,
      ...payload
    });

    // Send response back to page
    window.postMessage({
      channel: ZYBERLINK_CHANNEL,
      direction: 'to-page',
      id,
      response
    }, '*');

    console.log('[ZyberLink Content] Sent response to page:', id, response);
  } catch (error) {
    // Send error back to page
    window.postMessage({
      channel: ZYBERLINK_CHANNEL,
      direction: 'to-page',
      id,
      response: {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      }
    }, '*');

    console.error('[ZyberLink Content] Error:', error);
  }
});

// Inject provider script as soon as possible
injectScript();

console.log('[ZyberLink Content] Content script initialized');

export {};
