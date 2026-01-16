/**
 * Core types for HAZE Blockchain SDK
 */

export type Hash = Uint8Array; // 32 bytes
export type Address = Uint8Array; // 32 bytes
export type Timestamp = number; // Unix timestamp in seconds

/**
 * Density levels for Mistborn assets
 */
export enum DensityLevel {
  Ethereal = "Ethereal", // 5KB - basic metadata
  Light = "Light",       // 50KB - main attributes + textures
  Dense = "Dense",       // 5MB - full set + 3D model
  Core = "Core",         // 50MB+ - all data + history
}

/**
 * Maximum size for each density level (in bytes)
 */
export const DENSITY_MAX_SIZES: Record<DensityLevel, number> = {
  [DensityLevel.Ethereal]: 5 * 1024,
  [DensityLevel.Light]: 50 * 1024,
  [DensityLevel.Dense]: 5 * 1024 * 1024,
  [DensityLevel.Core]: 50 * 1024 * 1024,
};

/**
 * Actions for Mistborn assets
 */
export enum AssetAction {
  Create = "Create",
  Update = "Update",
  Condense = "Condense",   // Increase density
  Evaporate = "Evaporate", // Decrease density
  Merge = "Merge",
  Split = "Split",
}

/**
 * Attribute for NFT
 */
export interface Attribute {
  name: string;
  value: string;
  rarity?: number;
}

/**
 * Asset data with density levels
 */
export interface AssetData {
  density: DensityLevel;
  metadata: Record<string, string>;
  attributes: Attribute[];
  game_id?: string;
  owner: Address;
}

/**
 * Transaction types in HAZE
 */
export type Transaction =
  | TransferTransaction
  | MistbornAssetTransaction
  | ContractCallTransaction
  | StakeTransaction;

/**
 * Transfer HAZE tokens
 */
export interface TransferTransaction {
  type: "Transfer";
  from: Address;
  to: Address;
  amount: bigint;
  fee: bigint;
  nonce: number;
  signature: Uint8Array;
}

/**
 * Create or update Mistborn NFT
 */
export interface MistbornAssetTransaction {
  type: "MistbornAsset";
  action: AssetAction;
  asset_id: Hash;
  data: AssetData;
  signature: Uint8Array;
}

/**
 * Execute smart contract
 */
export interface ContractCallTransaction {
  type: "ContractCall";
  contract: Address;
  method: string;
  args: Uint8Array;
  gas_limit: bigint;
  signature: Uint8Array;
}

/**
 * Stake tokens for validation
 */
export interface StakeTransaction {
  type: "Stake";
  validator: Address;
  amount: bigint;
  signature: Uint8Array;
}

/**
 * Block header
 */
export interface BlockHeader {
  hash: Hash;
  parent_hash: Hash;
  height: number;
  timestamp: Timestamp;
  validator: Address;
  merkle_root: Hash;
  state_root: Hash;
  wave_number: number;
  committee_id: number;
}

/**
 * Block in the HAZE blockchain
 */
export interface Block {
  header: BlockHeader;
  transactions: Transaction[];
  dag_references: Hash[];
}

/**
 * Account information
 */
export interface AccountInfo {
  address: string;
  balance: bigint;
  nonce: number;
  staked: bigint;
}

/**
 * Asset information
 */
export interface AssetInfo {
  asset_id: string;
  owner: string;
  density: string;
  created_at: number;
  updated_at: number;
}

/**
 * Block information
 */
export interface BlockInfo {
  hash: string;
  parent_hash: string;
  height: number;
  timestamp: number;
  validator: string;
  transaction_count: number;
  wave_number: number;
}

/**
 * Blockchain information
 */
export interface BlockchainInfo {
  current_height: number;
  total_supply: bigint;
  current_wave: number;
}

/**
 * Transaction response
 */
export interface TransactionResponse {
  hash: string;
  status: "pending" | "confirmed" | "failed";
}

/**
 * Liquidity pool
 */
export interface LiquidityPool {
  pool_id: string;
  asset1: string;
  asset2: string;
  reserve1: bigint;
  reserve2: bigint;
  fee_rate: number;
  total_liquidity: bigint;
}

/**
 * API Response wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}
