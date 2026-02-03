# HAZE Observability Guide

This document describes how to monitor and observe HAZE nodes in production.

## Metrics Endpoint

### Basic Metrics

The node exposes basic metrics via HTTP:

```bash
curl http://127.0.0.1:8080/api/v1/metrics/basic
```

Response:
```json
{
  "success": true,
  "data": {
    "current_height": 42,
    "last_finalized_height": 40,
    "last_finalized_wave": 2,
    "tx_pool_size": 15,
    "connected_peers": 0,
    "block_time_avg_ms": 5000
  }
}
```
`block_time_avg_ms` may be `null` if there are not enough blocks to compute an average.

Fields:
- `current_height`: Current blockchain height
- `last_finalized_height`: Last finalized checkpoint height
- `last_finalized_wave`: Last finalized wave number
- `tx_pool_size`: Number of transactions in the pool
- `connected_peers`: Number of connected P2P peers (MVP: always 0, network not accessible from API)
- `block_time_avg_ms`: Average block time in milliseconds (calculated from last 10 blocks)

## Logging

HAZE uses structured logging via `tracing`. Log levels can be controlled via `RUST_LOG`:

```bash
# Info level (default)
RUST_LOG=info cargo run --release

# Debug level (more verbose)
RUST_LOG=debug cargo run --release

# Trace level (very verbose, includes network events)
RUST_LOG=trace cargo run --release
```

### Key Log Messages

**Block Production:**
```
INFO Creating block with 10 transactions from pool
INFO Block created: height=42, hash=abc123..., txs=10, creation_time=15ms
INFO Block processed: height=42, process_time=5ms, total_time=20ms
```

**Metrics (every 30 seconds):**
```
INFO Metrics: height=42, finalized_height=40, finalized_wave=2, tx_pool=15, tx_per_sec_est=2
```

**Network:**
```
INFO Connected to peer: 12D3KooW...
INFO Peer is ahead: peer_height=45, local_height=42, requesting sync
```

**Sync:**
```
INFO Received blockchain info: height=45, finalized_height=40, finalized_wave=2
INFO State root matches at checkpoint height 40
```

## Log Aggregation

### Simple Log Collection Script

For local development, you can use simple shell scripts to aggregate logs:

**`scripts/aggregate_logs.sh`** (Linux/Mac):
```bash
#!/bin/bash
# Aggregate logs from multiple nodes

LOG_DIR="./logs"
OUTPUT="aggregated.log"

echo "Aggregating logs from $LOG_DIR..."

# Combine all node logs with timestamps
for log in "$LOG_DIR"/node*.log; do
    if [ -f "$log" ]; then
        echo "=== $(basename $log) ===" >> "$OUTPUT"
        cat "$log" >> "$OUTPUT"
        echo "" >> "$OUTPUT"
    fi
done

echo "Logs aggregated to $OUTPUT"
```

**`scripts/aggregate_logs.ps1`** (Windows):
```powershell
# Aggregate logs from multiple nodes

$LogDir = "./logs"
$Output = "aggregated.log"

Write-Host "Aggregating logs from $LogDir..."

Get-ChildItem "$LogDir/node*.log" | ForEach-Object {
    Add-Content -Path $Output -Value "=== $($_.Name) ==="
    Get-Content $_.FullName | Add-Content -Path $Output
    Add-Content -Path $Output -Value ""
}

Write-Host "Logs aggregated to $Output"
```

### Filtering Logs

**Extract metrics only:**
```bash
grep "Metrics:" node*.log
```

**Extract block creation events:**
```bash
grep "Block created:" node*.log
```

**Extract errors:**
```bash
grep -i "error\|failed\|warn" node*.log
```

**Extract sync events:**
```bash
grep -i "sync\|peer" node*.log
```

## Prometheus

The node exposes metrics in [Prometheus text exposition format](https://prometheus.io/docs/instrumenting/exposition_formats/).

### Endpoint

```bash
curl http://127.0.0.1:8080/api/v1/metrics/prometheus
```

**Response:** `Content-Type: text/plain; version=0.0.4; charset=utf-8`

### Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `haze_blockchain_height` | gauge | Current blockchain height |
| `haze_finalized_height` | gauge | Last finalized block height |
| `haze_finalized_wave` | gauge | Last finalized wave number |
| `haze_tx_pool_size` | gauge | Number of transactions in the pool |
| `haze_connected_peers` | gauge | Number of connected P2P peers |
| `haze_block_time_seconds` | gauge | Average block time in seconds (last 10 blocks); 0 if not available |

### Scrape configuration

Example `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'haze'
    static_configs:
      - targets: ['127.0.0.1:8080']
    metrics_path: '/api/v1/metrics/prometheus'
    scrape_interval: 15s
```

For multiple nodes, add one target per node or use a discovery mechanism.

## Grafana

Example panel queries (Prometheus data source):

- **Blockchain height:** `haze_blockchain_height`
- **Finalized height:** `haze_finalized_height`
- **Transaction pool size:** `haze_tx_pool_size`
- **Connected peers:** `haze_connected_peers`
- **Average block time (s):** `haze_block_time_seconds`
- **Height growth (rate):** `deriv(haze_blockchain_height[5m])` or `increase(haze_blockchain_height[1m])`

## Alerts

Suggested Prometheus/Alertmanager rules:

- **Height not increasing:** `increase(haze_blockchain_height[5m]) == 0` and `haze_blockchain_height > 0`
- **Finalization stuck:** `increase(haze_finalized_height[10m]) == 0` and `haze_finalized_height > 0`
- **High tx pool:** `haze_tx_pool_size > 5000` (adjust threshold)
- **No peers:** `haze_connected_peers == 0` (for nodes that should have peers)

## Health Checks

### Basic Health Check

```bash
curl http://127.0.0.1:8080/health
```

Returns: `{"success": true, "data": "OK"}`

### Extended Health Check

Check multiple endpoints:

```bash
#!/bin/bash
NODE_URL="http://127.0.0.1:8080"

echo "Health: $(curl -s $NODE_URL/health | jq -r '.data')"
echo "Height: $(curl -s $NODE_URL/api/v1/blockchain/info | jq -r '.data.current_height')"
echo "Metrics: $(curl -s $NODE_URL/api/v1/metrics/basic | jq -r '.data')"
```

## Troubleshooting

### Node Not Producing Blocks

1. Check transaction pool:
   ```bash
   curl http://127.0.0.1:8080/api/v1/metrics/basic | jq '.data.tx_pool_size'
   ```

2. Check logs for errors:
   ```bash
   grep -i "error\|failed" node.log
   ```

### Nodes Not Syncing

1. Check peer connections:
   ```bash
   grep "Connected to peer" node*.log
   ```

2. Check sync status:
   ```bash
   curl http://127.0.0.1:8080/api/v1/sync/status
   ```

3. Check for state root mismatches:
   ```bash
   grep "State root mismatch" node*.log
   ```

### High Transaction Pool Size

If `tx_pool_size` is consistently high:

1. Check block production rate (should be ~1 block per 5 seconds)
2. Check for block processing errors
3. Verify network connectivity (blocks should propagate)

## Best Practices

1. **Log Rotation**: Use log rotation tools (`logrotate` on Linux) to prevent disk space issues
2. **Structured Logging**: Use JSON logging format for easier parsing (future enhancement)
3. **Alerting**: Set up alerts for:
   - Height not increasing
   - High transaction pool size
   - State root mismatches
   - Network disconnections
4. **Monitoring**: Track key metrics over time to identify trends and issues
