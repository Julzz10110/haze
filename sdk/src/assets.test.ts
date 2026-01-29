/**
 * Tests for MistbornAsset builders (createCreateTransaction, createUpdateTransaction, etc.)
 */
import { describe, it, expect } from 'vitest';
import { MistbornAsset } from './assets';
import { DensityLevel, AssetAction } from './types';

function bytes32(n: number): Uint8Array {
  const a = new Uint8Array(32);
  a.fill(n & 0xff);
  return a;
}

describe('MistbornAsset', () => {
  describe('createAssetId', () => {
    it('returns 32 bytes', () => {
      const id = MistbornAsset.createAssetId('test');
      expect(id).toBeInstanceOf(Uint8Array);
      expect(id.length).toBe(32);
    });

    it('same input gives same id', () => {
      const id1 = MistbornAsset.createAssetId('same');
      const id2 = MistbornAsset.createAssetId('same');
      expect(id1).toEqual(id2);
    });

    it('different input gives different id', () => {
      const id1 = MistbornAsset.createAssetId('a');
      const id2 = MistbornAsset.createAssetId('b');
      expect(id1).not.toEqual(id2);
    });
  });

  describe('createCreateTransaction', () => {
    it('returns MistbornAsset transaction with action Create', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createCreateTransaction(
        assetId,
        owner,
        DensityLevel.Ethereal,
        { name: 'Test' },
        [],
        undefined
      );
      expect(tx.type).toBe('MistbornAsset');
      expect(tx.action).toBe(AssetAction.Create);
      expect(tx.asset_id).toEqual(assetId);
    });

    it('includes from, fee, nonce', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createCreateTransaction(
        assetId,
        owner,
        DensityLevel.Ethereal,
        {},
        []
      );
      expect(tx.from).toEqual(owner);
      expect(tx.fee).toBe(0n);
      expect(tx.nonce).toBe(0);
    });

    it('data.owner equals from', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createCreateTransaction(
        assetId,
        owner,
        DensityLevel.Light,
        {},
        []
      );
      expect(tx.data.owner).toEqual(owner);
    });

    it('signature is empty', () => {
      const tx = MistbornAsset.createCreateTransaction(
        MistbornAsset.createAssetId('x'),
        bytes32(1),
        DensityLevel.Ethereal,
        {},
        []
      );
      expect(tx.signature.length).toBe(0);
    });
  });

  describe('createUpdateTransaction', () => {
    it('returns MistbornAsset transaction with action Update', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createUpdateTransaction(
        assetId,
        owner,
        { key: 'value' },
        []
      );
      expect(tx.type).toBe('MistbornAsset');
      expect(tx.action).toBe(AssetAction.Update);
      expect(tx.from).toEqual(owner);
      expect(tx.fee).toBe(0n);
      expect(tx.nonce).toBe(0);
    });
  });

  describe('createCondenseTransaction', () => {
    it('returns MistbornAsset transaction with action Condense', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createCondenseTransaction(
        assetId,
        owner,
        DensityLevel.Dense,
        {},
        []
      );
      expect(tx.type).toBe('MistbornAsset');
      expect(tx.action).toBe(AssetAction.Condense);
      expect(tx.from).toEqual(owner);
      expect(tx.fee).toBe(0n);
      expect(tx.nonce).toBe(0);
    });
  });

  describe('createEvaporateTransaction', () => {
    it('returns MistbornAsset transaction with action Evaporate', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('asset1');
      const tx = MistbornAsset.createEvaporateTransaction(
        assetId,
        owner,
        DensityLevel.Ethereal
      );
      expect(tx.type).toBe('MistbornAsset');
      expect(tx.action).toBe(AssetAction.Evaporate);
      expect(tx.from).toEqual(owner);
      expect(tx.fee).toBe(0n);
      expect(tx.nonce).toBe(0);
    });
  });

  describe('assetIdToHex / hexToAssetId', () => {
    it('roundtrip preserves asset id', () => {
      const id = MistbornAsset.createAssetId('roundtrip');
      const hex = MistbornAsset.assetIdToHex(id);
      const back = MistbornAsset.hexToAssetId(hex);
      expect(back).not.toBeNull();
      expect(back!).toEqual(id);
    });

    it('hexToAssetId returns null for invalid hex', () => {
      expect(MistbornAsset.hexToAssetId('zz')).toBeNull();
      expect(MistbornAsset.hexToAssetId('ab')).toBeNull(); // not 32 bytes
    });
  });
});
