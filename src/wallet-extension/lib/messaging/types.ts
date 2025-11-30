import type { SupportedChain } from '../chains/types';

export type MessageType =
  | 'CONNECT'
  | 'DISCONNECT'
  | 'SIGN_MESSAGE'
  | 'SIGN_TRANSACTION'
  | 'SIGN_AND_SEND_TRANSACTION'
  | 'SEND_TRANSACTION'
  | 'BUILD_TRANSACTION'
  | 'ESTIMATE_FEE'
  | 'GET_BALANCE'
  | 'GET_ADDRESS'
  | 'UNLOCK_WALLET'
  | 'LOCK_WALLET'
  | 'CREATE_WALLET'
  | 'IMPORT_WALLET'
  | 'EXPORT_PRIVATE_KEY'
  | 'GET_WALLET_STATE'
  | 'GET_RPC_ENDPOINTS'
  | 'SET_ACTIVE_RPC'
  | 'ADD_CUSTOM_RPC'
  | 'REMOVE_CUSTOM_RPC'
  | 'GET_TRANSACTIONS'
  | 'SAVE_TRANSACTION'
  | 'GET_ZCASH_SHIELDED_ADDRESS'
  | 'GET_ZCASH_TOTAL_BALANCE'
  | 'SHIELD_ZCASH_FUNDS'
  | 'MINE_ZCASH_BLOCKS';

export interface Message {
  type: MessageType;
  chain?: SupportedChain;
  data?: any;
  [key: string]: any;
}

export interface MessageResponse {
  success: boolean;
  data?: any;
  error?: string;
}

export interface ConnectMessage extends Message {
  type: 'CONNECT';
  chain: SupportedChain;
}

export interface DisconnectMessage extends Message {
  type: 'DISCONNECT';
  chain: SupportedChain;
}

export interface SignMessageMessage extends Message {
  type: 'SIGN_MESSAGE';
  chain: SupportedChain;
  message: string | Uint8Array;
}

export interface SignTransactionMessage extends Message {
  type: 'SIGN_TRANSACTION';
  chain: SupportedChain;
  transaction: any;
}

export interface SignAndSendTransactionMessage extends Message {
  type: 'SIGN_AND_SEND_TRANSACTION';
  chain: SupportedChain;
  transaction: any;
}

export interface GetBalanceMessage extends Message {
  type: 'GET_BALANCE';
  chain: SupportedChain;
  address?: string;
}

export interface GetAddressMessage extends Message {
  type: 'GET_ADDRESS';
  chain: SupportedChain;
}

export interface UnlockWalletMessage extends Message {
  type: 'UNLOCK_WALLET';
  password: string;
}

export interface CreateWalletMessage extends Message {
  type: 'CREATE_WALLET';
  password: string;
  privateKey?: string;  // hex encoded expanded private key (64 bytes = 128 chars)
}

export interface ImportWalletMessage extends Message {
  type: 'IMPORT_WALLET';
  password: string;
  privateKey: string;  // hex encoded expanded private key
}

export interface GetWalletStateMessage extends Message {
  type: 'GET_WALLET_STATE';
}

export interface WalletState {
  isLocked: boolean;
  hasVault: boolean;
  addresses?: {
    solana: string;
    starknet: string;
    zcash: string;
  };
}
