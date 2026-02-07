# Fog Economy (Unity)

Client for HAZE Fog Economy: liquidity pools and client-side swap quotes.

## Usage

```csharp
using Haze;
using Haze.Economy;

var client = new HazeClient("http://localhost:8080");
var economy = new FogEconomy(client);

// List pools
var pools = await economy.GetPoolsAsync();

// Get one pool
var pool = await economy.GetPoolAsync("pool:asset1:asset2");

// Create pool (asset IDs, initial reserves, fee in basis points)
var poolId = await economy.CreatePoolAsync(
    "asset1-hex-or-id",
    "asset2-hex-or-id",
    reserve1: 1_000_000,
    reserve2: 2_000_000,
    feeRate: 30); // 0.3%

// Swap quote (client-side only; no REST swap endpoint yet)
ulong output = FogEconomy.ComputeSwapOutput(pool, inputAmount: 1000, isAsset1Input: true);

// Liquidity shares for adding liquidity
ulong shares = FogEconomy.ComputeLiquidityShares(pool, amount1, amount2);
```

## Swap formula

Constant product: `k = reserve1 * reserve2`. Fee is applied to **output** (basis points), matching the TypeScript SDK. Node swap execution is not exposed via REST; this API is for quotes only.

## Types

- `LiquidityPool`: pool_id, asset1, asset2, reserve1, reserve2, fee_rate, total_liquidity (strings for large numbers).
