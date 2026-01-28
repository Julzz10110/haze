/**
 * Mistborn Assets - Dynamic NFT system
 */

import { KeyPair, signTransaction } from './crypto';
import {
  MistbornAssetTransaction,
  AssetAction,
  AssetData,
  DensityLevel,
  Attribute,
  Hash,
  Address,
} from './types';
import { sha256, bytesToHex } from './utils';

/**
 * Mistborn Asset builder
 */
export class MistbornAsset {
  /**
   * Create asset ID from data
   */
  static createAssetId(data: string | Uint8Array): Hash {
    const input = typeof data === 'string' 
      ? Buffer.from(data, 'utf-8') 
      : Buffer.from(data);
    return sha256(input);
  }

  /**
   * Create asset data
   */
  static createAssetData(
    owner: Address,
    density: DensityLevel,
    metadata: Record<string, string>,
    attributes: Attribute[] = [],
    gameId?: string
  ): AssetData {
    return {
      density,
      metadata,
      attributes,
      game_id: gameId,
      owner,
    };
  }

  /**
   * Create a transaction to create a new asset
   */
  static createCreateTransaction(
    assetId: Hash,
    owner: Address,
    density: DensityLevel,
    metadata: Record<string, string>,
    attributes: Attribute[] = [],
    gameId?: string
  ): MistbornAssetTransaction {
    const data = this.createAssetData(owner, density, metadata, attributes, gameId);
    
    return {
      type: 'MistbornAsset',
      from: owner,
      action: AssetAction.Create,
      asset_id: assetId,
      data,
      fee: 0n,
      nonce: 0,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Create a transaction to update an asset
   */
  static createUpdateTransaction(
    assetId: Hash,
    owner: Address,
    metadata: Record<string, string>,
    attributes: Attribute[] = []
  ): MistbornAssetTransaction {
    const data: AssetData = {
      density: DensityLevel.Ethereal, // Will be updated by backend
      metadata,
      attributes,
      owner,
    };

    return {
      type: 'MistbornAsset',
      from: owner,
      action: AssetAction.Update,
      asset_id: assetId,
      data,
      fee: 0n,
      nonce: 0,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Create a transaction to condense (increase density) an asset
   */
  static createCondenseTransaction(
    assetId: Hash,
    owner: Address,
    newDensity: DensityLevel,
    additionalMetadata: Record<string, string> = {},
    additionalAttributes: Attribute[] = []
  ): MistbornAssetTransaction {
    const data: AssetData = {
      density: newDensity,
      metadata: additionalMetadata,
      attributes: additionalAttributes,
      owner,
    };

    return {
      type: 'MistbornAsset',
      from: owner,
      action: AssetAction.Condense,
      asset_id: assetId,
      data,
      fee: 0n,
      nonce: 0,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Create a transaction to evaporate (decrease density) an asset
   */
  static createEvaporateTransaction(
    assetId: Hash,
    owner: Address,
    newDensity: DensityLevel
  ): MistbornAssetTransaction {
    const data: AssetData = {
      density: newDensity,
      metadata: {},
      attributes: [],
      owner,
    };

    return {
      type: 'MistbornAsset',
      from: owner,
      action: AssetAction.Evaporate,
      asset_id: assetId,
      data,
      fee: 0n,
      nonce: 0,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Sign an asset transaction
   */
  static async sign(
    transaction: MistbornAssetTransaction,
    keyPair: KeyPair
  ): Promise<MistbornAssetTransaction> {
    const signature = await signTransaction(transaction, keyPair);
    return {
      ...transaction,
      signature,
    };
  }

  /**
   * Get asset ID as hex string
   */
  static assetIdToHex(assetId: Hash): string {
    return bytesToHex(assetId);
  }

  /**
   * Get asset ID from hex string
   */
  static hexToAssetId(hex: string): Hash | null {
    try {
      const bytes = new Uint8Array(Buffer.from(hex, 'hex'));
      if (bytes.length !== 32) {
        return null;
      }
      return bytes;
    } catch {
      return null;
    }
  }
}
