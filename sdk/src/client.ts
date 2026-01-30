/**
 * HTTP client for HAZE Blockchain REST API
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import {
  ApiResponse,
  BlockchainInfo,
  AccountInfo,
  BlockInfo,
  TransactionResponse,
  AssetInfo,
  LiquidityPool,
  Transaction,
} from './types';
import { encodeTransactionForApi } from './transaction';

export interface HazeClientConfig {
  baseUrl: string;
  timeout?: number;
}

/**
 * HAZE Blockchain API Client
 */
export class HazeClient {
  private axios: AxiosInstance;
  private baseUrl: string;

  constructor(config: HazeClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, ''); // Remove trailing slash
    this.axios = axios.create({
      baseURL: this.baseUrl,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    });
  }

  /**
   * Get health status
   */
  async healthCheck(): Promise<string> {
    const response = await this.axios.get<ApiResponse<string>>('/health');
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Health check failed');
    }
    return response.data.data;
  }

  /**
   * Get blockchain information
   */
  async getBlockchainInfo(): Promise<BlockchainInfo> {
    const response = await this.axios.get<ApiResponse<BlockchainInfo>>(
      '/api/v1/blockchain/info'
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Failed to get blockchain info');
    }
    // Convert string bigint to bigint
    const data = response.data.data;
    return {
      ...data,
      total_supply: BigInt(data.total_supply as any),
    };
  }

  /**
   * Send a transaction
   */
  async sendTransaction(transaction: Transaction): Promise<TransactionResponse> {
    try {
      const response = await this.axios.post<ApiResponse<TransactionResponse>>(
        '/api/v1/transactions',
        { transaction: encodeTransactionForApi(transaction) }
      );
      if (!response.data.success || !response.data.data) {
        throw new Error(response.data.error || 'Failed to send transaction');
      }
      return response.data.data;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        const axiosError = error as AxiosError<ApiResponse<any>>;
        if (axiosError.response?.data?.error) {
          throw new Error(axiosError.response.data.error);
        }
      }
      throw error;
    }
  }

  /**
   * Get transaction by hash
   */
  async getTransaction(hash: string): Promise<TransactionResponse> {
    const response = await this.axios.get<ApiResponse<TransactionResponse>>(
      `/api/v1/transactions/${hash}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Transaction not found');
    }
    return response.data.data;
  }

  /**
   * Get block by hash
   */
  async getBlockByHash(hash: string): Promise<BlockInfo> {
    const response = await this.axios.get<ApiResponse<BlockInfo>>(
      `/api/v1/blocks/${hash}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Block not found');
    }
    return response.data.data;
  }

  /**
   * Get block by height
   */
  async getBlockByHeight(height: number): Promise<BlockInfo> {
    const response = await this.axios.get<ApiResponse<BlockInfo>>(
      `/api/v1/blocks/height/${height}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Block not found');
    }
    return response.data.data;
  }

  /**
   * Get account information
   */
  async getAccount(address: string): Promise<AccountInfo> {
    const response = await this.axios.get<ApiResponse<AccountInfo>>(
      `/api/v1/accounts/${address}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Account not found');
    }
    // Convert string bigint to bigint
    const data = response.data.data;
    return {
      ...data,
      balance: BigInt(data.balance as any),
      staked: BigInt(data.staked as any),
    };
  }

  /**
   * Get account balance
   */
  async getBalance(address: string): Promise<bigint> {
    const response = await this.axios.get<ApiResponse<string>>(
      `/api/v1/accounts/${address}/balance`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Failed to get balance');
    }
    return BigInt(response.data.data);
  }

  /**
   * Get asset information
   */
  async getAsset(assetId: string): Promise<AssetInfo> {
    const response = await this.axios.get<ApiResponse<AssetInfo>>(
      `/api/v1/assets/${assetId}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Asset not found');
    }
    return response.data.data;
  }

  /**
   * Get all liquidity pools
   */
  async getLiquidityPools(): Promise<LiquidityPool[]> {
    const response = await this.axios.get<ApiResponse<LiquidityPool[]>>(
      '/api/v1/economy/pools'
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Failed to get liquidity pools');
    }
    // Convert string bigint to bigint
    return response.data.data.map(pool => ({
      ...pool,
      reserve1: BigInt(pool.reserve1 as any),
      reserve2: BigInt(pool.reserve2 as any),
      total_liquidity: BigInt(pool.total_liquidity as any),
    }));
  }

  /**
   * Get liquidity pool by ID
   */
  async getLiquidityPool(poolId: string): Promise<LiquidityPool> {
    const response = await this.axios.get<ApiResponse<LiquidityPool>>(
      `/api/v1/economy/pools/${poolId}`
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Liquidity pool not found');
    }
    // Convert string bigint to bigint
    const data = response.data.data;
    return {
      ...data,
      reserve1: BigInt(data.reserve1 as any),
      reserve2: BigInt(data.reserve2 as any),
      total_liquidity: BigInt(data.total_liquidity as any),
    };
  }

  /**
   * Create liquidity pool
   */
  async createLiquidityPool(
    asset1: string,
    asset2: string,
    reserve1: bigint,
    reserve2: bigint,
    feeRate: number
  ): Promise<{ pool_id: string; status: string }> {
    const response = await this.axios.post<ApiResponse<{ pool_id: string; status: string }>>(
      '/api/v1/economy/pools',
      {
        asset1,
        asset2,
        reserve1: reserve1.toString(),
        reserve2: reserve2.toString(),
        fee_rate: feeRate,
      }
    );
    if (!response.data.success || !response.data.data) {
      throw new Error(response.data.error || 'Failed to create liquidity pool');
    }
    return response.data.data;
  }

}
