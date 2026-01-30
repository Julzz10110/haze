/**
 * Tests for TransactionBuilder (createTransfer, createStake, sign) and encodeTransactionForApi
 */
import { describe, it, expect } from 'vitest';
import { TransactionBuilder, encodeTransactionForApi } from './transaction';
import { KeyPair } from './crypto';
import { MistbornAsset } from './assets';
import { DensityLevel, AssetAction } from './types';
import type { ContractCallTransaction } from './types';

function bytes32(n: number): Uint8Array {
  const a = new Uint8Array(32);
  a.fill(n & 0xff);
  return a;
}

describe('TransactionBuilder', () => {
  describe('createTransfer', () => {
    it('returns object with type Transfer', () => {
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        1000n,
        10n,
        0
      );
      expect(tx.type).toBe('Transfer');
    });

    it('includes from, to, amount, fee, nonce', () => {
      const from = bytes32(1);
      const to = bytes32(2);
      const tx = TransactionBuilder.createTransfer(from, to, 1000n, 10n, 5);
      expect(tx.from).toEqual(from);
      expect(tx.to).toEqual(to);
      expect(tx.amount).toBe(1000n);
      expect(tx.fee).toBe(10n);
      expect(tx.nonce).toBe(5);
    });

    it('signature is empty Uint8Array', () => {
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        1000n,
        10n,
        0
      );
      expect(tx.signature).toBeInstanceOf(Uint8Array);
      expect(tx.signature.length).toBe(0);
    });
  });

  describe('createStake', () => {
    it('returns object with type Stake', () => {
      const tx = TransactionBuilder.createStake(
        bytes32(1),
        bytes32(2),
        5000n,
        0n,
        0
      );
      expect(tx.type).toBe('Stake');
    });

    it('includes from, validator, amount, fee, nonce', () => {
      const from = bytes32(1);
      const validator = bytes32(2);
      const tx = TransactionBuilder.createStake(from, validator, 5000n, 1n, 2);
      expect(tx.from).toEqual(from);
      expect(tx.validator).toEqual(validator);
      expect(tx.amount).toBe(5000n);
      expect(tx.fee).toBe(1n);
      expect(tx.nonce).toBe(2);
    });

    it('signature is empty Uint8Array', () => {
      const tx = TransactionBuilder.createStake(
        bytes32(1),
        bytes32(2),
        5000n,
        0n,
        0
      );
      expect(tx.signature).toBeInstanceOf(Uint8Array);
      expect(tx.signature.length).toBe(0);
    });
  });

  describe('sign', () => {
    it('returns transaction with non-empty signature for Transfer', async () => {
      const keyPair = await KeyPair.generate();
      const tx = TransactionBuilder.createTransfer(
        keyPair.getAddress(),
        bytes32(2),
        1000n,
        10n,
        0
      );
      const signed = await TransactionBuilder.sign(tx, keyPair);
      expect(signed.type).toBe('Transfer');
      expect((signed as { signature: Uint8Array }).signature.length).toBe(64);
    });

    it('returns transaction with non-empty signature for Stake', async () => {
      const keyPair = await KeyPair.generate();
      const tx = TransactionBuilder.createStake(
        keyPair.getAddress(),
        bytes32(2),
        5000n,
        0n,
        0
      );
      const signed = await TransactionBuilder.sign(tx, keyPair);
      expect(signed.type).toBe('Stake');
      expect((signed as { signature: Uint8Array }).signature.length).toBe(64);
    });
  });

  describe('getHash / getHashHex', () => {
    it('getHash returns 32 bytes', () => {
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        1000n,
        10n,
        0
      );
      const hash = TransactionBuilder.getHash(tx);
      expect(hash).toBeInstanceOf(Uint8Array);
      expect(hash.length).toBe(32);
    });

    it('getHashHex returns hex string of length 64', () => {
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        1000n,
        10n,
        0
      );
      const hex = TransactionBuilder.getHashHex(tx);
      expect(hex).toMatch(/^[0-9a-f]{64}$/);
    });
  });

  describe('encodeTransactionForApi', () => {
    it('Transfer: encodes to API format (hex strings, decimal strings for bigint)', () => {
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        1000n,
        10n,
        5
      );
      tx.signature = new Uint8Array(64).fill(0xab);
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('Transfer');
      expect(encoded.from).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.to).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.amount).toBe('1000');
      expect(encoded.fee).toBe('10');
      expect(encoded.nonce).toBe(5);
      expect(encoded.signature).toMatch(/^[0-9a-f]{128}$/);
    });

    it('Stake: encodes to API format', () => {
      const tx = TransactionBuilder.createStake(
        bytes32(1),
        bytes32(2),
        5000n,
        1n,
        2
      );
      tx.signature = new Uint8Array(64).fill(0);
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('Stake');
      expect(encoded.from).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.validator).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.amount).toBe('5000');
      expect(encoded.fee).toBe('1');
      expect(encoded.nonce).toBe(2);
      expect(encoded.signature).toMatch(/^[0-9a-f]{128}$/);
    });

    it('ContractCall: encodes to API format', () => {
      const tx: ContractCallTransaction = {
        type: 'ContractCall',
        from: bytes32(1),
        contract: bytes32(2),
        method: 'transfer',
        args: new Uint8Array([1, 2, 3]),
        gas_limit: 100000n,
        fee: 0n,
        nonce: 0,
        signature: new Uint8Array(64),
      };
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('ContractCall');
      expect(encoded.from).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.contract).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.method).toBe('transfer');
      expect(encoded.args).toMatch(/^[0-9a-f]+$/);
      expect(encoded.gas_limit).toBe('100000');
      expect(encoded.fee).toBe('0');
      expect(encoded.nonce).toBe(0);
      expect(encoded.signature).toMatch(/^[0-9a-f]{128}$/);
    });

    it('MistbornAsset Create: encodes to API format', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('create-test');
      const tx = MistbornAsset.createCreateTransaction(
        assetId,
        owner,
        DensityLevel.Ethereal,
        { name: 'Test' },
        [],
        undefined
      );
      tx.signature = new Uint8Array(64).fill(0x11);
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('MistbornAsset');
      expect(encoded.action).toBe(AssetAction.Create);
      expect(encoded.from).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.asset_id).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.fee).toBe('0');
      expect(encoded.nonce).toBe(0);
      expect(encoded.data).toBeDefined();
      const data = encoded.data as Record<string, unknown>;
      expect(data.density).toBe(DensityLevel.Ethereal);
      expect(data.metadata).toEqual({ name: 'Test' });
      expect(data.owner).toMatch(/^[0-9a-f]{64}$/);
      expect(encoded.signature).toMatch(/^[0-9a-f]{128}$/);
    });

    it('MistbornAsset Condense: encodes data and action', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('condense');
      const tx = MistbornAsset.createCondenseTransaction(
        assetId,
        owner,
        DensityLevel.Dense,
        { extra: 'meta' },
        []
      );
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('MistbornAsset');
      expect(encoded.action).toBe(AssetAction.Condense);
      expect(encoded.asset_id).toMatch(/^[0-9a-f]{64}$/);
      const data = encoded.data as Record<string, unknown>;
      expect(data.density).toBe(DensityLevel.Dense);
      expect(data.metadata).toEqual({ extra: 'meta' });
    });

    it('MistbornAsset Evaporate: encodes data and action', () => {
      const owner = bytes32(1);
      const assetId = MistbornAsset.createAssetId('evaporate');
      const tx = MistbornAsset.createEvaporateTransaction(
        assetId,
        owner,
        DensityLevel.Ethereal
      );
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.type).toBe('MistbornAsset');
      expect(encoded.action).toBe(AssetAction.Evaporate);
      expect((encoded.data as Record<string, unknown>).density).toBe(DensityLevel.Ethereal);
    });

    it('bigint amounts are stringified without loss', () => {
      const bigAmount = 18446744073709551615n;
      const tx = TransactionBuilder.createTransfer(
        bytes32(1),
        bytes32(2),
        bigAmount,
        0n,
        0
      );
      const encoded = encodeTransactionForApi(tx);
      expect(encoded.amount).toBe('18446744073709551615');
    });
  });
});
