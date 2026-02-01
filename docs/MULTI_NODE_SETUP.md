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

### 3. Check Sync Status and Connected Peers

Use the sync status API to verify connectivity and heights across nodes:

```bash
# Node 1 (bootstrap) — may show 0 or 2 peers depending on who connected
curl http://127.0.0.1:8080/api/v1/sync/status

# Node 2 and 3 — should show at least 1 connected peer (bootstrap)
curl http://127.0.0.1:8081/api/v1/sync/status
curl http://127.0.0.1:8082/api/v1/sync/status
```

Response shape: `{"success":true,"data":{"current_height":N,"last_finalized_height":N,"last_finalized_wave":N,"syncing":false,"connected_peers":M}}`. After all nodes are up, expect `connected_peers` ≥ 1 on nodes 2 and 3; node 1 should see 2 peers once 2 and 3 have connected.

**Basic metrics** (includes same heights + `connected_peers` and `tx_pool_size`):

```bash
curl http://127.0.0.1:8080/api/v1/metrics/basic
curl http://127.0.0.1:8081/api/v1/metrics/basic
curl http://127.0.0.1:8082/api/v1/metrics/basic
```

### 4. Send Transaction to Node 1

Transactions must be **signed**; the body is `{"transaction": <Transaction>}`. Use the [TypeScript SDK](../sdk/README.md#building-and-signing-a-transaction) or see [API transaction contract](API_TRANSACTIONS.md) for the exact JSON shape (hex strings for byte fields).

```bash
# Example: use SDK to build and sign, then POST the JSON
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{"transaction": {"Transfer": {"from": "<hex>", "to": "<hex>", "amount": "1000", "fee": "0", "nonce": 0, "signature": "<hex>"}}}'
```

### 5. Verify Transaction Propagation

The transaction should:
1. Be added to Node 1's transaction pool
2. Be broadcast to Node 2 and Node 3
3. Appear in all nodes' transaction pools
4. Be included in the next block created by any node

### 6. Check Block Synchronization

After a block is created:
- Check block height on all nodes (should be synchronized)
- Verify blocks are the same across nodes
- Check that state (accounts, assets) is consistent

**Step 1:** Get current height from blockchain info (use this height for the latest block):

```bash
curl http://127.0.0.1:8080/api/v1/blockchain/info
# Response includes "current_height": N
```

**Step 2:** Fetch the latest block by that height on each node (replace `N` with the value from step 1):

```bash
curl http://127.0.0.1:8080/api/v1/blocks/height/N
curl http://127.0.0.1:8081/api/v1/blocks/height/N
curl http://127.0.0.1:8082/api/v1/blocks/height/N
```

Heights and block hashes should match across nodes once sync has propagated. You can also use `/api/v1/sync/status` to compare `current_height` and `last_finalized_height` on all nodes.

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

### Late-joiner (catch-up) sync

When a node joins the network later (or was offline), it syncs from peers as follows:

1. **Connect:** The node connects to bootstrap node(s) and discovers peers.
2. **Blockchain info:** Periodically (every 30 seconds) the node requests *blockchain info* from a peer (current height, finalized height, state root).
3. **Catch-up:** If the peer’s height is higher than the local height, the node starts *catch-up sync*:
   - It requests missing blocks from the peer in batches (100 blocks per request).
   - After each batch is received and applied, it checks local height again.
   - If still behind the target height, it immediately requests the next batch from the same (or another) peer.
   - This continues until local height matches the target or no more blocks are available.
4. **Result:** The late-joiner catches up to the peer’s height without waiting for the next 30-second tick between batches; multiple batches are requested in sequence until synced.

You can verify catch-up by starting Node 1, letting it produce blocks, then starting Node 2: Node 2 should reach the same height as Node 1 within a short time (check via `/api/v1/sync/status` or `/api/v1/blockchain/info`).

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
