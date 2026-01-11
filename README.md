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
- Rust 1.70+ (latest stable version recommended)
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
- Network parameters
- Consensus parameters
- VM settings
- Database paths

## Usage

### Basic Example

Run the API usage example:

```bash
cargo run --example basic_usage
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
