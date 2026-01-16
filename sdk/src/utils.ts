/**
 * Utility functions for HAZE SDK
 */

import { Hash, Address } from './types';
import { createHash } from 'crypto';

/**
 * Convert bytes to hex string
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('hex');
}

/**
 * Convert hex string to bytes
 */
export function hexToBytes(hex: string): Uint8Array {
  return new Uint8Array(Buffer.from(hex, 'hex'));
}

/**
 * Convert hex string to hash (32 bytes)
 */
export function hexToHash(hex: string): Hash | null {
  try {
    const bytes = hexToBytes(hex);
    if (bytes.length !== 32) {
      return null;
    }
    return bytes;
  } catch {
    return null;
  }
}

/**
 * Convert hash to hex string
 */
export function hashToHex(hash: Hash): string {
  return bytesToHex(hash);
}

/**
 * Convert address to hex string
 */
export function addressToHex(address: Address): string {
  return bytesToHex(address);
}

/**
 * Convert hex string to address (32 bytes)
 */
export function hexToAddress(hex: string): Address | null {
  try {
    const bytes = hexToBytes(hex);
    if (bytes.length !== 32) {
      return null;
    }
    return bytes;
  } catch {
    return null;
  }
}

/**
 * Compute SHA256 hash
 */
export function sha256(data: Uint8Array | string): Hash {
  const input = typeof data === 'string' ? Buffer.from(data, 'utf-8') : Buffer.from(data);
  return new Uint8Array(createHash('sha256').update(input).digest());
}

/**
 * Check if a value is a valid address (32 bytes)
 */
export function isValidAddress(value: Uint8Array): boolean {
  return value.length === 32;
}

/**
 * Check if a value is a valid hash (32 bytes)
 */
export function isValidHash(value: Uint8Array): boolean {
  return value.length === 32;
}

/**
 * Convert bigint to string (for JSON serialization)
 */
export function bigintToString(value: bigint): string {
  return value.toString();
}

/**
 * Convert string to bigint (from JSON)
 */
export function stringToBigint(value: string): bigint {
  return BigInt(value);
}

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
