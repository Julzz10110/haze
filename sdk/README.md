# HAZE Blockchain TypeScript SDK v0.1

TypeScript/JavaScript SDK for HAZE Blockchain - High-performance Asset Zone Engine for GameFi.

> "Where games breathe blockchain"

## Installation

```bash
npm install @haze/sdk
```

Or using yarn:

```bash
yarn add @haze/sdk
```

## Quick Start

```typescript
import { HazeClient, KeyPair, DEFAULT_API_URL } from '@haze/sdk';

// Initialize client
const client = new HazeClient({
  baseUrl: DEFAULT_API_URL, // or your HAZE node URL
});

// Generate key pair
const keyPair = await KeyPair.generate();
const address = keyPair.getAddressHex();

// Get blockchain info
const info = await client.getBlockchainInfo();
console.log('Current height:', info.current_height);
```

## Features

- **REST API Client** - Full access to HAZE blockchain API
- **Cryptography** - Ed25519 key generation and transaction signing
- **Mistborn Assets** - Dynamic NFT creation and management
- **Fog Economy** - Liquidity pools and economic operations
- **TypeScript Support** - Full type definitions included

## API Reference

### HazeClient

Main client for interacting with HAZE blockchain.

```typescript
const client = new HazeClient({
  baseUrl: 'http://localhost:8080',
  timeout: 30000, // optional
});

// Health check
await client.healthCheck();

// Get blockchain info
const info = await client.getBlockchainInfo();

// Get account balance
const balance = await client.getBalance(address);

// Send transaction
const result = await client.sendTransaction(transaction);
```

### KeyPair

Generate and manage cryptographic key pairs.

```typescript
// Generate new key pair
const keyPair = await KeyPair.generate();

// Get address
const address = keyPair.getAddressHex();

// Create from existing private key
const keyPair2 = await KeyPair.fromPrivateKey(privateKeyHex);

// Sign data
const signature = await keyPair.sign(message);
```

### Building and signing a transaction

Every user-signed transaction includes `from` (signer), `fee`, and `nonce`. The client signs the **canonical byte payload** (see node `get_transaction_data_for_signing`); the SDK’s `encodeTransaction` matches that format.

**Transfer:**

```typescript
const tx = TransactionBuilder.createTransfer(
  fromAddress,  // Uint8Array (32 bytes)
  toAddress,
  amount,       // bigint
  fee,          // bigint
  nonce        // number
);
const signedTx = await TransactionBuilder.sign(tx, keyPair);
await client.sendTransaction(signedTx);
```

**Mistborn asset (Create / Condense / etc.):**

```typescript
const assetTx = MistbornAsset.createCreateTransaction(
  assetId,
  ownerAddress,  // also used as `from` (signer)
  DensityLevel.Ethereal,
  { name: 'My NFT' },
  []
);
// Set fee/nonce if needed: assetTx.fee = 0n; assetTx.nonce = 0;
const signed = await MistbornAsset.sign(assetTx, keyPair);
await client.sendTransaction(signed);
```

**Stake:**

```typescript
const stakeTx = TransactionBuilder.createStake(
  fromAddress,
  validatorAddress,
  amount,
  fee,
  nonce
);
const signedTx = await TransactionBuilder.sign(stakeTx, keyPair);
await client.sendTransaction(signedTx);
```

The API expects JSON with byte fields as **hex strings**. See [API transaction contract](../docs/API_TRANSACTIONS.md) for the full request shape.

### TransactionBuilder

Build and sign transactions.

```typescript
// Create transfer transaction
const tx = TransactionBuilder.createTransfer(
  fromAddress,
  toAddress,
  amount, // bigint
  fee,    // bigint
  nonce   // number
);

// Sign transaction
const signedTx = await TransactionBuilder.sign(tx, keyPair);

// Get transaction hash
const hash = TransactionBuilder.getHashHex(signedTx);
```

### MistbornAsset

Work with dynamic NFTs.

```typescript
import { DensityLevel } from '@haze/sdk';

// Create asset ID
const assetId = MistbornAsset.createAssetId('unique_asset_id');

// Create asset transaction
const assetTx = MistbornAsset.createCreateTransaction(
  assetId,
  ownerAddress,
  DensityLevel.Ethereal,
  {
    name: 'Legendary Sword',
    rarity: 'legendary',
  },
  [
    { name: 'attack', value: '100', rarity: 0.95 },
  ],
  'game_id' // optional
);

// Sign asset transaction
const signedTx = await MistbornAsset.sign(assetTx, keyPair);

// Condense (increase density)
const condenseTx = MistbornAsset.createCondenseTransaction(
  assetId,
  ownerAddress,
  DensityLevel.Light,
  { texture: 'sword.png' }
);
```

### FogEconomy

Interact with economic systems.

```typescript
const economy = new FogEconomy(client);

// Get liquidity pools
const pools = await economy.getLiquidityPools();

// Create liquidity pool
const poolId = await economy.createLiquidityPool(
  'HAZE',
  'GOLD',
  reserve1, // bigint
  reserve2, // bigint
  30 // fee rate in basis points (0.3%)
);

// Calculate swap amount
const outputAmount = economy.calculateSwapAmount(
  pool,
  inputAmount,
  true // isAsset1Input
);
```

## Density Levels

Mistborn Assets support different density levels:

- **Ethereal** (5KB) - Basic metadata
- **Light** (50KB) - Main attributes + textures
- **Dense** (5MB) - Full set + 3D model
- **Core** (50MB+) - All data + history

## Examples

See `examples/basic-usage.ts` for complete examples.

Run examples:

```bash
# Install dependencies
npm install

# Build SDK
npm run build

# Run example (requires HAZE node running)
npx ts-node examples/basic-usage.ts
```

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run dev

# Clean
npm run clean
```

## API Endpoints

The SDK communicates with HAZE nodes via REST API. Full list and transaction contract (hex fields, `from`/`fee`/`nonce`): [API transaction contract](../docs/API_TRANSACTIONS.md).

- `GET /health` - Health check
- `GET /api/v1/blockchain/info` - Blockchain information
- `GET /api/v1/metrics/basic` - Basic metrics
- `POST /api/v1/transactions` - Send transaction
- `GET /api/v1/transactions/:hash` - Get transaction
- `GET /api/v1/accounts/:address`, `GET .../balance` - Account
- `GET /api/v1/assets/:asset_id`, `POST /api/v1/assets` - Assets; `.../history`, `.../versions`, `.../snapshot`, `GET /api/v1/assets/search`
- `POST /api/v1/assets/:asset_id/condense`, `evaporate`, `merge`, `split`; `POST /api/v1/assets/estimate-gas`; `GET|POST .../permissions`; `GET .../export`, `POST .../import`
- `GET /api/v1/economy/pools`, `POST /api/v1/economy/pools`, `GET .../pools/:pool_id`
- `POST /api/v1/sync/start`, `GET /api/v1/sync/status` - Sync
- `WS /api/v1/ws` - WebSocket