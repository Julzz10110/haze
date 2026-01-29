# Multi-Node Setup Guide

This guide explains how to run multiple HAZE nodes for testing network consensus and state synchronization.

## Overview

The multi-node setup allows you to run 2-3 (or more) HAZE nodes that:
- Connect to each other via P2P network (libp2p)
- Share transactions and blocks
- Maintain synchronized blockchain state
- Test consensus mechanisms

## Quick Start

### Using Scripts

**Linux/macOS:**
```bash
chmod +x scripts/start_multi_node.sh
./scripts/start_multi_node.sh 3  # Start 3 nodes
```

**Windows (PowerShell):**
```powershell
.\scripts\start_multi_node.ps1 -NodeCount 3
```

### Manual Setup

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Create configuration files for each node:**  
   Minimal config below; other keys (consensus, vm, storage.blob_storage_path, etc.) use defaults. After first run, see generated `haze_config.json` for the full structure.

   **Node 1** (`haze_config_node1.json`):
   ```json
   {
     "node_id": "node-1",
     "network": {
       "listen_addr": "/ip4/127.0.0.1/tcp/9000",
       "bootstrap_nodes": [],
       "node_type": "core"
     },
     "api": {
       "listen_addr": "127.0.0.1:8080"
     },
     "storage": {
       "db_path": "./haze_db_node1"
     }
   }
   ```

   **Node 2** (`haze_config_node2.json`):
   ```json
   {
     "node_id": "node-2",
     "network": {
       "listen_addr": "/ip4/127.0.0.1/tcp/9001",
       "bootstrap_nodes": ["/ip4/127.0.0.1/tcp/9000"],
       "node_type": "core"
     },
     "api": {
       "listen_addr": "127.0.0.1:8081"
     },
     "storage": {
       "db_path": "./haze_db_node2"
     }
   }
   ```

   **Node 3** (`haze_config_node3.json`):
   ```json
   {
     "node_id": "node-3",
     "network": {
       "listen_addr": "/ip4/127.0.0.1/tcp/9002",
       "bootstrap_nodes": ["/ip4/127.0.0.1/tcp/9000"],
       "node_type": "core"
     },
     "api": {
       "listen_addr": "127.0.0.1:8082"
     },
     "storage": {
       "db_path": "./haze_db_node3"
     }
   }
   ```

3. **Start each node in a separate terminal:**

   **Terminal 1 (Node 1):**
   ```bash
   cp haze_config_node1.json haze_config.json
   cargo run --release
   ```

   **Terminal 2 (Node 2):**
   ```bash
   cp haze_config_node2.json haze_config.json
   cargo run --release
   ```

   **Terminal 3 (Node 3):**
   ```bash
   cp haze_config_node3.json haze_config.json
   cargo run --release
   ```

## Testing Multi-Node Consensus

### 1. Check Node Health

```bash
# Node 1
curl http://127.0.0.1:8080/health

# Node 2
curl http://127.0.0.1:8081/health

# Node 3
curl http://127.0.0.1:8082/health
```

### 2. Check Blockchain Info

```bash
# All nodes should show similar info
curl http://127.0.0.1:8080/api/v1/blockchain/info
curl http://127.0.0.1:8081/api/v1/blockchain/info
curl http://127.0.0.1:8082/api/v1/blockchain/info
```

### 3. Send Transaction to Node 1

Transactions must be **signed**; the body is `{"transaction": <Transaction>}`. Use the [TypeScript SDK](../sdk/README.md#building-and-signing-a-transaction) or see [API transaction contract](API_TRANSACTIONS.md) for the exact JSON shape (hex strings for byte fields).

```bash
# Example: use SDK to build and sign, then POST the JSON
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{"transaction": {"Transfer": {"from": "<hex>", "to": "<hex>", "amount": "1000", "fee": "0", "nonce": 0, "signature": "<hex>"}}}'
```

### 4. Verify Transaction Propagation

The transaction should:
1. Be added to Node 1's transaction pool
2. Be broadcast to Node 2 and Node 3
3. Appear in all nodes' transaction pools
4. Be included in the next block created by any node

### 5. Check Block Synchronization

After a block is created:
- Check block height on all nodes (should be synchronized)
- Verify blocks are the same across nodes
- Check that state (accounts, assets) is consistent

```bash
# Get latest block from each node
curl http://127.0.0.1:8080/api/v1/blocks/height/0
curl http://127.0.0.1:8081/api/v1/blocks/height/0
curl http://127.0.0.1:8082/api/v1/blocks/height/0
```

## Network Architecture

```
Node 1 (Bootstrap)
  ├── Listen: 127.0.0.1:9000
  ├── API: 127.0.0.1:8080
  └── DB: ./haze_db_node1

Node 2
  ├── Listen: 127.0.0.1:9001
  ├── API: 127.0.0.1:8081
  ├── DB: ./haze_db_node2
  └── Bootstrap: → Node 1

Node 3
  ├── Listen: 127.0.0.1:9002
  ├── API: 127.0.0.1:8082
  ├── DB: ./haze_db_node3
  └── Bootstrap: → Node 1
```

## How It Works

1. **Bootstrap Connection:**
   - Node 1 starts first (no bootstrap nodes)
   - Node 2 and Node 3 connect to Node 1 on startup
   - All nodes discover each other through the P2P network

2. **Transaction Propagation:**
   - Transaction sent to any node is added to its pool
   - Node broadcasts transaction to all connected peers
   - Peers validate and add to their pools (gossip protocol)

3. **Block Propagation:**
   - When a node creates a block, it broadcasts to all peers
   - Peers validate the block and apply it to their state
   - Blocks are also gossiped to other peers

4. **State Synchronization:**
   - All nodes process the same blocks in the same order
   - State root is verified after each block
   - Height and state should match across all nodes

## Troubleshooting

### Nodes Not Connecting

- Check that Node 1 is started first
- Verify bootstrap addresses are correct
- Check firewall settings
- Look at logs for connection errors

### State Divergence

- Check logs for block processing errors
- Verify all nodes are processing blocks in order
- Check for network partitions

### High CPU/Memory Usage

- Reduce `max_transactions_per_block` in config
- Increase block creation interval
- Check for memory leaks in logs

## Next Steps

- Implement advanced consensus (BFT, DAG finalization)
- Add peer discovery mechanisms
- Implement state sync for nodes joining late
- Add network metrics and monitoring
