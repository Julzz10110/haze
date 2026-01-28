/**
 * Transaction builder and utilities
 */

import { KeyPair, signTransaction } from './crypto';
import { Transaction, TransferTransaction, StakeTransaction, Address } from './types';
import { sha256 } from './utils';

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
