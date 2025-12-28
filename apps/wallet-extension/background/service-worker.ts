import { handleMessage } from '../lib/messaging/handlers';

console.log('[ZyberLink] Background service worker initialized');

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('[ZyberLink] Message received:', message.type);

  handleMessage(message)
    .then(response => {
      sendResponse({ success: true, data: response });
    })
    .catch(error => {
      console.error('[ZyberLink] Message handler error:', error);
      sendResponse({ success: false, error: error.message });
    });

  return true;
});

chrome.runtime.onInstalled.addListener((details) => {
  if (details.reason === 'install') {
    console.log('[ZyberLink] Extension installed');
    chrome.storage.local.set({ installed: true, installedAt: Date.now() });
  }
});

export {};
