/**
 * Tests for TransactionBuilder (createTransfer, createStake, sign)
 */
import { describe, it, expect } from 'vitest';
import { TransactionBuilder } from './transaction';
import { KeyPair } from './crypto';

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
});
