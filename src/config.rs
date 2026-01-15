//! Configuration for HAZE node

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{HazeError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API server listen address
    pub listen_addr: String,
    
    /// Enable CORS
    pub enable_cors: bool,
    
    /// Enable WebSocket support
    pub enable_websocket: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Node identity
    pub node_id: String,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    
    /// VM configuration
    pub vm: VMConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// API configuration
    pub api: ApiConfig,
    
    /// Logging level
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address
    pub listen_addr: String,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    
    /// Node type: core, edge, light, mobile
    pub node_type: String,
    
    /// Minimum stake for core nodes
    pub min_core_stake: u64,
    
    /// Minimum stake for edge nodes
    pub min_edge_stake: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Committee rotation interval (seconds)
    pub committee_rotation_interval: u64,
    
    /// Wave finalization threshold (ms)
    pub wave_finalization_threshold: u64,
    
    /// Golden wave threshold (ms)
    pub golden_wave_threshold: u64,
    
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    /// WASM cache size (MB)
    pub wasm_cache_size: usize,
    
    /// Gas limit per transaction
    pub gas_limit: u64,
    
    /// Gas price
    pub gas_price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database path
    pub db_path: PathBuf,
    
    /// State cache size (MB)
    pub state_cache_size: usize,
    
    /// Blob storage path for large files
    pub blob_storage_path: PathBuf,
    
    /// Maximum blob size (bytes) - for Core density assets
    pub max_blob_size: usize,
    
    /// Chunk size for streaming large files (bytes)
    pub blob_chunk_size: usize,
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let default_config = Self::default();
        
        // Try to load from config file
        let config_path = PathBuf::from("haze_config.json");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| HazeError::Config(format!("Failed to read config: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| HazeError::Config(format!("Failed to parse config: {}", e)))
        } else {
            // Save default config
            let content = serde_json::to_string_pretty(&default_config)
                .map_err(|e| HazeError::Config(format!("Failed to serialize config: {}", e)))?;
            std::fs::write(&config_path, content)
                .map_err(|e| HazeError::Config(format!("Failed to write config: {}", e)))?;
            Ok(default_config)
        }
    }

    pub fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            network: NetworkConfig {
                listen_addr: "/ip4/0.0.0.0/tcp/9000".to_string(),
                bootstrap_nodes: vec![],
                node_type: "light".to_string(),
                min_core_stake: 1000,
                min_edge_stake: 100,
            },
            consensus: ConsensusConfig {
                committee_rotation_interval: 900, // 15 minutes
                wave_finalization_threshold: 200,
                golden_wave_threshold: 500,
                max_transactions_per_block: 10000,
            },
            vm: VMConfig {
                wasm_cache_size: 512,
                gas_limit: 10_000_000,
                gas_price: 1,
            },
            storage: StorageConfig {
                db_path: PathBuf::from("./haze_db"),
                state_cache_size: 256,
                blob_storage_path: PathBuf::from("./haze_db/blobs"),
                max_blob_size: 100 * 1024 * 1024, // 100MB for Core density
                blob_chunk_size: 1024 * 1024, // 1MB chunks
            },
            api: ApiConfig {
                listen_addr: "127.0.0.1:8080".to_string(),
                enable_cors: true,
                enable_websocket: true,
            },
            log_level: "info".to_string(),
        }
    }
}