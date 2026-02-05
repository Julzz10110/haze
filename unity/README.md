# HAZE Blockchain SDK for Unity

C# SDK for HAZE blockchain - Unity integration for Mistborn assets and Fog economy.

## Requirements

- Unity 2020.3 LTS or later
- .NET Standard 2.1 or .NET Framework 4.8
- [Chaos.NaCl](https://github.com/CodesInChaos/Chaos.NaCl) library for Ed25519 signing
- [Newtonsoft.Json](https://github.com/JamesNK/Newtonsoft.Json) (included via Unity Package Manager)

## Installation

### Option 1: Git URL (Unity Package Manager)

1. Open Unity Package Manager (Window → Package Manager)
2. Click "+" → "Add package from git URL"
3. Enter: `https://github.com/haze-blockchain/haze.git?path=unity`
4. Click "Add"

### Option 2: OpenUPM (recommended)

```bash
openupm add com.haze.blockchain
```

### Option 3: Manual

1. Download or clone this repository
2. Copy the `unity` folder into your Unity project's `Packages` folder
3. Install dependencies: Chaos.NaCl and Newtonsoft.Json

## Dependencies

This package requires:

- **Chaos.NaCl**: Ed25519 cryptography library
  - Install via NuGet or download from [GitHub](https://github.com/CodesInChaos/Chaos.NaCl)
  - Place `Chaos.NaCl.dll` in `Assets/Plugins/` or reference via .asmdef

- **Newtonsoft.Json**: JSON serialization (already included in Unity 2020.3+ via Package Manager)

## Quick Start

### 1. Configure the client

```csharp
using Haze.Client;
using Haze.Crypto;

// Create client with node URL
var client = new HazeClient("http://localhost:8080");
```

### 2. Generate a key pair

```csharp
// Generate new key pair
var keyPair = KeyPair.Generate();
var address = keyPair.GetAddressHex();
Debug.Log($"My address: {address}");

// Or restore from private key
var privateKeyHex = "your-64-char-hex-private-key";
var restoredKeyPair = KeyPair.FromPrivateKeyHex(privateKeyHex);
```

### 3. Get account balance

```csharp
try
{
    var balance = await client.GetBalanceAsync(address);
    Debug.Log($"Balance: {balance}");
}
catch (Exception ex)
{
    Debug.LogError($"Failed to get balance: {ex.Message}");
}
```

### 4. Send a transfer

```csharp
using Haze;

// Create transfer transaction
var recipient = new Address(Utils.HexToBytes("recipient-address-hex"));
var transfer = TransactionBuilder.CreateTransfer(
    from: keyPair.GetAddress(),
    to: recipient,
    amount: 1000000, // 1 HAZE (assuming 6 decimals)
    fee: 1000,
    nonce: 0
);

// Sign the transaction
var signedTransfer = TransactionBuilder.Sign(transfer, keyPair);

// Send to node
try
{
    var response = await client.SendTransactionAsync(signedTransfer);
    Debug.Log($"Transaction sent! Hash: {response.hash}");
}
catch (Exception ex)
{
    Debug.LogError($"Failed to send transaction: {ex.Message}");
}
```

### 5. Create a Mistborn asset

```csharp
// Create asset ID from seed
var assetId = TransactionBuilder.CreateAssetId("my-unique-seed-123");

// Create asset transaction
var assetTx = TransactionBuilder.CreateAsset(
    assetId: assetId,
    owner: keyPair.GetAddress(),
    density: DensityLevel.Ethereal,
    metadata: new Dictionary<string, string>
    {
        ["name"] = "My NFT",
        ["description"] = "A test asset"
    },
    gameId: "my-game-id"
);

// Sign and send
var signedAssetTx = TransactionBuilder.Sign(assetTx, keyPair);
var response = await client.CreateAssetAsync(signedAssetTx);
Debug.Log($"Asset created! Hash: {response.hash}");
```

### 6. Search assets

```csharp
// Search by owner
var myAssets = await client.SearchAssetsAsync(owner: keyPair.GetAddressHex());

// Search by game ID
var gameAssets = await client.SearchAssetsAsync(gameId: "my-game-id");

foreach (var asset in myAssets)
{
    Debug.Log($"Asset: {asset.asset_id}, Owner: {asset.owner}, Density: {asset.density}");
}
```

## API Reference

### HazeClient

Main HTTP client for HAZE blockchain API.

**Methods:**
- `HealthCheckAsync()` - Check node health
- `GetBlockchainInfoAsync()` - Get blockchain information
- `GetAccountAsync(address)` - Get account info
- `GetBalanceAsync(address)` - Get account balance
- `SendTransactionAsync(transaction)` - Send any transaction
- `GetTransactionAsync(hash)` - Get transaction by hash
- `GetBlockByHashAsync(hash)` - Get block by hash
- `GetBlockByHeightAsync(height)` - Get block by height
- `GetAssetAsync(assetId)` - Get asset info
- `SearchAssetsAsync(owner, gameId)` - Search assets
- `CreateAssetAsync(transaction)` - Create Mistborn asset
- `EstimateGasAsync(transaction)` - Estimate gas for asset transaction
- `GetLiquidityPoolsAsync()` - Get all liquidity pools
- `GetLiquidityPoolAsync(poolId)` - Get pool by ID
- `CreateLiquidityPoolAsync(...)` - Create liquidity pool

### KeyPair

Ed25519 key pair for signing transactions.

**Methods:**
- `Generate()` - Generate new key pair
- `FromPrivateKey(byte[])` - Restore from private key bytes
- `FromPrivateKeyHex(string)` - Restore from private key hex
- `GetAddress()` - Get address (32-byte public key)
- `GetAddressHex()` - Get address as hex string
- `Sign(byte[])` - Sign a message

### TransactionBuilder

Build and sign transactions.

**Transfer:**
- `CreateTransfer(...)` - Create transfer transaction
- `Sign(TransferTransaction, KeyPair)` - Sign transfer

**Mistborn Assets:**
- `CreateAssetId(string)` - Create asset ID from seed
- `CreateAsset(...)` - Create asset transaction
- `UpdateAsset(...)` - Update asset transaction
- `CondenseAsset(...)` - Condense (increase density)
- `EvaporateAsset(...)` - Evaporate (decrease density)
- `MergeAssets(...)` - Merge two assets
- `SplitAsset(...)` - Split asset
- `Sign(MistbornAssetTransaction, KeyPair)` - Sign asset transaction

## Examples

See `Samples~/` folder for complete examples:
- Basic usage (generate key, get balance, send transfer)
- Mistborn assets (create, search, update)
- Economy (pools, swap calculation)

## Notes

- All amounts (`amount`, `fee`) are strings in API to avoid precision loss (C# `ulong` is used internally, converted to string for JSON)
- Addresses and hashes are 32 bytes, sent as 64-character hex strings
- Transaction signing uses canonical payload format matching Rust node and TypeScript SDK
- For WebGL builds, ensure CORS is enabled on the HAZE node

## License

See main repository LICENSE file.

## Support

- Documentation: [docs/](https://github.com/haze-blockchain/haze/tree/main/docs)
- API Reference: [API_TRANSACTIONS.md](https://github.com/haze-blockchain/haze/blob/main/docs/API_TRANSACTIONS.md)
- Mistborn Guide: [MISTBORN_GUIDE.md](https://github.com/haze-blockchain/haze/blob/main/docs/MISTBORN_GUIDE.md)
