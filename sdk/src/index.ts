/**
 * HAZE Blockchain TypeScript SDK
 * 
 * High-performance Asset Zone Engine SDK
 * "Where games breathe blockchain"
 */

// Core types
export * from './types';

// Utilities
export * from './utils';

// Cryptography
export { KeyPair } from './crypto';

// API Client
export { HazeClient, HazeClientConfig } from './client';

// Transaction builder and encoding
export { TransactionBuilder, encodeTransactionForApi } from './transaction';

// Mistborn Assets
export { MistbornAsset } from './assets';

// Fog Economy
export { FogEconomy } from './economy';

// Re-export commonly used types
import {
  DensityLevel,
  AssetAction,
  Transaction,
  Address,
  Hash,
} from './types';

export {
  DensityLevel,
  AssetAction,
  Transaction,
  Address,
  Hash,
};

/**
 * SDK version
 */
export const SDK_VERSION = '0.1.0';

/**
 * Default API endpoint
 */
export const DEFAULT_API_URL = 'http://localhost:8080';
