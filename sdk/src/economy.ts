/**
 * Fog Economy - Economic systems for HAZE
 */

import { HazeClient } from './client';
import { LiquidityPool } from './types';

/**
 * Fog Economy client
 */
export class FogEconomy {
  private client: HazeClient;

  constructor(client: HazeClient) {
    this.client = client;
  }

  /**
   * Get all liquidity pools
   */
  async getLiquidityPools(): Promise<LiquidityPool[]> {
    return await this.client.getLiquidityPools();
  }

  /**
   * Get liquidity pool by ID
   */
  async getLiquidityPool(poolId: string): Promise<LiquidityPool> {
    return await this.client.getLiquidityPool(poolId);
  }

  /**
   * Create a new liquidity pool
   */
  async createLiquidityPool(
    asset1: string,
    asset2: string,
    reserve1: bigint,
    reserve2: bigint,
    feeRate: number
  ): Promise<string> {
    const result = await this.client.createLiquidityPool(
      asset1,
      asset2,
      reserve1,
      reserve2,
      feeRate
    );
    return result.pool_id;
  }

  /**
   * Calculate swap amount using constant product formula (x * y = k)
   */
  calculateSwapAmount(
    pool: LiquidityPool,
    inputAmount: bigint,
    isAsset1Input: boolean
  ): bigint {
    const k = pool.reserve1 * pool.reserve2;
    
    if (isAsset1Input) {
      // Swapping asset1 for asset2
      const newReserve1 = pool.reserve1 + inputAmount;
      const newReserve2 = k / newReserve1;
      const outputAmount = pool.reserve2 - newReserve2;
      
      // Apply fee (fee_rate is in basis points)
      const fee = (outputAmount * BigInt(pool.fee_rate)) / BigInt(10000);
      return outputAmount - fee;
    } else {
      // Swapping asset2 for asset1
      const newReserve2 = pool.reserve2 + inputAmount;
      const newReserve1 = k / newReserve2;
      const outputAmount = pool.reserve1 - newReserve1;
      
      // Apply fee
      const fee = (outputAmount * BigInt(pool.fee_rate)) / BigInt(10000);
      return outputAmount - fee;
    }
  }

  /**
   * Calculate liquidity shares for adding liquidity
   */
  calculateLiquidityShares(
    pool: LiquidityPool,
    reserve1Amount: bigint,
    reserve2Amount: bigint
  ): bigint {
    if (pool.total_liquidity === 0n) {
      // First liquidity provider gets geometric mean
      return this.geometricMean(reserve1Amount, reserve2Amount);
    }

    // Calculate shares based on proportional contribution
    const share1 = (reserve1Amount * pool.total_liquidity) / pool.reserve1;
    const share2 = (reserve2Amount * pool.total_liquidity) / pool.reserve2;
    
    // Return minimum to maintain ratio
    return share1 < share2 ? share1 : share2;
  }

  /**
   * Geometric mean: sqrt(a * b)
   */
  private geometricMean(a: bigint, b: bigint): bigint {
    const product = a * b;
    // Simple integer square root approximation
    let x = product;
    let y = (x + 1n) / 2n;
    while (y < x) {
      x = y;
      y = (x + product / x) / 2n;
    }
    return x;
  }
}
