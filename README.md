# HAZE Blockchain

**High-performance Asset Zone Engine** - specialized Layer 1 blockchain for GameFi

> "Where games breathe blockchain"

## Concept

HAZE creates a "high-performance zone" for gaming assets and transactions, using the metaphor of "digital fog" - a distributed, fluid, yet omnipresent environment where gaming assets exist and interact at sub-second speeds.

## Architecture

### Multi-layered Structure

```
┌─────────────────────────────────────────┐
│           Haze Interface Layer          │ ← "Surface of the fog"
│   Unity/Unreal SDK | Web3.js | REST     │
├─────────────────────────────────────────┤
│          Haze Execution Cloud           │ ← "Body of the fog"
│    WASM VM + Game Primitives + State    │
├─────────────────────────────────────────┤
│        Haze Consensus Core              │ ← "Core of the fog"
│    DAG-Narwhal Modified "Fog Consensus" │
├─────────────────────────────────────────┤
│        Haze Distribution Network        │ ← "Foundation of the fog"
│       libp2p with Priority Channels     │
└─────────────────────────────────────────┘
```

## Core Components

### 1. Fog Consensus
- **Haze Committees**: Dynamic validator groups
- **Wave Finalization**: Transactions propagate in waves through DAG
- **Haze Weights**: Reputation system based on gaming activity

### 2. HazeVM
- WASM-based virtual machine
- Game primitives:
  - **Asset Mist**: Dynamic NFTs
  - **Economy Fog**: Built-in economic systems
  - **Quest Haze**: Verifiable quests
  - **Battle Smoke**: PvP system

### 3. Mistborn Assets
Dynamic NFTs with density levels:
- **Ethereal** (5KB) - basic metadata
- **Light** (50KB) - main attributes + textures
- **Dense** (5MB) - full set + 3D model
- **Core** (50MB+) - all data + history

Mechanisms:
- Condensation (density increase)
- Evaporation (density decrease)
- Merging (NFT combination)
- Splitting (component separation)

### 4. Haze Mesh Network Topology
- Core nodes (1000+ HAZE stake)
- Edge nodes (100+ HAZE stake)
- Light nodes (no stake)
- Mobile nodes

## Installation and Running

### Requirements
- Rust 1.85+ (project uses edition 2024; or latest stable)
- Cargo

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd haze

# Build the project
cargo build --release

# Run the node
cargo run --release
```

### Configuration

On first run, a `haze_config.json` file is created with default settings. You can edit it to configure:
- Network parameters (`network.listen_addr` - default: `/ip4/0.0.0.0/tcp/9000`)
- Consensus parameters (`consensus.max_transactions_per_block` - default: 10000)
- VM settings
- Database paths (`storage.db_path` - default: `./haze_db`)
- API settings (`api.listen_addr` - default: `127.0.0.1:8080`)

### MVP Node Quick Start

After starting the node, you should see initialization logs:

```
═══════════════════════════════════════════════════════════
  HAZE Blockchain - High-performance Asset Zone Engine
  Where games breathe blockchain
═══════════════════════════════════════════════════════════
✓ Configuration loaded from: haze_config.json
✓ State manager initialized
✓ Consensus engine initialized
✓ Network layer initialized
✓ WebSocket event broadcaster initialized
✓ API server state initialized
═══════════════════════════════════════════════════════════
  HAZE node is running!
  API: http://127.0.0.1:8080/health
  WebSocket: ws://127.0.0.1:8080/api/v1/ws
  Press Ctrl+C to shutdown
═══════════════════════════════════════════════════════════
```

**Verify the node is running:**

```bash
# Health check
curl http://127.0.0.1:8080/health

# Get blockchain info
curl http://127.0.0.1:8080/api/v1/blockchain/info
```

Expected response:
```json
{
  "success": true,
  "data": {
    "current_height": 0,
    "total_supply": 1000000000,
    "current_wave": 0,
    "state_root": "<hex>",
    "last_finalized_height": 0,
    "last_finalized_wave": 0
  }
}
```

## Usage

### Quick Start Example

1. **Start the node:**
   ```bash
   cargo run --release
   ```

2. **In another terminal, check the node status:**
   ```bash
   curl http://127.0.0.1:8080/health
   curl http://127.0.0.1:8080/api/v1/blockchain/info
   ```

3. **Send a transaction** (using Rust example):
   ```bash
   cargo run --example basic_usage
   ```

### TypeScript SDK

TypeScript SDK v0.1 is available in the `sdk/` directory. See [SDK README](sdk/README.md) for detailed documentation.

Quick start:

```bash
cd sdk
npm install
npm run build
```

Example usage:

```typescript
import { HazeClient, KeyPair, DEFAULT_API_URL } from '@haze/sdk';

const client = new HazeClient({ baseUrl: DEFAULT_API_URL });
const keyPair = await KeyPair.generate();
const info = await client.getBlockchainInfo();
console.log('Blockchain height:', info.current_height);
console.log('Total supply:', info.total_supply.toString());
console.log('Current wave:', info.current_wave);
console.log('State root:', info.state_root);
console.log('Last finalized height:', info.last_finalized_height);
```

#### Multi-node e2e / load test

After starting multiple nodes locally (see multi-node scripts), you can run a simple multi-node
consistency / load test from the `sdk/` directory:

```bash
cd sdk
npm run build

# Default: assumes 3 nodes on 8080/8081/8082
node dist/examples/multi-node-e2e.js

# Or specify explicit node URLs and tx count
HAZE_E2E_NODE_URLS="http://127.0.0.1:8080,http://127.0.0.1:8081,http://127.0.0.1:8082" \
HAZE_E2E_TX_COUNT=20 \
node dist/examples/multi-node-e2e.js
```

The script will:
- ping all nodes and fetch `BlockchainInfo`
- optionally send transfer transactions via one node
- verify that heights, block hashes (for first few heights) and checkpoint state roots match.

### Rust API

Run the API usage example:

```bash
cargo run --example basic_usage
```

### Developer Workflow

1. **Start a local node:**
   ```bash
   cargo run --release
   ```

2. **Send transactions via REST API:**  
   Transactions must be **signed** by the sender. Use the [TypeScript SDK](sdk/README.md) or build the payload as in [API transaction contract](docs/API_TRANSACTIONS.md). Example (SDK):
   ```typescript
   const tx = TransactionBuilder.createTransfer(from, to, amount, fee, nonce);
   const signed = await TransactionBuilder.sign(tx, keyPair);
   await client.sendTransaction(signed);
   ```
   See [Building and signing a transaction](sdk/README.md#building-and-signing-a-transaction) and [API transaction contract](docs/API_TRANSACTIONS.md) for the full request shape.

3. **Monitor blocks:**
   ```bash
   # Get latest block
   curl http://127.0.0.1:8080/api/v1/blocks/height/0
   ```

4. **Use WebSocket for real-time events:**
   ```javascript
   const ws = new WebSocket('ws://127.0.0.1:8080/api/v1/ws');
   ws.onmessage = (event) => {
     const data = JSON.parse(event.data);
     console.log('Event:', data);
   };
   ```

### Function Examples

#### Creating a Key Pair and Address
```rust
use haze::crypto::KeyPair;

let keypair = KeyPair::generate();
let address = keypair.address();
```

#### Staking Tokens
```rust
use haze::tokenomics::Tokenomics;

let tokenomics = Tokenomics::new();
tokenomics.stake(validator_address, validator_address, amount)?;
```

#### Creating a Mistborn NFT
```rust
use haze::assets::MistbornAsset;
use haze::types::DensityLevel;

let asset = MistbornAsset::create(
    asset_id,
    owner_address,
    DensityLevel::Ethereal,
    metadata,
);
```

#### Creating a Liquidity Pool
```rust
use haze::economy::FogEconomy;

let economy = FogEconomy::new();
let pool_id = economy.create_liquidity_pool(
    "HAZE".to_string(),
    "GOLD".to_string(),
    reserve1,
    reserve2,
    fee_rate,
)?;
```

## MVP Status

The current MVP implementation includes:

**Core Features:**
- Single node operation with full blockchain functionality
- REST API server (HTTP endpoints)
- WebSocket support for real-time events
- Transaction pool and block creation
- Mistborn Assets (dynamic NFTs) with density levels
- Fog Economy (liquidity pools)
- Basic consensus (DAG-based with wave finalization)
- P2P network layer (libp2p)

**API Endpoints:** (see [API transaction contract](docs/API_TRANSACTIONS.md) for transaction format)
- `GET /health` - Health check
- `GET /api/v1/blockchain/info` - Blockchain information
- `GET /api/v1/metrics/basic` - Basic metrics (height, finalized height, tx pool, block time)
- `POST /api/v1/transactions` - Send transaction
- `GET /api/v1/transactions/:hash` - Get transaction
- `GET /api/v1/blocks/:hash` - Get block by hash
- `GET /api/v1/blocks/height/:height` - Get block by height
- `GET /api/v1/accounts/:address` - Get account info; `GET .../balance` - Balance
- `GET /api/v1/assets/:asset_id` - Get asset; `POST /api/v1/assets` - Create asset
- `GET /api/v1/assets/:asset_id/history` - Asset history; `.../versions`, `.../snapshot`; `GET /api/v1/assets/search` - Search
- `POST /api/v1/assets/:asset_id/condense`, `.../evaporate`, `.../merge`, `.../split` - Asset ops
- `POST /api/v1/assets/estimate-gas` - Estimate gas; `GET|POST .../permissions`; `GET .../export`, `POST .../import`
- `GET /api/v1/economy/pools`, `POST /api/v1/economy/pools`, `GET .../pools/:pool_id`
- `POST /api/v1/sync/start`, `GET /api/v1/sync/status` - Sync
- `WS /api/v1/ws` - WebSocket for real-time events

## Target Performance Metrics

- Transaction propagation: < 50 ms (95th percentile)
- Initial finalization: 200 ms
- Full finalization: 450 ms
- Throughput: 20,000 TPS (peak)
- Asset capacity: 100+ million NFTs
- Transaction cost: $0.0003 on average

## Tokenomics

### HAZE Token
- Initial supply: 1,000,000,000 HAZE
- Annual inflation: 3% (decreases by 0.5% each year)
- Inflation distribution: 70% to stakers, 30% to treasury

### Utility Functions
- Gas for transactions (50% burned)
- Staking for validators
- Protocol governance
- Access to premium features
