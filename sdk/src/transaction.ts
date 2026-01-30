/**
 * Transaction builder and utilities
 */

import { KeyPair, signTransaction } from './crypto';
import { Transaction, TransferTransaction, StakeTransaction, Address } from './types';
import { sha256, bytesToHex } from './utils';

/**
 * Serialize a value for API JSON: Uint8Array -> hex, bigint -> string, nested objects recursively.
 */
function serializeForApi(value: unknown): unknown {
  if (value instanceof Uint8Array) {
    return bytesToHex(value);
  }
  if (typeof value === 'bigint') {
    return value.toString();
  }
  if (Array.isArray(value)) {
    return value.map(serializeForApi);
  }
  if (value !== null && typeof value === 'object') {
    const result: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      result[key] = serializeForApi(val);
    }
    return result;
  }
  return value;
}

/**
 * Encode transaction to API format (hex strings for bytes, decimal strings for bigint).
 * Use this when sending transactions to the REST API.
 */
export function encodeTransactionForApi(tx: Transaction): Record<string, unknown> {
  return serializeForApi(tx) as Record<string, unknown>;
}

/**
 * Transaction builder
 */
export class TransactionBuilder {
  /**
   * Create a transfer transaction
   */
  static createTransfer(
    from: Address,
    to: Address,
    amount: bigint,
    fee: bigint,
    nonce: number
  ): TransferTransaction {
    return {
      type: 'Transfer',
      from,
      to,
      amount,
      fee,
      nonce,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Create a stake transaction
   */
  static createStake(
    from: Address,
    validator: Address,
    amount: bigint,
    fee: bigint,
    nonce: number
  ): StakeTransaction {
    return {
      type: 'Stake',
      from,
      validator,
      amount,
      fee,
      nonce,
      signature: new Uint8Array(0), // Will be set when signing
    };
  }

  /**
   * Sign a transaction
   */
  static async sign(transaction: Transaction, keyPair: KeyPair): Promise<Transaction> {
    const signature = await signTransaction(transaction, keyPair);
    
    // Create a copy with signature
    if (transaction.type === 'Transfer') {
      return {
        ...transaction,
        signature,
      };
    } else if (transaction.type === 'Stake') {
      return {
        ...transaction,
        signature,
      };
    } else if (transaction.type === 'MistbornAsset') {
      return {
        ...transaction,
        signature,
      };
    } else if (transaction.type === 'ContractCall') {
      return {
        ...transaction,
        signature,
      };
    }
    
    return transaction;
  }

  /**
   * Get transaction hash
   */
  static getHash(transaction: Transaction): Uint8Array {
    // Serialize transaction (excluding signature for hash calculation)
    const txData = JSON.stringify(transaction, (key, value) => {
      if (key === 'signature') {
        return undefined;
      }
      if (typeof value === 'bigint') {
        return value.toString();
      }
      if (value instanceof Uint8Array) {
        return Array.from(value).map(b => b.toString(16).padStart(2, '0')).join('');
      }
      return value;
    });

    return sha256(Buffer.from(txData, 'utf-8'));
  }

  /**
   * Get transaction hash as hex string
   */
  static getHashHex(transaction: Transaction): string {
    const hash = this.getHash(transaction);
    return Buffer.from(hash).toString('hex');
  }
}
