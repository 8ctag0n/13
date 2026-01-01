export interface Transaction {
  id: string;
  type: 'send' | 'receive' | 'swap' | 'contract';
  chain: SupportedChain;
  amount: string;
  symbol: string;
  to?: string;
  from?: string;
  timestamp: number;
  status: 'pending' | 'confirmed' | 'failed';
  signature?: string;
}

export interface ChainAdapter {
  chainId: string;
  name: string;
  symbol: string;

  getAddress(keypair: any): string;
  getBalance(address: string, rpcUrl: string): Promise<string>;
  getTransactions(address: string, rpcUrl: string, limit?: number): Promise<Transaction[]>;
  buildTransaction(params: TxParams, rpcUrl?: string): Promise<any>;
  signTransaction(tx: any, keypair: any): Promise<any>;
  sendTransaction(signedTx: any, rpcUrl: string): Promise<string>;
}

export interface TxParams {
  to: string;
  amount: string;
  memo?: string;
  from?: string;
}

export interface RPCConfig {
  chainId: string;
  endpoints: RPCEndpoint[];
  activeEndpoint: string;
}

export interface RPCEndpoint {
  url: string;
  name: string;
  custom: boolean;
}

export interface ChainConfig {
  solana: RPCConfig;
  starknet: RPCConfig;
  zcash: RPCConfig;
}

export type SupportedChain = 'solana' | 'starknet' | 'zcash';

export interface WalletBalance {
  chain: SupportedChain;
  address: string;
  balance: string;
  symbol: string;
}
