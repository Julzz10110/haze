using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Haze;

namespace Haze.Economy
{
    /// <summary>
    /// Fog Economy client: liquidity pools and client-side swap quote.
    /// Matches TypeScript SDK constant-product formula (fee on output).
    /// </summary>
    public class FogEconomy
    {
        private readonly HazeClient _client;

        public FogEconomy(HazeClient client)
        {
            _client = client ?? throw new ArgumentNullException(nameof(client));
        }

        /// <summary>
        /// Get all liquidity pools
        /// </summary>
        public async Task<List<LiquidityPool>> GetPoolsAsync()
        {
            return await _client.GetLiquidityPoolsAsync();
        }

        /// <summary>
        /// Get liquidity pool by ID
        /// </summary>
        public async Task<LiquidityPool> GetPoolAsync(string poolId)
        {
            return await _client.GetLiquidityPoolAsync(poolId);
        }

        /// <summary>
        /// Create a new liquidity pool
        /// </summary>
        /// <param name="asset1">Asset 1 ID (hex or identifier)</param>
        /// <param name="asset2">Asset 2 ID (hex or identifier)</param>
        /// <param name="reserve1">Initial reserve for asset 1</param>
        /// <param name="reserve2">Initial reserve for asset 2</param>
        /// <param name="feeRate">Fee in basis points (e.g. 30 = 0.3%)</param>
        /// <returns>Pool ID of the created pool</returns>
        public async Task<string> CreatePoolAsync(string asset1, string asset2, ulong reserve1, ulong reserve2, int feeRate)
        {
            var result = await _client.CreateLiquidityPoolAsync(
                asset1,
                asset2,
                reserve1.ToString(),
                reserve2.ToString(),
                feeRate);
            return result.pool_id;
        }

        /// <summary>
        /// Calculate swap output using constant product (x * y = k).
        /// Fee is applied to output (matches TypeScript SDK).
        /// </summary>
        /// <param name="pool">Liquidity pool (reserve1, reserve2, fee_rate)</param>
        /// <param name="inputAmount">Amount to swap in</param>
        /// <param name="isAsset1Input">True if input is asset1, false if asset2</param>
        /// <returns>Output amount after fee (in the other asset)</returns>
        public static ulong ComputeSwapOutput(LiquidityPool pool, ulong inputAmount, bool isAsset1Input)
        {
            if (pool == null)
                throw new ArgumentNullException(nameof(pool));

            ulong r1 = ParseReserve(pool.reserve1);
            ulong r2 = ParseReserve(pool.reserve2);
            ulong k = r1 * r2;

            if (isAsset1Input)
            {
                // Swapping asset1 for asset2
                ulong newReserve1 = r1 + inputAmount;
                if (newReserve1 == 0) return 0;
                ulong newReserve2 = k / newReserve1;
                ulong outputAmount = r2 - newReserve2;
                ulong fee = (outputAmount * (uint)pool.fee_rate) / 10000u;
                return outputAmount - fee;
            }
            else
            {
                // Swapping asset2 for asset1
                ulong newReserve2 = r2 + inputAmount;
                if (newReserve2 == 0) return 0;
                ulong newReserve1 = k / newReserve2;
                ulong outputAmount = r1 - newReserve1;
                ulong fee = (outputAmount * (uint)pool.fee_rate) / 10000u;
                return outputAmount - fee;
            }
        }

        /// <summary>
        /// Calculate swap output using pool data with string reserves (from API)
        /// </summary>
        public static ulong ComputeSwapOutput(LiquidityPool pool, string inputAmountStr, bool isAsset1Input)
        {
            ulong inputAmount = ParseReserve(inputAmountStr);
            return ComputeSwapOutput(pool, inputAmount, isAsset1Input);
        }

        /// <summary>
        /// Calculate liquidity shares for adding liquidity (matches TypeScript SDK).
        /// First provider gets geometric mean; later providers get min of proportional shares.
        /// </summary>
        public static ulong ComputeLiquidityShares(LiquidityPool pool, ulong reserve1Amount, ulong reserve2Amount)
        {
            if (pool == null)
                throw new ArgumentNullException(nameof(pool));

            ulong totalLiq = ParseReserve(pool.total_liquidity);
            if (totalLiq == 0)
                return GeometricMean(reserve1Amount, reserve2Amount);

            ulong r1 = ParseReserve(pool.reserve1);
            ulong r2 = ParseReserve(pool.reserve2);
            ulong share1 = (reserve1Amount * totalLiq) / r1;
            ulong share2 = (reserve2Amount * totalLiq) / r2;
            return share1 < share2 ? share1 : share2;
        }

        private static ulong GeometricMean(ulong a, ulong b)
        {
            ulong product = a * b;
            if (product == 0) return 0;
            ulong x = product;
            ulong y = (x + 1) / 2;
            while (y < x)
            {
                x = y;
                y = (x + product / x) / 2;
            }
            return x;
        }

        private static ulong ParseReserve(string s)
        {
            if (string.IsNullOrEmpty(s)) return 0;
            if (ulong.TryParse(s, out var v)) return v;
            throw new FormatException($"Invalid reserve value: {s}");
        }
    }
}
