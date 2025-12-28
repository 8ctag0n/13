export async function sendMessage<T = any>(message: any): Promise<T> {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(message, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      } else if (!response) {
        reject(new Error('No response from background'));
      } else if (response.success) {
        resolve(response.data);
      } else {
        reject(new Error(response.error || 'Unknown error'));
      }
    });
  });
}

export function truncateAddress(address: string, start = 6, end = 4): string {
  if (!address || address.length <= start + end) {
    return address;
  }
  return `${address.slice(0, start)}...${address.slice(-end)}`;
}

export function formatBalance(balance: string | number, decimals = 4): string {
  const num = typeof balance === 'string' ? parseFloat(balance) : balance;
  if (isNaN(num)) return '0.0000';
  return num.toFixed(decimals);
}

export function validateAddress(address: string, chain: string): boolean {
  switch (chain) {
    case 'solana':
      return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(address);
    case 'starknet':
      return /^0x[a-fA-F0-9]{64}$/.test(address);
    case 'zcash':
      return /^[tz][a-zA-Z0-9]{34,95}$/.test(address);
    default:
      return false;
  }
}

export function getChainSymbol(chain: string): string {
  const symbols: Record<string, string> = {
    solana: 'SOL',
    starknet: 'ETH',
    zcash: 'ZEC'
  };
  return symbols[chain] || chain.toUpperCase();
}

export function getChainColor(chain: string): string {
  const colors: Record<string, string> = {
    solana: 'var(--cyber-cyan)',
    starknet: 'var(--cyber-purple)',
    zcash: 'var(--cyber-yellow)'
  };
  return colors[chain] || 'var(--cyber-cyan)';
}
