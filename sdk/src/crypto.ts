/**
 * Cryptographic functions for HAZE SDK
 * Uses Ed25519 for signing (compatible with Rust ed25519-dalek)
 */

import * as ed25519 from '@noble/ed25519';
import { Address, Hash } from './types';
import { sha256, bytesToHex, hexToBytes } from './utils';

/**
 * Key pair for signing transactions
 */
export class KeyPair {
  private privateKey: Uint8Array;
  public publicKey: Uint8Array;

  private constructor(privateKey: Uint8Array, publicKey: Uint8Array) {
    this.privateKey = privateKey;
    this.publicKey = publicKey;
  }

  /**
   * Generate a new key pair
   */
  static async generate(): Promise<KeyPair> {
    const privateKey = ed25519.utils.randomPrivateKey();
    const publicKey = await ed25519.getPublicKey(privateKey);
    return new KeyPair(privateKey, publicKey);
  }

  /**
   * Create key pair from private key (hex string or bytes)
   */
  static async fromPrivateKey(privateKey: string | Uint8Array): Promise<KeyPair> {
    const privKey = typeof privateKey === 'string' 
      ? hexToBytes(privateKey) 
      : privateKey;
    
    if (privKey.length !== 32) {
      throw new Error('Private key must be 32 bytes');
    }

    const publicKey = await ed25519.getPublicKey(privKey);
    return new KeyPair(privKey, publicKey);
  }

  /**
   * Get address from public key (SHA256 of public key)
   */
  getAddress(): Address {
    return sha256(this.publicKey);
  }

  /**
   * Get address as hex string
   */
  getAddressHex(): string {
    return bytesToHex(this.getAddress());
  }

  /**
   * Get public key as hex string
   */
  getPublicKeyHex(): string {
    return bytesToHex(this.publicKey);
  }

  /**
   * Get private key as hex string (use with caution!)
   */
  getPrivateKeyHex(): string {
    return bytesToHex(this.privateKey);
  }

  /**
   * Sign a message
   */
  async sign(message: Uint8Array): Promise<Uint8Array> {
    return await ed25519.sign(message, this.privateKey);
  }

  /**
   * Sign a hash
   */
  async signHash(hash: Hash): Promise<Uint8Array> {
    return await this.sign(hash);
  }

  /**
   * Verify a signature
   */
  static async verify(
    message: Uint8Array,
    signature: Uint8Array,
    publicKey: Uint8Array
  ): Promise<boolean> {
    try {
      return await ed25519.verify(signature, message, publicKey);
    } catch {
      return false;
    }
  }

  /**
   * Verify a signature for a hash
   */
  static async verifyHash(
    hash: Hash,
    signature: Uint8Array,
    publicKey: Uint8Array
  ): Promise<boolean> {
    return await this.verify(hash, signature, publicKey);
  }
}

/**
 * Sign transaction data
 * Transaction is serialized to bytes, hashed, and then signed
 */
export async function signTransaction(
  transaction: any,
  keyPair: KeyPair
): Promise<Uint8Array> {
  // Serialize transaction (excluding signature field)
  const txData = JSON.stringify(transaction, (key, value) => {
    // Skip signature field when serializing
    if (key === 'signature') {
      return undefined;
    }
    // Convert bigint to string for JSON
    if (typeof value === 'bigint') {
      return value.toString();
    }
    // Convert Uint8Array to hex
    if (value instanceof Uint8Array) {
      return bytesToHex(value);
    }
    return value;
  });

  // Hash the serialized transaction
  const hash = sha256(Buffer.from(txData, 'utf-8'));

  // Sign the hash
  return await keyPair.signHash(hash);
}
