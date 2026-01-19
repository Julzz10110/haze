/**
 * Cryptographic functions for HAZE SDK
 * Uses Ed25519 for signing (compatible with Rust ed25519-dalek)
 */

import * as ed25519 from '@noble/ed25519';
import { Address, Hash, Transaction, AssetAction, DensityLevel } from './types';
import { bytesToHex, hexToBytes } from './utils';

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
    // Rust node treats Address as the 32-byte ED25519 public key.
    return this.publicKey;
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
  transaction: Transaction,
  keyPair: KeyPair
): Promise<Uint8Array> {
  const message = getTransactionDataForSigning(transaction);
  return await keyPair.sign(message);
}

function u64le(value: bigint): Uint8Array {
  if (value < 0n || value > 18446744073709551615n) {
    throw new Error('Value out of u64 range');
  }
  const out = new Uint8Array(8);
  let v = value;
  for (let i = 0; i < 8; i++) {
    out[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return out;
}

function concatBytes(chunks: Uint8Array[]): Uint8Array {
  const len = chunks.reduce((sum, c) => sum + c.length, 0);
  const out = new Uint8Array(len);
  let offset = 0;
  for (const c of chunks) {
    out.set(c, offset);
    offset += c.length;
  }
  return out;
}

function actionToByte(action: AssetAction): number {
  switch (action) {
    case AssetAction.Create: return 0;
    case AssetAction.Update: return 1;
    case AssetAction.Condense: return 2;
    case AssetAction.Evaporate: return 3;
    case AssetAction.Merge: return 4;
    case AssetAction.Split: return 5;
  }
}

function densityToByte(density: DensityLevel): number {
  switch (density) {
    case DensityLevel.Ethereal: return 0;
    case DensityLevel.Light: return 1;
    case DensityLevel.Dense: return 2;
    case DensityLevel.Core: return 3;
  }
}

/**
 * Must exactly match Rust `ConsensusEngine::get_transaction_data_for_signing`.
 */
export function getTransactionDataForSigning(tx: Transaction): Uint8Array {
  const enc = new TextEncoder();

  switch (tx.type) {
    case 'Transfer': {
      return concatBytes([
        enc.encode('Transfer'),
        tx.from,
        tx.to,
        u64le(tx.amount),
        u64le(tx.fee),
        u64le(BigInt(tx.nonce)),
      ]);
    }
    case 'Stake': {
      return concatBytes([
        enc.encode('Stake'),
        tx.validator,
        u64le(tx.amount),
      ]);
    }
    case 'ContractCall': {
      return concatBytes([
        enc.encode('ContractCall'),
        tx.contract,
        enc.encode(tx.method),
        new Uint8Array([0]),
        u64le(tx.gas_limit),
        tx.args,
      ]);
    }
    case 'MistbornAsset': {
      return concatBytes([
        enc.encode('MistbornAsset'),
        new Uint8Array([actionToByte(tx.action)]),
        tx.asset_id,
        tx.data.owner,
        new Uint8Array([densityToByte(tx.data.density)]),
      ]);
    }
  }
}
