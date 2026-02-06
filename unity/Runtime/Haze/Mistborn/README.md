# Mistborn API Documentation

High-level C# API for Mistborn asset operations in Unity.

## Overview

The `MistbornAsset` class provides a convenient wrapper around Mistborn operations, handling transaction building, signing, and submission.

## Usage

### Initialization

```csharp
using Haze;
using Haze.Crypto;
using Haze.Mistborn;

var client = new HazeClient("http://localhost:8080");
var keyPair = KeyPair.Generate();
var mistborn = new MistbornAsset(client, keyPair);
```

### Create Asset

```csharp
var assetId = MistbornAsset.CreateAssetId("my-unique-seed");
var metadata = new Dictionary<string, string>
{
    ["name"] = "My NFT",
    ["description"] = "A test asset"
};

var response = await mistborn.CreateAsync(
    assetId: assetId,
    density: DensityLevel.Ethereal,
    metadata: metadata,
    gameId: "my-game-id"
);

Debug.Log($"Asset created! Hash: {response.hash}");
```

### Update Asset

```csharp
var updatedMetadata = new Dictionary<string, string>
{
    ["name"] = "Updated Name",
    ["level"] = "10"
};

await mistborn.UpdateAsync(
    assetId: assetId,
    metadata: updatedMetadata
);
```

### Condense (Increase Density)

```csharp
await mistborn.CondenseAsync(
    assetId: assetId,
    newDensity: DensityLevel.Light,
    additionalMetadata: new Dictionary<string, string>
    {
        ["upgraded"] = "true"
    }
);
```

### Evaporate (Decrease Density)

```csharp
await mistborn.EvaporateAsync(
    assetId: assetId,
    newDensity: DensityLevel.Ethereal
);
```

### Merge Assets

```csharp
var otherAssetId = MistbornAsset.CreateAssetId("other-seed");
await mistborn.MergeAsync(
    assetId: assetId,
    otherAssetId: otherAssetId
);
```

### Split Asset

```csharp
var componentIds = new List<string>
{
    MistbornAsset.AssetIdToHex(MistbornAsset.CreateAssetId("component1")),
    MistbornAsset.AssetIdToHex(MistbornAsset.CreateAssetId("component2"))
};

await mistborn.SplitAsync(
    assetId: assetId,
    componentIds: componentIds
);
```

### Search Assets

```csharp
// Search by owner (current key pair)
var myAssets = await mistborn.SearchByOwnerAsync();

// Search by game ID
var gameAssets = await mistborn.SearchByGameIdAsync("my-game-id");

// Search by owner and game ID
var filteredAssets = await mistborn.SearchAsync(gameId: "my-game-id");
```

### Get Asset Info

```csharp
var assetInfo = await mistborn.GetAssetAsync(assetId);
Debug.Log($"Asset density: {assetInfo.density}");
Debug.Log($"Owner: {assetInfo.owner}");
```

### Estimate Gas

```csharp
var tx = TransactionBuilder.CreateAsset(...);
var estimate = await mistborn.EstimateGasAsync(tx);
Debug.Log($"Gas cost: {estimate.gas_cost}, Fee: {estimate.fee}");
```

## Density Levels

Use `DensityLimits` helper to check limits:

```csharp
using Haze.Mistborn;

// Get max size for density
var maxSize = DensityLimits.GetMaxSize(DensityLevel.Ethereal); // 5120 bytes

// Check if metadata fits
var fits = DensityLimits.IsWithinLimit(DensityLevel.Ethereal, metadataSize);

// Get recommended density
var recommended = DensityLimits.GetRecommendedDensity(metadataSize);
```

## Density Limits

| Density | Max Size | Use Case |
|---------|----------|----------|
| Ethereal | 5 KB | Basic metadata, simple NFTs |
| Light | 50 KB | Attributes + textures |
| Dense | 5 MB | Full set + 3D model |
| Core | 50 MB+ | All data + history |

## Error Handling

All methods throw exceptions on failure:

```csharp
try
{
    await mistborn.CreateAsync(...);
}
catch (Exception ex)
{
    Debug.LogError($"Failed to create asset: {ex.Message}");
}
```

## See Also

- [MISTBORN_GUIDE.md](../../../docs/MISTBORN_GUIDE.md) - General Mistborn guide
- [PERFORMANCE.md](../../../docs/PERFORMANCE.md) - Gas and limits documentation
- Sample scene: `Samples~/Mistborn/MistbornSampleScene.cs`
