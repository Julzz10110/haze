# PowerShell script to start multiple HAZE nodes for testing multi-node consensus

param(
    [int]$NodeCount = 3
)

$ErrorActionPreference = "Stop"

Write-Host "HAZE Multi-Node Test Setup" -ForegroundColor Blue
Write-Host "================================"

$BasePort = 9000
$ApiBasePort = 8080

Write-Host "Setting up $NodeCount nodes..." -ForegroundColor Green

# Build the project first
Write-Host "Building HAZE..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Function to create node config
function Create-NodeConfig {
    param(
        [int]$NodeId,
        [int]$NetworkPort,
        [int]$ApiPort,
        [string]$BootstrapNodes
    )
    
    $ConfigFile = "haze_config_node${NodeId}.json"
    $DbPath = "./haze_db_node${NodeId}"
    
    $Config = @{
        node_id = "node-$NodeId"
        network = @{
            listen_addr = "/ip4/127.0.0.1/tcp/$NetworkPort"
            bootstrap_nodes = ($BootstrapNodes | ConvertFrom-Json)
            node_type = "core"
            min_core_stake = 1000
            min_edge_stake = 100
        }
        consensus = @{
            committee_rotation_interval = 900
            wave_finalization_threshold = 200
            golden_wave_threshold = 500
            max_transactions_per_block = 10000
        }
        vm = @{
            wasm_cache_size = 512
            gas_limit = 10000000
            gas_price = 1
        }
        storage = @{
            db_path = $DbPath
            state_cache_size = 256
            blob_storage_path = "$DbPath/blobs"
            max_blob_size = 104857600
            blob_chunk_size = 1048576
        }
        api = @{
            listen_addr = "127.0.0.1:$ApiPort"
            enable_cors = $true
            enable_websocket = $true
        }
        log_level = "info"
    }
    
    $Config | ConvertTo-Json -Depth 10 | Set-Content $ConfigFile
    return $ConfigFile
}

$Pids = @()

# Create configs and start nodes
for ($i = 1; $i -le $NodeCount; $i++) {
    $NetworkPort = $BasePort + $i - 1
    $ApiPort = $ApiBasePort + $i - 1
    
    if ($i -eq 1) {
        $Bootstrap = "[]"
    } else {
        $Bootstrap = "[\"/ip4/127.0.0.1/tcp/$BasePort\"]"
    }
    
    $ConfigFile = Create-NodeConfig -NodeId $i -NetworkPort $NetworkPort -ApiPort $ApiPort -BootstrapNodes $Bootstrap
    
    Write-Host "Starting Node $i..." -ForegroundColor Yellow
    Write-Host "  Network: 127.0.0.1:$NetworkPort"
    Write-Host "  API:     127.0.0.1:$ApiPort"
    Write-Host "  Config:  $ConfigFile"
    Write-Host "  DB:      ./haze_db_node$i"
    
    # Copy config to default location
    Copy-Item $ConfigFile haze_config.json -Force
    
    # Start node in background
    $Process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release" -NoNewWindow -PassThru -RedirectStandardOutput "node${i}.log" -RedirectStandardError "node${i}.log"
    $Pids += $Process.Id
    
    Write-Host "Node $i started (PID: $($Process.Id))" -ForegroundColor Green
    Write-Host ""
    
    # Wait a bit before starting next node
    Start-Sleep -Seconds 2
}

Write-Host "All $NodeCount nodes started!" -ForegroundColor Green
Write-Host ""
Write-Host "Node information:"
for ($i = 1; $i -le $NodeCount; $i++) {
    $ApiPort = $ApiBasePort + $i - 1
    Write-Host "  Node $i`: http://127.0.0.1:$ApiPort/health"
}
Write-Host ""
Write-Host "Logs are in: node1.log, node2.log, node3.log, ..."
Write-Host ""
Write-Host "To stop all nodes, run:"
Write-Host "  Stop-Process -Id $($Pids -join ',')"
Write-Host ""
Write-Host "Press Ctrl+C to stop all nodes..."

# Wait for user interrupt
try {
    while ($true) {
        Start-Sleep -Seconds 1
    }
} finally {
    Write-Host ""
    Write-Host "Stopping all nodes..." -ForegroundColor Yellow
    Stop-Process -Id $Pids -ErrorAction SilentlyContinue
}
