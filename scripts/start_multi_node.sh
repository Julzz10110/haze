#!/bin/bash
# Script to start multiple HAZE nodes for testing multi-node consensus

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}HAZE Multi-Node Test Setup${NC}"
echo "================================"

# Create directories for each node
NODE_COUNT=${1:-3}
BASE_PORT=9000
API_BASE_PORT=8080

echo -e "${GREEN}Setting up ${NODE_COUNT} nodes...${NC}"

# Build the project first
echo "Building HAZE..."
cargo build --release

# Function to create node config
create_node_config() {
    local node_id=$1
    local network_port=$2
    local api_port=$3
    local bootstrap_nodes=$4
    
    local config_file="haze_config_node${node_id}.json"
    local db_path="./haze_db_node${node_id}"
    
    cat > "$config_file" <<EOF
{
  "node_id": "node-${node_id}",
  "network": {
    "listen_addr": "/ip4/127.0.0.1/tcp/${network_port}",
    "bootstrap_nodes": ${bootstrap_nodes},
    "node_type": "core",
    "min_core_stake": 1000,
    "min_edge_stake": 100
  },
  "consensus": {
    "committee_rotation_interval": 900,
    "wave_finalization_threshold": 200,
    "golden_wave_threshold": 500,
    "max_transactions_per_block": 10000
  },
  "vm": {
    "wasm_cache_size": 512,
    "gas_limit": 10000000,
    "gas_price": 1
  },
  "storage": {
    "db_path": "${db_path}",
    "state_cache_size": 256,
    "blob_storage_path": "${db_path}/blobs",
    "max_blob_size": 104857600,
    "blob_chunk_size": 1048576
  },
  "api": {
    "listen_addr": "127.0.0.1:${api_port}",
    "enable_cors": true,
    "enable_websocket": true
  },
  "log_level": "info"
}
EOF
    echo "$config_file"
}

# Generate bootstrap addresses (first node has no bootstrap, others bootstrap to first)
BOOTSTRAP_NODES="[]"
if [ $NODE_COUNT -gt 1 ]; then
    BOOTSTRAP_NODES="[\"/ip4/127.0.0.1/tcp/${BASE_PORT}\"]"
fi

# Create configs and start nodes
PIDS=()
for i in $(seq 1 $NODE_COUNT); do
    NETWORK_PORT=$((BASE_PORT + i - 1))
    API_PORT=$((API_BASE_PORT + i - 1))
    
    if [ $i -eq 1 ]; then
        BOOTSTRAP="[]"
    else
        BOOTSTRAP="[\"/ip4/127.0.0.1/tcp/${BASE_PORT}\"]"
    fi
    
    CONFIG_FILE=$(create_node_config $i $NETWORK_PORT $API_PORT "$BOOTSTRAP")
    
    echo -e "${YELLOW}Starting Node ${i}...${NC}"
    echo "  Network: 127.0.0.1:${NETWORK_PORT}"
    echo "  API:     127.0.0.1:${API_PORT}"
    echo "  Config:  ${CONFIG_FILE}"
    echo "  DB:      ./haze_db_node${i}"
    
    # Set config file via environment variable or copy to default location
    cp "$CONFIG_FILE" haze_config.json
    
    # Start node in background
    RUST_LOG=info cargo run --release > "node${i}.log" 2>&1 &
    PID=$!
    PIDS+=($PID)
    
    echo -e "${GREEN}Node ${i} started (PID: ${PID})${NC}"
    echo ""
    
    # Wait a bit before starting next node
    sleep 2
done

echo -e "${GREEN}All ${NODE_COUNT} nodes started!${NC}"
echo ""
echo "Node information:"
for i in $(seq 1 $NODE_COUNT); do
    API_PORT=$((API_BASE_PORT + i - 1))
    echo "  Node ${i}: http://127.0.0.1:${API_PORT}/health"
done
echo ""
echo "Logs are in: node1.log, node2.log, node3.log, ..."
echo ""
echo "To stop all nodes, run: kill ${PIDS[*]}"
echo ""
echo "Press Ctrl+C to stop all nodes..."

# Wait for Ctrl+C
trap "echo ''; echo 'Stopping all nodes...'; kill ${PIDS[*]} 2>/dev/null; exit" INT
wait
