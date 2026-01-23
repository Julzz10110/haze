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
    
    /// Asset operations gas costs configuration
    pub asset_gas: AssetGasConfig,
    
    /// Asset limits and quotas configuration
    pub asset_limits: AssetLimits,
    
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
    
    /// Enable strict block validation (hash/height/parent checks)
    pub strict_block_validation: bool,
    
    /// Maximum allowed future block height delta before rejecting
    /// (relative to current local height). Used only when
    /// `strict_block_validation` is enabled.
    pub max_future_block_height_delta: u64,
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

/// Gas costs configuration for asset operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGasConfig {
    /// Base gas cost for creating an asset
    pub create_base: u64,
    
    /// Gas cost per KB of metadata for create operation
    pub create_per_kb: u64,
    
    /// Base gas cost for updating an asset
    pub update_base: u64,
    
    /// Gas cost per KB of updated metadata
    pub update_per_kb: u64,
    
    /// Base gas cost for condensing (increasing density)
    pub condense_base: u64,
    
    /// Gas cost multiplier based on target density level
    /// (Ethereal->Light: 1x, Light->Dense: 2x, Dense->Core: 5x)
    pub condense_density_multiplier: u64,
    
    /// Gas cost per KB of new data for condense
    pub condense_per_kb: u64,
    
    /// Base gas cost for evaporating (decreasing density)
    pub evaporate_base: u64,
    
    /// Gas cost for merging assets (base)
    pub merge_base: u64,
    
    /// Gas cost per KB of combined asset size
    pub merge_per_kb: u64,
    
    /// Base gas cost for splitting an asset
    pub split_base: u64,
    
    /// Gas cost per component created in split
    pub split_per_component: u64,
    
    /// Gas cost per KB of component data
    pub split_per_kb: u64,
}

/// Asset limits and quotas configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLimits {
    /// Maximum number of assets per account (base limit)
    pub max_assets_per_account: u64,
    
    /// Maximum metadata size per asset (bytes)
    pub max_metadata_size: usize,
    
    /// Maximum number of blob files per asset
    pub max_blob_files_per_asset: u64,
    
    /// Quotas for different node types
    pub quotas: NodeQuotas,
}

/// Quotas for different node types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeQuotas {
    /// Core node quotas (1000+ HAZE stake)
    pub core: NodeQuota,
    
    /// Edge node quotas (100+ HAZE stake)
    pub edge: NodeQuota,
    
    /// Light node quotas (no stake)
    pub light: NodeQuota,
    
    /// Mobile node quotas
    pub mobile: NodeQuota,
}

/// Quota configuration for a node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeQuota {
    /// Maximum assets per account
    pub max_assets_per_account: u64,
    
    /// Maximum metadata size per asset (bytes)
    pub max_metadata_size: usize,
    
    /// Maximum blob files per asset
    pub max_blob_files_per_asset: u64,
    
    /// Maximum total blob storage per account (bytes)
    pub max_blob_storage_per_account: u64,
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
                strict_block_validation: false,
                max_future_block_height_delta: 2,
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
            asset_gas: AssetGasConfig {
                create_base: 10_000,
                create_per_kb: 100,
                update_base: 5_000,
                update_per_kb: 50,
                condense_base: 15_000,
                condense_density_multiplier: 1, // Base multiplier
                condense_per_kb: 200,
                evaporate_base: 2_000, // Minimal cost for archiving
                merge_base: 20_000,
                merge_per_kb: 150,
                split_base: 15_000,
                split_per_component: 5_000,
                split_per_kb: 100,
            },
            asset_limits: AssetLimits {
                max_assets_per_account: 10_000,
                max_metadata_size: 50 * 1024 * 1024, // 50MB (Core density max)
                max_blob_files_per_asset: 100,
                quotas: NodeQuotas {
                    core: NodeQuota {
                        max_assets_per_account: 100_000,
                        max_metadata_size: 50 * 1024 * 1024, // 50MB
                        max_blob_files_per_asset: 500,
                        max_blob_storage_per_account: 10 * 1024 * 1024 * 1024, // 10GB
                    },
                    edge: NodeQuota {
                        max_assets_per_account: 50_000,
                        max_metadata_size: 50 * 1024 * 1024, // 50MB
                        max_blob_files_per_asset: 200,
                        max_blob_storage_per_account: 5 * 1024 * 1024 * 1024, // 5GB
                    },
                    light: NodeQuota {
                        max_assets_per_account: 10_000,
                        max_metadata_size: 5 * 1024 * 1024, // 5MB (Dense max)
                        max_blob_files_per_asset: 100,
                        max_blob_storage_per_account: 1 * 1024 * 1024 * 1024, // 1GB
                    },
                    mobile: NodeQuota {
                        max_assets_per_account: 1_000,
                        max_metadata_size: 50 * 1024, // 50KB (Light max)
                        max_blob_files_per_asset: 10,
                        max_blob_storage_per_account: 100 * 1024 * 1024, // 100MB
                    },
                },
            },
            log_level: "info".to_string(),
        }
    }
    
    /// Get quota for current node type
    pub fn get_node_quota(&self) -> &NodeQuota {
        match self.network.node_type.as_str() {
            "core" => &self.asset_limits.quotas.core,
            "edge" => &self.asset_limits.quotas.edge,
            "light" => &self.asset_limits.quotas.light,
            "mobile" => &self.asset_limits.quotas.mobile,
            _ => &self.asset_limits.quotas.light, // Default to light
        }
    }
}