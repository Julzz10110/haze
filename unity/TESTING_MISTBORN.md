# Testing Mistborn API

## Quick Test (No Node Required)

### Step 1: Add Test Script

1. Create empty GameObject in Unity
2. Add `MistbornApiTest` component (from `Tests/MistbornApiTest.cs`)
3. Set `testWithoutNode = true` in Inspector

### Step 2: Run Tests

1. Right-click component → **"Run All Mistborn Tests"**
2. Check Console for output

**Expected Output:**
```
=== Mistborn API Tests ===

[Test] Density Limits
✓ Ethereal max: 5120 bytes (5 KB)
✓ Light max: 51200 bytes (50 KB)
✓ Dense max: 5242880 bytes (5 MB)
✓ Core max: 52428800 bytes (50 MB+)
✓ GetMaxSize works correctly
✓ Small metadata fits Ethereal
✓ Large metadata correctly rejected for Ethereal
✓ Large metadata fits Dense
✓ Recommended for 1 KB: Ethereal (should be Ethereal)
✓ Recommended for 100 KB: Dense (should be Dense)
✓ Recommended for 10 MB: Core (should be Core)
✓ GetRecommendedDensity works correctly

[Test] Mistborn API Helpers
✓ CreateAssetId produces consistent results
✓ AssetIdToHex works: abc123def456...
✓ HexToAssetId works correctly
✓ Different seeds produce different asset IDs
✓ CreateAssetId from bytes works: 789abc012def...

[Skipped] Node-dependent tests (testWithoutNode = true)

=== All Mistborn Tests Completed ===
```

## Full Test (With Node)

### Step 1: Start Node

```bash
cd /path/to/haze
cargo run
```

Verify: `curl http://localhost:8080/health`

### Step 2: Run Full Tests

1. Add `MistbornApiTest` component
2. Set `testWithoutNode = false`
3. Set `nodeUrl = "http://localhost:8080"`
4. Right-click → **"Run All Mistborn Tests"**

**Additional Tests:**
- Asset creation (requires balance for gas)
- Asset update
- Asset search by owner
- Asset search by game ID

## Test Individual Components

### Test Density Limits Only

```csharp
// Right-click component → "Test Density Limits Only"
// Or use code:
var maxSize = DensityLimits.GetMaxSize(DensityLevel.Ethereal);
Debug.Log($"Ethereal max: {maxSize}");
```

### Test Mistborn Helpers Only

```csharp
// Right-click component → "Test Mistborn Helpers Only"
// Or use code:
var assetId = MistbornAsset.CreateAssetId("test-seed");
var hex = MistbornAsset.AssetIdToHex(assetId);
Debug.Log($"Asset ID: {hex}");
```

### Test Asset Operations Only

```csharp
// Right-click component → "Test Asset Operations Only"
// Requires running node
```

## Test Simple Example

### Option 1: Use MistbornSimpleExample

1. Add `MistbornSimpleExample` component to GameObject
2. Set `nodeUrl` (if testing with node)
3. Press Play
4. Check Console for output

**Expected Flow:**
- Generate key pair
- Get balance
- Create asset
- Search assets
- Get asset details
- Update asset (if balance available)

### Option 2: Use MistbornSampleScene

1. Create Unity scene with UI Canvas
2. Add UI elements (see `Samples~/Mistborn/README.md`)
3. Add `MistbornSampleScene` component
4. Assign UI references in Inspector
5. Press Play

## What Gets Tested

### ✅ Density Limits
- GetMaxSize for all density levels
- IsWithinLimit validation
- GetRecommendedDensity logic

### ✅ Mistborn API Helpers
- CreateAssetId (from string and bytes)
- AssetIdToHex conversion
- HexToAssetId restoration
- Consistency (same seed = same ID)

### ✅ Asset Operations (requires node)
- Create asset
- Estimate gas
- Get asset info
- Update asset metadata
- Search by owner
- Search by game ID

## Troubleshooting

### "MistbornAsset not found"
- Ensure `using Haze.Mistborn;` is present
- Check that MistbornAsset.cs is in Runtime/Haze/Mistborn/

### "DensityLimits not found"
- Ensure `using Haze.Mistborn;` is present
- Check that DensityLimits.cs exists

### "Asset creation failed"
- Verify node is running
- Check account has balance for gas fees
- Verify nonce is correct (get from account info)

### "Search returns empty"
- Normal for new accounts
- Create an asset first, then search
- Wait a moment after creation before searching

## Next Steps

After tests pass:
- Test Condense/Evaporate operations
- Test Merge/Split operations
- Create custom UI using Mistborn API
- Integrate into game project
