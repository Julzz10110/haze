/**
 * Tests for getTransactionDataForSigning (canonical payload for node verification)
 */
import { describe, it, expect } from 'vitest';
import { getTransactionDataForSigning } from './crypto';
import { AssetAction, DensityLevel } from './types';

function bytes32(n: number): Uint8Array {
  const a = new Uint8Array(32);
  a.fill(n & 0xff);
  return a;
}

describe('getTransactionDataForSigning', () => {
  describe('Transfer', () => {
    it('produces payload starting with Transfer tag', () => {
      const payload = getTransactionDataForSigning({
        type: 'Transfer',
        from: bytes32(1),
        to: bytes32(2),
        amount: 1000n,
        fee: 10n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(new TextDecoder().decode(payload.subarray(0, 8))).toBe('Transfer');
    });

    it('payload length is 8 + 32 + 32 + 8 + 8 + 8', () => {
      const payload = getTransactionDataForSigning({
        type: 'Transfer',
        from: bytes32(1),
        to: bytes32(2),
        amount: 1000n,
        fee: 10n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(payload.length).toBe(8 + 32 + 32 + 8 + 8 + 8);
    });

    it('changing nonce changes payload', () => {
      const base = {
        type: 'Transfer' as const,
        from: bytes32(1),
        to: bytes32(2),
        amount: 1000n,
        fee: 10n,
        nonce: 0,
        signature: new Uint8Array(0),
      };
      const p0 = getTransactionDataForSigning({ ...base, nonce: 0 });
      const p1 = getTransactionDataForSigning({ ...base, nonce: 1 });
      expect(p0).not.toEqual(p1);
    });

    it('changing fee changes payload', () => {
      const base = {
        type: 'Transfer' as const,
        from: bytes32(1),
        to: bytes32(2),
        amount: 1000n,
        fee: 10n,
        nonce: 0,
        signature: new Uint8Array(0),
      };
      const p0 = getTransactionDataForSigning({ ...base, fee: 10n });
      const p1 = getTransactionDataForSigning({ ...base, fee: 20n });
      expect(p0).not.toEqual(p1);
    });
  });

  describe('Stake', () => {
    it('produces payload starting with Stake tag', () => {
      const payload = getTransactionDataForSigning({
        type: 'Stake',
        from: bytes32(1),
        validator: bytes32(2),
        amount: 5000n,
        fee: 0n,
        nonce: 1,
        signature: new Uint8Array(0),
      });
      expect(new TextDecoder().decode(payload.subarray(0, 5))).toBe('Stake');
    });

    it('changing nonce changes payload', () => {
      const base = {
        type: 'Stake' as const,
        from: bytes32(1),
        validator: bytes32(2),
        amount: 5000n,
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      };
      const p0 = getTransactionDataForSigning({ ...base, nonce: 0 });
      const p1 = getTransactionDataForSigning({ ...base, nonce: 2 });
      expect(p0).not.toEqual(p1);
    });
  });

  describe('ContractCall', () => {
    const CONTRACTCALL_TAG_LEN = 12; // "ContractCall".length

    it('produces payload starting with ContractCall tag', () => {
      const payload = getTransactionDataForSigning({
        type: 'ContractCall',
        from: bytes32(1),
        contract: bytes32(2),
        method: 'transfer',
        args: new Uint8Array(0),
        gas_limit: 100000n,
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(new TextDecoder().decode(payload.subarray(0, CONTRACTCALL_TAG_LEN))).toBe('ContractCall');
    });

    it('includes from, fee, nonce in payload', () => {
      const from = bytes32(0xab);
      const payload = getTransactionDataForSigning({
        type: 'ContractCall',
        from,
        contract: bytes32(2),
        method: 'x',
        args: new Uint8Array(0),
        gas_limit: 1n,
        fee: 5n,
        nonce: 3,
        signature: new Uint8Array(0),
      });
      // After "ContractCall" (12 bytes) comes from (32 bytes)
      const fromInPayload = payload.subarray(CONTRACTCALL_TAG_LEN, CONTRACTCALL_TAG_LEN + 32);
      expect(fromInPayload).toEqual(from);
    });
  });

  describe('MistbornAsset', () => {
    it('produces payload starting with MistbornAsset tag', () => {
      const payload = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Create,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: {},
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(new TextDecoder().decode(payload.subarray(0, 13))).toBe('MistbornAsset');
    });

    it('changing nonce changes payload', () => {
      const base = {
        type: 'MistbornAsset' as const,
        from: bytes32(1),
        action: AssetAction.Create as const,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal as const,
          metadata: {} as Record<string, string>,
          attributes: [] as { name: string; value: string; rarity?: number }[],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      };
      const p0 = getTransactionDataForSigning({ ...base, nonce: 0 });
      const p1 = getTransactionDataForSigning({ ...base, nonce: 1 });
      expect(p0).not.toEqual(p1);
    });

    it('changing fee changes payload', () => {
      const base = {
        type: 'MistbornAsset' as const,
        from: bytes32(1),
        action: AssetAction.Create as const,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal as const,
          metadata: {} as Record<string, string>,
          attributes: [] as { name: string; value: string; rarity?: number }[],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      };
      const p0 = getTransactionDataForSigning({ ...base, fee: 0n });
      const p1 = getTransactionDataForSigning({ ...base, fee: 1n });
      expect(p0).not.toEqual(p1);
    });

    it('Create action uses action byte 0', () => {
      const payload = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Create,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: {},
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      // After "MistbornAsset"(13) + from(32) = 45, next byte is action
      expect(payload[45]).toBe(0);
    });

    it('Merge action includes other_asset_id in payload when in metadata', () => {
      const otherId = bytes32(99);
      const otherIdHex = Buffer.from(otherId).toString('hex');
      const payloadWith = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Merge,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: { _other_asset_id: otherIdHex },
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      const payloadWithout = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Merge,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: {},
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(payloadWith.length).toBeGreaterThan(payloadWithout.length);
    });

    it('Split action includes _components in payload when in metadata', () => {
      const payloadWith = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Split,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: { _components: 'id1,id2' },
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      const payloadWithout = getTransactionDataForSigning({
        type: 'MistbornAsset',
        from: bytes32(1),
        action: AssetAction.Split,
        asset_id: bytes32(10),
        data: {
          density: DensityLevel.Ethereal,
          metadata: {},
          attributes: [],
          owner: bytes32(1),
        },
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(0),
      });
      expect(payloadWith.length).toBeGreaterThan(payloadWithout.length);
    });
  });
});
