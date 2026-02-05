using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using UnityEngine;

namespace Haze
{
    /// <summary>
    /// HAZE Blockchain API Client
    /// </summary>
    public class HazeClient
    {
        private readonly HttpClient _httpClient;
        private readonly string _baseUrl;

        public HazeClient(string baseUrl, int timeoutSeconds = 30)
        {
            _baseUrl = baseUrl.TrimEnd('/');
            _httpClient = new HttpClient
            {
                BaseAddress = new Uri(_baseUrl),
                Timeout = TimeSpan.FromSeconds(timeoutSeconds)
            };
            _httpClient.DefaultRequestHeaders.Add("Content-Type", "application/json");
        }

        /// <summary>
        /// Get health status
        /// </summary>
        public async Task<string> HealthCheckAsync()
        {
            var response = await GetAsync<ApiResponse<string>>("/health");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Health check failed");
            return response.data;
        }

        /// <summary>
        /// Get blockchain information
        /// </summary>
        public async Task<BlockchainInfo> GetBlockchainInfoAsync()
        {
            var response = await GetAsync<ApiResponse<BlockchainInfo>>("/api/v1/blockchain/info");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to get blockchain info");
            return response.data;
        }

        /// <summary>
        /// Get account information
        /// </summary>
        public async Task<AccountInfo> GetAccountAsync(string address)
        {
            var response = await GetAsync<ApiResponse<AccountInfo>>($"/api/v1/accounts/{address}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Account not found");
            return response.data;
        }

        /// <summary>
        /// Get account balance
        /// </summary>
        public async Task<string> GetBalanceAsync(string address)
        {
            var response = await GetAsync<ApiResponse<string>>($"/api/v1/accounts/{address}/balance");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to get balance");
            return response.data;
        }

        /// <summary>
        /// Send a transaction
        /// </summary>
        public async Task<TransactionResponse> SendTransactionAsync(object transaction)
        {
            var settings = new JsonSerializerSettings
            {
                Converters = { new TransactionJsonConverter() },
                NullValueHandling = NullValueHandling.Ignore
            };
            var transactionJson = JsonConvert.SerializeObject(transaction, settings);
            var transactionDict = JsonConvert.DeserializeObject<Dictionary<string, object>>(transactionJson);
            
            var requestBody = new Dictionary<string, object>
            {
                ["transaction"] = transactionDict
            };
            var json = JsonConvert.SerializeObject(requestBody);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await PostAsync<ApiResponse<TransactionResponse>>("/api/v1/transactions", content);
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to send transaction");
            return response.data;
        }

        /// <summary>
        /// Get transaction by hash
        /// </summary>
        public async Task<TransactionResponse> GetTransactionAsync(string hash)
        {
            var response = await GetAsync<ApiResponse<TransactionResponse>>($"/api/v1/transactions/{hash}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Transaction not found");
            return response.data;
        }

        /// <summary>
        /// Get block by hash
        /// </summary>
        public async Task<BlockInfo> GetBlockByHashAsync(string hash)
        {
            var response = await GetAsync<ApiResponse<BlockInfo>>($"/api/v1/blocks/{hash}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Block not found");
            return response.data;
        }

        /// <summary>
        /// Get block by height
        /// </summary>
        public async Task<BlockInfo> GetBlockByHeightAsync(int height)
        {
            var response = await GetAsync<ApiResponse<BlockInfo>>($"/api/v1/blocks/height/{height}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Block not found");
            return response.data;
        }

        /// <summary>
        /// Get asset information
        /// </summary>
        public async Task<AssetInfo> GetAssetAsync(string assetId)
        {
            var response = await GetAsync<ApiResponse<AssetInfo>>($"/api/v1/assets/{assetId}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Asset not found");
            return response.data;
        }

        /// <summary>
        /// Search assets by owner and/or game_id
        /// </summary>
        public async Task<List<AssetInfo>> SearchAssetsAsync(string owner = null, string gameId = null)
        {
            var queryParams = new List<string>();
            if (!string.IsNullOrEmpty(owner))
                queryParams.Add($"owner={Uri.EscapeDataString(owner)}");
            if (!string.IsNullOrEmpty(gameId))
                queryParams.Add($"game_id={Uri.EscapeDataString(gameId)}");

            var query = queryParams.Count > 0 ? "?" + string.Join("&", queryParams) : "";
            var response = await GetAsync<ApiResponse<List<AssetInfo>>>($"/api/v1/assets/search{query}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to search assets");
            return response.data;
        }

        /// <summary>
        /// Create asset (POST /api/v1/assets)
        /// </summary>
        public async Task<TransactionResponse> CreateAssetAsync(MistbornAssetTransaction transaction)
        {
            var settings = new JsonSerializerSettings
            {
                Converters = { new TransactionJsonConverter() },
                NullValueHandling = NullValueHandling.Ignore
            };
            var transactionJson = JsonConvert.SerializeObject(transaction, settings);
            var transactionDict = JsonConvert.DeserializeObject<Dictionary<string, object>>(transactionJson);
            
            var requestBody = new Dictionary<string, object>
            {
                ["transaction"] = transactionDict
            };
            var json = JsonConvert.SerializeObject(requestBody);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await PostAsync<ApiResponse<TransactionResponse>>("/api/v1/assets", content);
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to create asset");
            return response.data;
        }

        /// <summary>
        /// Estimate gas for asset transaction
        /// </summary>
        public async Task<GasEstimate> EstimateGasAsync(MistbornAssetTransaction transaction)
        {
            var settings = new JsonSerializerSettings
            {
                Converters = { new TransactionJsonConverter() },
                NullValueHandling = NullValueHandling.Ignore
            };
            var transactionJson = JsonConvert.SerializeObject(transaction, settings);
            var transactionDict = JsonConvert.DeserializeObject<Dictionary<string, object>>(transactionJson);
            
            var requestBody = new Dictionary<string, object>
            {
                ["transaction"] = transactionDict
            };
            var json = JsonConvert.SerializeObject(requestBody);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await PostAsync<ApiResponse<GasEstimate>>("/api/v1/assets/estimate-gas", content);
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to estimate gas");
            return response.data;
        }

        /// <summary>
        /// Get all liquidity pools
        /// </summary>
        public async Task<List<LiquidityPool>> GetLiquidityPoolsAsync()
        {
            var response = await GetAsync<ApiResponse<List<LiquidityPool>>>("/api/v1/economy/pools");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to get liquidity pools");
            return response.data;
        }

        /// <summary>
        /// Get liquidity pool by ID
        /// </summary>
        public async Task<LiquidityPool> GetLiquidityPoolAsync(string poolId)
        {
            var response = await GetAsync<ApiResponse<LiquidityPool>>($"/api/v1/economy/pools/{poolId}");
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Pool not found");
            return response.data;
        }

        /// <summary>
        /// Create liquidity pool
        /// </summary>
        public async Task<CreatePoolResponse> CreateLiquidityPoolAsync(string asset1, string asset2, string reserve1, string reserve2, int feeRate)
        {
            var requestBody = new
            {
                asset1,
                asset2,
                reserve1,
                reserve2,
                fee_rate = feeRate
            };
            var json = JsonConvert.SerializeObject(requestBody);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await PostAsync<ApiResponse<CreatePoolResponse>>("/api/v1/economy/pools", content);
            if (!response.success || response.data == null)
                throw new Exception(response.error ?? "Failed to create pool");
            return response.data;
        }

        private async Task<T> GetAsync<T>(string path)
        {
            try
            {
                var response = await _httpClient.GetAsync(path);
                var content = await response.Content.ReadAsStringAsync();
                if (!response.IsSuccessStatusCode)
                {
                    throw new Exception($"HTTP {response.StatusCode}: {content}");
                }
                return JsonConvert.DeserializeObject<T>(content);
            }
            catch (Exception ex)
            {
                throw new Exception($"Request failed: {ex.Message}", ex);
            }
        }

        private async Task<T> PostAsync<T>(string path, HttpContent content)
        {
            try
            {
                var response = await _httpClient.PostAsync(path, content);
                var responseContent = await response.Content.ReadAsStringAsync();
                if (!response.IsSuccessStatusCode)
                {
                    throw new Exception($"HTTP {response.StatusCode}: {responseContent}");
                }
                return JsonConvert.DeserializeObject<T>(responseContent);
            }
            catch (Exception ex)
            {
                throw new Exception($"Request failed: {ex.Message}", ex);
            }
        }


        public void Dispose()
        {
            _httpClient?.Dispose();
        }
    }

    [Serializable]
    public class GasEstimate
    {
        public string gas_cost;
        public string fee;
    }

    [Serializable]
    public class CreatePoolResponse
    {
        public string pool_id;
    }
}
