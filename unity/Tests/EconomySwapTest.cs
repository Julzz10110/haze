using System;
using Haze;
using Haze.Economy;
using UnityEngine;

namespace Haze.Tests
{
    /// <summary>
    /// Verifies swap calculation matches constant-product formula (fee on output).
    /// Run without a node.
    /// </summary>
    public class EconomySwapTest : MonoBehaviour
    {
        [ContextMenu("Run Swap Calculation Test")]
        public void RunSwapTest()
        {
            Debug.Log("=== Economy Swap Calculation Test ===");

            var pool = new LiquidityPool
            {
                pool_id = "pool:test:a:b",
                asset1 = "asset1",
                asset2 = "asset2",
                reserve1 = "10000",
                reserve2 = "20000",
                fee_rate = 30,
                total_liquidity = "14142"
            };

            // Swap 1000 asset1 -> asset2
            // k = 10000 * 20000 = 200_000_000
            // newReserve1 = 11000, newReserve2 = 200_000_000 / 11000 = 18181, output = 20000 - 18181 = 1819
            // fee = 1819 * 30 / 10000 = 5, result = 1814
            ulong output = FogEconomy.ComputeSwapOutput(pool, 1000, isAsset1Input: true);
            const ulong expected = 1814;
            if (output == expected)
            {
                Debug.Log($"✓ Swap output correct: {output} (expected {expected})");
            }
            else
            {
                Debug.LogError($"✗ Swap output: {output}, expected {expected}");
            }

            // Reverse: 1000 asset2 -> asset1
            ulong outputRev = FogEconomy.ComputeSwapOutput(pool, 1000, isAsset1Input: false);
            // newReserve2 = 21000, newReserve1 = 200_000_000 / 21000 = 9523, output = 10000 - 9523 = 477
            // fee = 477 * 30 / 10000 = 1, result = 476
            const ulong expectedRev = 476;
            if (outputRev == expectedRev)
            {
                Debug.Log($"✓ Swap (asset2 in) output correct: {outputRev} (expected {expectedRev})");
            }
            else
            {
                Debug.LogError($"✗ Swap (asset2 in) output: {outputRev}, expected {expectedRev}");
            }

            // Liquidity shares: first provider geometric mean
            ulong shares = FogEconomy.ComputeLiquidityShares(pool, 10000, 20000);
            // total_liquidity is 14142 ≈ sqrt(10000*20000). So first provider gets sqrt(10000*20000) = 14142
            if (shares > 0 && shares <= 15000)
            {
                Debug.Log($"✓ Liquidity shares (first provider): {shares}");
            }
            else
            {
                Debug.LogError($"✗ Unexpected liquidity shares: {shares}");
            }

            Debug.Log("=== Swap test done ===");
        }
    }
}
