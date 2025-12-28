// Provider script - Injected into page context
// Communicates with content script via postMessage

const ZYBERLINK_CHANNEL = 'zyberlink-wallet';

// Pending requests awaiting response
const pendingRequests = new Map<string, {
  resolve: (value: any) => void;
  reject: (error: Error) => void;
}>();

// Generate unique request ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

// Send message to content script and wait for response
function sendToExtension(type: string, payload: any = {}): Promise<any> {
  return new Promise((resolve, reject) => {
    const id = generateId();

    // Store pending request
    pendingRequests.set(id, { resolve, reject });

    // Send to content script
    window.postMessage({
      channel: ZYBERLINK_CHANNEL,
      direction: 'to-extension',
      id,
      type,
      payload
    }, '*');

    // Timeout after 30 seconds
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }
    }, 30000);
  });
}

// Listen for responses from content script
window.addEventListener('message', (event) => {
  if (event.source !== window) return;
  if (event.data?.channel !== ZYBERLINK_CHANNEL) return;
  if (event.data?.direction !== 'to-page') return;

  const { id, response } = event.data;
  const pending = pendingRequests.get(id);

  if (pending) {
    pendingRequests.delete(id);

    if (response?.success) {
      pending.resolve(response.data);
    } else {
      pending.reject(new Error(response?.error || 'Request failed'));
    }
  }
});

// Event emitter for wallet events
class EventEmitter {
  private listeners: Map<string, Set<Function>> = new Map();

  on(event: string, callback: Function) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback);
  }

  off(event: string, callback: Function) {
    this.listeners.get(event)?.delete(callback);
  }

  emit(event: string, ...args: any[]) {
    this.listeners.get(event)?.forEach(cb => cb(...args));
  }
}

// ZyberLink Provider class
class ZyberLinkProvider extends EventEmitter {
  private _chainId: string;
  private _publicKey: string | null = null;
  private _isConnected: boolean = false;

  constructor(chain: 'solana' | 'starknet' | 'zcash') {
    super();
    this._chainId = chain;
  }

  get publicKey(): string | null {
    return this._publicKey;
  }

  get isConnected(): boolean {
    return this._isConnected;
  }

  get isZyberLink(): boolean {
    return true;
  }

  async connect(): Promise<{ publicKey: string }> {
    console.log(`[ZyberLink] Connecting to ${this._chainId}...`);

    try {
      const result = await sendToExtension('CONNECT', { chain: this._chainId });
      this._publicKey = result.publicKey;
      this._isConnected = true;

      this.emit('connect', { publicKey: this._publicKey });

      console.log(`[ZyberLink] Connected:`, this._publicKey);
      return { publicKey: this._publicKey };
    } catch (error) {
      console.error(`[ZyberLink] Connection failed:`, error);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    console.log(`[ZyberLink] Disconnecting from ${this._chainId}...`);

    try {
      await sendToExtension('DISCONNECT', { chain: this._chainId });
      this._publicKey = null;
      this._isConnected = false;

      this.emit('disconnect');
    } catch (error) {
      console.error(`[ZyberLink] Disconnect failed:`, error);
      throw error;
    }
  }

  async signMessage(message: Uint8Array | string): Promise<{ signature: Uint8Array }> {
    if (!this._isConnected) {
      throw new Error('Wallet not connected');
    }

    console.log(`[ZyberLink] Signing message on ${this._chainId}...`);

    const result = await sendToExtension('SIGN_MESSAGE', {
      chain: this._chainId,
      message: typeof message === 'string' ? message : Array.from(message)
    });

    return {
      signature: new Uint8Array(result.signature)
    };
  }

  async signTransaction(transaction: any): Promise<any> {
    if (!this._isConnected) {
      throw new Error('Wallet not connected');
    }

    console.log(`[ZyberLink] Signing transaction on ${this._chainId}...`);

    return await sendToExtension('SIGN_TRANSACTION', {
      chain: this._chainId,
      transaction
    });
  }

  async signAndSendTransaction(transaction: any): Promise<{ signature: string }> {
    if (!this._isConnected) {
      throw new Error('Wallet not connected');
    }

    console.log(`[ZyberLink] Sign and send transaction on ${this._chainId}...`);

    return await sendToExtension('SIGN_AND_SEND_TRANSACTION', {
      chain: this._chainId,
      transaction
    });
  }

  async signAllTransactions(transactions: any[]): Promise<any[]> {
    if (!this._isConnected) {
      throw new Error('Wallet not connected');
    }

    console.log(`[ZyberLink] Signing ${transactions.length} transactions...`);

    const signed = [];
    for (const tx of transactions) {
      signed.push(await this.signTransaction(tx));
    }
    return signed;
  }

  // Solana-specific: request method for wallet-standard compatibility
  async request(args: { method: string; params?: any }): Promise<any> {
    const { method, params } = args;

    switch (method) {
      case 'connect':
        return this.connect();
      case 'disconnect':
        return this.disconnect();
      case 'signMessage':
        return this.signMessage(params?.message);
      case 'signTransaction':
        return this.signTransaction(params?.transaction);
      case 'signAndSendTransaction':
        return this.signAndSendTransaction(params?.transaction);
      default:
        throw new Error(`Method not supported: ${method}`);
    }
  }
}

// Inject providers into window
const injectProviders = () => {
  const solanaProvider = new ZyberLinkProvider('solana');
  const starknetProvider = new ZyberLinkProvider('starknet');
  const zcashProvider = new ZyberLinkProvider('zcash');

  // Standard wallet interfaces
  Object.defineProperty(window, 'solana', {
    value: solanaProvider,
    writable: false,
    configurable: false
  });

  Object.defineProperty(window, 'starknet', {
    value: starknetProvider,
    writable: false,
    configurable: false
  });

  // ZyberLink unified interface
  Object.defineProperty(window, 'zyberlink', {
    value: {
      solana: solanaProvider,
      starknet: starknetProvider,
      zcash: zcashProvider,
      version: '0.2.0',
      isZyberLink: true
    },
    writable: false,
    configurable: false
  });

  // Dispatch initialization event
  window.dispatchEvent(new CustomEvent('zyberlink#initialized', {
    detail: { version: '0.2.0' }
  }));

  // Dispatch wallet-standard discovery event (for Solana)
  window.dispatchEvent(new Event('wallet-standard:app-ready'));

  console.log('[ZyberLink] Providers injected - v0.2.0');
};

// Inject immediately
injectProviders();

export {};
