//! State management for HAZE blockchain

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use sled::Db;
use tokio::sync::broadcast;
use crate::types::{Address, Hash, Block, Transaction, AssetAction, AssetPermission, PermissionLevel};
use crate::config::Config;
use crate::error::{HazeError, Result};
use crate::tokenomics::Tokenomics;
use crate::economy::FogEconomy;
use crate::ws_events::WsEvent;
use dashmap::DashMap;
use hex;

/// State manager for blockchain state
pub struct StateManager {
    db: Arc<Db>,
    config: Arc<Config>,
    accounts: Arc<DashMap<Address, AccountState>>,
    assets: Arc<DashMap<Hash, AssetState>>,
    blocks: Arc<DashMap<Hash, Block>>,
    current_height: Arc<RwLock<u64>>,
    tokenomics: Arc<Tokenomics>,
    economy: Arc<FogEconomy>,
    ws_tx: Arc<RwLock<Option<broadcast::Sender<WsEvent>>>>,
    
    // Indexes for fast asset search
    asset_index_by_owner: Arc<DashMap<Address, Vec<Hash>>>,
    asset_index_by_game_id: Arc<DashMap<String, Vec<Hash>>>,
    asset_index_by_density: Arc<DashMap<u8, Vec<Hash>>>, // Using u8 for density level
    
    // Cache for frequently accessed assets (LRU-like with access counter)
    asset_access_count: Arc<DashMap<Hash, u64>>, // Track access frequency
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
    pub staked: u64,
}

/// History entry for asset (same as in assets.rs but needed here for serialization)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetHistoryEntry {
    pub timestamp: i64,
    pub action: AssetAction,
    pub changes: HashMap<String, String>,
}

/// Asset version snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetVersion {
    pub version: u64,
    pub timestamp: i64,
    pub data: crate::types::AssetData,
    pub blob_refs: HashMap<String, Hash>,
}

/// Quota usage information for an account
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaUsage {
    pub assets_count: u64,
    pub assets_limit: u64,
    pub blob_files_count: u64,
    pub blob_files_limit: u64,
    pub blob_storage_estimate: u64,
    pub blob_storage_limit: u64,
    pub metadata_size_limit: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetState {
    pub owner: Address,
    pub data: crate::types::AssetData,
    pub created_at: i64,
    pub updated_at: i64,
    /// Blob references for large files (Core density)
    /// Maps blob key to blob hash
    #[serde(default)]
    pub blob_refs: HashMap<String, Hash>,
    /// History of asset changes (limited to last 100 entries)
    #[serde(default)]
    pub history: Vec<AssetHistoryEntry>,
    /// Version snapshots (limited to last 10 versions)
    #[serde(default)]
    pub versions: Vec<AssetVersion>,
    /// Current version number (increments on each snapshot)
    #[serde(default)]
    pub current_version: u64,
    /// Permission grants (GameContract, PublicRead)
    #[serde(default)]
    pub permissions: Vec<AssetPermission>,
    /// If true, anyone can read the asset
    #[serde(default)]
    pub public_read: bool,
}

impl StateManager {
    /// Create a new StateManager
    ///
    /// Initializes the state manager with an empty state, default tokenomics, and economy.
    ///
    /// # Arguments
    /// * `config` - Configuration for the node, including database path
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened.
    ///
    /// # Example
    /// ```no_run
    /// use haze::config::Config;
    /// use haze::state::StateManager;
    /// use std::path::PathBuf;
    ///
    /// let mut config = Config::default();
    /// // Use a unique database path to avoid conflicts
    /// config.storage.db_path = PathBuf::from("./haze_db_doctest");
    /// let state_manager = StateManager::new(&config).unwrap();
    /// assert_eq!(state_manager.current_height(), 0);
    /// ```
    pub fn new(config: &Config) -> Result<Self> {
        let db = sled::open(&config.storage.db_path)
            .map_err(|e| HazeError::Database(format!("Failed to open database: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
            config: Arc::new(config.clone()),
            accounts: Arc::new(DashMap::new()),
            assets: Arc::new(DashMap::new()),
            blocks: Arc::new(DashMap::new()),
            current_height: Arc::new(RwLock::new(0)),
            tokenomics: Arc::new(Tokenomics::new()),
            economy: Arc::new(FogEconomy::new()),
            ws_tx: Arc::new(RwLock::new(None)),
            asset_index_by_owner: Arc::new(DashMap::new()),
            asset_index_by_game_id: Arc::new(DashMap::new()),
            asset_index_by_density: Arc::new(DashMap::new()),
            asset_access_count: Arc::new(DashMap::new()),
        })
    }

    /// Set WebSocket broadcaster for real-time event notifications
    pub fn set_ws_tx(&self, tx: broadcast::Sender<WsEvent>) {
        *self.ws_tx.write() = Some(tx);
    }

    /// Broadcast WebSocket event if broadcaster is available
    fn broadcast_event(&self, event: WsEvent) {
        if let Some(ref tx) = *self.ws_tx.read() {
            let _ = tx.send(event);
        }
    }

    /// Add history entry to asset state (limited to last 100 entries)
    fn add_asset_history(asset_state: &mut AssetState, action: AssetAction, changes: HashMap<String, String>) {
        let history_entry = AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action,
            changes,
        };
        
        asset_state.history.push(history_entry);
        
        // Limit history to last 100 entries
        if asset_state.history.len() > 100 {
            asset_state.history.remove(0);
        }
    }

    /// Create a version snapshot from asset state
    fn create_version_from_state(asset_state: &AssetState) -> AssetVersion {
        AssetVersion {
            version: asset_state.current_version + 1,
            timestamp: chrono::Utc::now().timestamp(),
            data: asset_state.data.clone(),
            blob_refs: asset_state.blob_refs.clone(),
        }
    }

    /// Add snapshot to asset state (limited to last 10 versions)
    fn add_asset_snapshot(asset_state: &mut AssetState) {
        let snapshot = Self::create_version_from_state(asset_state);
        asset_state.current_version = snapshot.version;
        asset_state.versions.push(snapshot);
        
        // Limit versions to last 10
        if asset_state.versions.len() > 10 {
            asset_state.versions.remove(0);
        }
    }

    /// Add asset to indexes (only if not already present)
    fn add_asset_to_indexes(&self, asset_id: &Hash, asset_state: &AssetState) {
        // Index by owner (optimized: check before adding to avoid unnecessary clone)
        self.asset_index_by_owner
            .entry(asset_state.owner)
            .or_insert_with(Vec::new)
            .push(*asset_id);
        
        // Index by game_id
        if let Some(ref game_id) = asset_state.data.game_id {
            self.asset_index_by_game_id
                .entry(game_id.clone())
                .or_insert_with(Vec::new)
                .push(*asset_id);
        }
        
        // Index by density
        let density_level = asset_state.data.density as u8;
        let mut density_assets = self.asset_index_by_density
            .entry(density_level)
            .or_insert_with(Vec::new);
        if !density_assets.contains(asset_id) {
            density_assets.push(*asset_id);
        }
    }

    /// Remove asset from indexes
    fn remove_asset_from_indexes(&self, asset_id: &Hash, asset_state: &AssetState) {
        // Remove from owner index
        if let Some(mut owner_assets) = self.asset_index_by_owner.get_mut(&asset_state.owner) {
            owner_assets.retain(|&id| id != *asset_id);
            if owner_assets.is_empty() {
                drop(owner_assets);
                self.asset_index_by_owner.remove(&asset_state.owner);
            }
        }
        
        // Remove from game_id index
        if let Some(ref game_id) = asset_state.data.game_id {
            if let Some(mut game_assets) = self.asset_index_by_game_id.get_mut(game_id) {
                game_assets.retain(|&id| id != *asset_id);
                if game_assets.is_empty() {
                    drop(game_assets);
                    self.asset_index_by_game_id.remove(game_id);
                }
            }
        }
        
        // Remove from density index
        let density_level = asset_state.data.density as u8;
        if let Some(mut density_assets) = self.asset_index_by_density.get_mut(&density_level) {
            density_assets.retain(|&id| id != *asset_id);
            if density_assets.is_empty() {
                drop(density_assets);
                self.asset_index_by_density.remove(&density_level);
            }
        }
    }

    /// Get account state by address
    ///
    /// # Arguments
    /// * `address` - The account address
    ///
    /// # Returns
    /// `Some(AccountState)` if the account exists, `None` otherwise.
    ///
    /// # Example
    /// ```
    /// use haze::crypto::KeyPair;
    /// use haze::state::StateManager;
    /// use haze::config::Config;
    ///
    /// let config = Config::default();
    /// let state = StateManager::new(&config)?;
    /// let keypair = KeyPair::generate();
    /// let address = keypair.address();
    ///
    /// // New account doesn't exist yet
    /// assert!(state.get_account(&address).is_none());
    /// # Ok::<(), haze::error::HazeError>(())
    /// ```
    pub fn get_account(&self, address: &Address) -> Option<AccountState> {
        self.accounts.get(address).map(|v| v.clone())
    }

    /// Get asset state by asset ID
    ///
    /// # Arguments
    /// * `asset_id` - The asset identifier (hash)
    ///
    /// # Returns
    /// `Some(AssetState)` if the asset exists, `None` otherwise.
    ///
    /// # Performance
    /// This method tracks access frequency for cache optimization.
    pub fn get_asset(&self, asset_id: &Hash) -> Option<AssetState> {
        let result = self.assets.get(asset_id).map(|v| v.clone());
        
        // Track access frequency for cache optimization
        if result.is_some() {
            *self.asset_access_count.entry(*asset_id).or_insert(0) += 1;
        }
        
        result
    }
    
    /// Get asset state without blob data (lazy loading)
    ///
    /// # Arguments
    /// * `asset_id` - The asset identifier (hash)
    ///
    /// # Returns
    /// `Some(AssetState)` if the asset exists, `None` otherwise.
    /// Blob references are included but blob data is not loaded.
    pub fn get_asset_lightweight(&self, asset_id: &Hash) -> Option<AssetState> {
        // Same as get_asset, but explicitly indicates lazy blob loading
        self.get_asset(asset_id)
    }
    
    /// Get most frequently accessed assets
    ///
    /// # Arguments
    /// * `limit` - Maximum number of assets to return
    ///
    /// # Returns
    /// Vector of (asset_id, access_count) tuples sorted by access frequency
    pub fn get_most_accessed_assets(&self, limit: usize) -> Vec<(Hash, u64)> {
        let mut access_list: Vec<(Hash, u64)> = self.asset_access_count
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        
        access_list.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by access count descending
        access_list.truncate(limit);
        access_list
    }

    /// Get asset history by asset ID
    ///
    /// # Arguments
    /// * `asset_id` - The asset identifier (hash)
    /// * `limit` - Maximum number of history entries to return (0 = all)
    ///
    /// # Returns
    /// `Some(Vec<AssetHistoryEntry>)` if the asset exists, `None` otherwise.
    pub fn get_asset_history(&self, asset_id: &Hash, limit: usize) -> Option<Vec<AssetHistoryEntry>> {
        self.assets.get(asset_id).map(|asset_state| {
            let history = asset_state.history.clone();
            if limit > 0 && history.len() > limit {
                history.into_iter().rev().take(limit).rev().collect()
            } else {
                history
            }
        })
    }

    /// Get asset version by version number
    ///
    /// # Arguments
    /// * `asset_id` - The asset identifier (hash)
    /// * `version` - The version number (0 = current version)
    ///
    /// # Returns
    /// `Some(AssetVersion)` if the version exists, `None` otherwise.
    pub fn get_asset_version(&self, asset_id: &Hash, version: u64) -> Option<AssetVersion> {
        self.assets.get(asset_id).and_then(|asset_state| {
            if version == 0 || version == asset_state.current_version {
                // Return current version
                Some(AssetVersion {
                    version: asset_state.current_version,
                    timestamp: asset_state.updated_at,
                    data: asset_state.data.clone(),
                    blob_refs: asset_state.blob_refs.clone(),
                })
            } else {
                // Find version in history
                asset_state.versions.iter()
                    .find(|v| v.version == version)
                    .cloned()
            }
        })
    }

    /// Get all versions of an asset
    pub fn get_asset_versions(&self, asset_id: &Hash) -> Option<Vec<AssetVersion>> {
        self.assets.get(asset_id).map(|asset_state| {
            let mut versions = asset_state.versions.clone();
            // Add current version only if it's not already in versions
            let current_exists = versions.iter().any(|v| v.version == asset_state.current_version);
            if !current_exists {
                versions.push(AssetVersion {
                    version: asset_state.current_version,
                    timestamp: asset_state.updated_at,
                    data: asset_state.data.clone(),
                    blob_refs: asset_state.blob_refs.clone(),
                });
            }
            versions.sort_by_key(|v| v.version);
            versions
        })
    }

    /// Create a manual snapshot of an asset
    pub fn create_asset_snapshot(&self, asset_id: &Hash) -> Result<u64> {
        let mut asset_state = self.assets.get_mut(asset_id)
            .ok_or_else(|| HazeError::InvalidTransaction(
                "Asset not found".to_string()
            ))?;
        
        Self::add_asset_snapshot(&mut asset_state);
        let version = asset_state.current_version;
        let owner = asset_state.owner;
        self.broadcast_event(WsEvent::AssetVersionCreated {
            asset_id: hex::encode(asset_id),
            version,
            owner: hex::encode(owner),
        });
        Ok(version)
    }

    /// Search assets by owner
    ///
    /// # Returns
    /// Vector of asset IDs owned by the address (sorted by creation time, most recent first)
    pub fn search_assets_by_owner(&self, owner: &Address) -> Vec<Hash> {
        let mut assets = self.asset_index_by_owner
            .get(owner)
            .map(|v| v.clone())
            .unwrap_or_default();
        
        // Sort by creation time (most recent first) for better UX
        assets.sort_by(|a, b| {
            let time_a = self.assets.get(a).map(|s| s.created_at).unwrap_or(0);
            let time_b = self.assets.get(b).map(|s| s.created_at).unwrap_or(0);
            time_b.cmp(&time_a) // Descending order
        });
        
        assets
    }

    /// Search assets by game_id
    ///
    /// # Returns
    /// Vector of asset IDs for the game (sorted by creation time, most recent first)
    pub fn search_assets_by_game_id(&self, game_id: &str) -> Vec<Hash> {
        let mut assets = self.asset_index_by_game_id
            .get(game_id)
            .map(|v| v.clone())
            .unwrap_or_default();
        
        // Sort by creation time (most recent first) for better UX
        assets.sort_by(|a, b| {
            let time_a = self.assets.get(a).map(|s| s.created_at).unwrap_or(0);
            let time_b = self.assets.get(b).map(|s| s.created_at).unwrap_or(0);
            time_b.cmp(&time_a) // Descending order
        });
        
        assets
    }

    /// Search assets by density level
    ///
    /// # Returns
    /// Vector of asset IDs with the specified density (sorted by creation time, most recent first)
    pub fn search_assets_by_density(&self, density: crate::types::DensityLevel) -> Vec<Hash> {
        let density_level = density as u8;
        let mut assets = self.asset_index_by_density
            .get(&density_level)
            .map(|v| v.clone())
            .unwrap_or_default();
        
        // Sort by creation time (most recent first) for better UX
        assets.sort_by(|a, b| {
            let time_a = self.assets.get(a).map(|s| s.created_at).unwrap_or(0);
            let time_b = self.assets.get(b).map(|s| s.created_at).unwrap_or(0);
            time_b.cmp(&time_a) // Descending order
        });
        
        assets
    }

    /// Full-text search in metadata (simple substring matching)
    pub fn search_assets_by_metadata(&self, query: &str) -> Vec<Hash> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        for entry in self.assets.iter() {
            let asset_state = entry.value();
            // Search in metadata values
            for value in asset_state.data.metadata.values() {
                if value.to_lowercase().contains(&query_lower) {
                    results.push(*entry.key());
                    break; // Found match, no need to check other values for this asset
                }
            }
        }
        
        results
    }

    /// Check if account has reached asset limit
    ///
    /// # Arguments
    /// * `owner` - Account address
    ///
    /// # Returns
    /// `Ok(())` if within limits, `Err(HazeError)` if limit exceeded
    fn check_asset_count_limit(&self, owner: &Address) -> Result<()> {
        let quota = self.config.get_node_quota();
        let current_count = self.search_assets_by_owner(owner).len() as u64;
        
        if current_count >= quota.max_assets_per_account {
            return Err(HazeError::InvalidTransaction(
                format!(
                    "Asset limit exceeded: {} >= {} (node type: {})",
                    current_count,
                    quota.max_assets_per_account,
                    self.config.network.node_type
                )
            ));
        }
        
        Ok(())
    }

    /// Check if metadata size is within limits
    ///
    /// # Arguments
    /// * `metadata_size` - Size of metadata in bytes
    ///
    /// # Returns
    /// `Ok(())` if within limits, `Err(HazeError)` if limit exceeded
    fn check_metadata_size_limit(&self, metadata_size: usize) -> Result<()> {
        let quota = self.config.get_node_quota();
        
        if metadata_size > quota.max_metadata_size {
            return Err(HazeError::AssetSizeExceeded(
                metadata_size,
                quota.max_metadata_size
            ));
        }
        
        Ok(())
    }

    /// Check if asset has reached blob file limit
    ///
    /// # Arguments
    /// * `asset_id` - Asset identifier
    /// * `additional_blobs` - Number of additional blob files to add
    ///
    /// # Returns
    /// `Ok(())` if within limits, `Err(HazeError)` if limit exceeded
    fn check_blob_files_limit(&self, asset_id: &Hash, additional_blobs: u64) -> Result<()> {
        let quota = self.config.get_node_quota();
        
        let current_blob_count = if let Some(asset_state) = self.assets.get(asset_id) {
            asset_state.blob_refs.len() as u64
        } else {
            0
        };
        
        let new_blob_count = current_blob_count + additional_blobs;
        
        if new_blob_count > quota.max_blob_files_per_asset {
            return Err(HazeError::InvalidTransaction(
                format!(
                    "Blob files limit exceeded: {} > {} (node type: {})",
                    new_blob_count,
                    quota.max_blob_files_per_asset,
                    self.config.network.node_type
                )
            ));
        }
        
        Ok(())
    }

    /// Check if account has reached blob storage limit
    ///
    /// # Arguments
    /// * `owner` - Account address
    /// * `additional_size` - Additional blob storage size in bytes
    ///
    /// # Returns
    /// `Ok(())` if within limits, `Err(HazeError)` if limit exceeded
    fn check_blob_storage_limit(&self, owner: &Address, additional_size: u64) -> Result<()> {
        let quota = self.config.get_node_quota();
        
        // Calculate current blob storage for this account
        let mut current_storage: u64 = 0;
        let owner_assets = self.search_assets_by_owner(owner);
        
        for asset_id in &owner_assets {
            if let Some(asset_state) = self.assets.get(asset_id) {
                // Estimate blob storage size (we don't have exact sizes, so use blob count * average)
                // This is a conservative estimate
                let blob_count = asset_state.blob_refs.len() as u64;
                current_storage += blob_count * 1024 * 1024; // Estimate 1MB per blob
            }
        }
        
        let new_storage = current_storage + additional_size;
        
        if new_storage > quota.max_blob_storage_per_account {
            return Err(HazeError::InvalidTransaction(
                format!(
                    "Blob storage limit exceeded: {} > {} bytes (node type: {})",
                    new_storage,
                    quota.max_blob_storage_per_account,
                    self.config.network.node_type
                )
            ));
        }
        
        Ok(())
    }

    /// Check if caller has write permission for an asset (Update, Condense, Evaporate, Merge, Split).
    /// Owner always has full access. GameContract grantees have limited access when game_id matches.
    fn check_asset_write_permission(
        &self,
        asset_state: &AssetState,
        caller: &Address,
    ) -> Result<()> {
        if asset_state.owner == *caller {
            return Ok(());
        }
        let now = chrono::Utc::now().timestamp();
        for p in &asset_state.permissions {
            if p.grantee != *caller {
                continue;
            }
            if p.level != PermissionLevel::GameContract {
                continue;
            }
            if let Some(ref exp) = p.expires_at {
                if now > *exp {
                    continue;
                }
            }
            match (&p.game_id, &asset_state.data.game_id) {
                (Some(perm_gid), Some(asset_gid)) if perm_gid == asset_gid => return Ok(()),
                (None, _) => return Ok(()), // No game restriction: allow any game
                _ => {}
            }
        }
        Err(HazeError::AccessDenied(
            "Caller is not owner and has no GameContract permission".to_string(),
        ))
    }

    /// Get quota usage for an account
    ///
    /// # Arguments
    /// * `owner` - Account address
    ///
    /// # Returns
    /// Quota usage information
    pub fn get_quota_usage(&self, owner: &Address) -> QuotaUsage {
        let quota = self.config.get_node_quota();
        let owner_assets = self.search_assets_by_owner(owner);
        
        let mut total_blob_files = 0;
        let mut total_blob_storage_estimate = 0u64;
        
        for asset_id in &owner_assets {
            if let Some(asset_state) = self.assets.get(asset_id) {
                let blob_count = asset_state.blob_refs.len() as u64;
                total_blob_files += blob_count;
                total_blob_storage_estimate += blob_count * 1024 * 1024; // Estimate 1MB per blob
            }
        }
        
        QuotaUsage {
            assets_count: owner_assets.len() as u64,
            assets_limit: quota.max_assets_per_account,
            blob_files_count: total_blob_files,
            blob_files_limit: quota.max_blob_files_per_asset, // Per asset, but we show total
            blob_storage_estimate: total_blob_storage_estimate,
            blob_storage_limit: quota.max_blob_storage_per_account,
            metadata_size_limit: quota.max_metadata_size,
        }
    }

    /// Get block by hash
    pub fn get_block(&self, hash: &Hash) -> Option<Block> {
        self.blocks.get(hash).map(|v| v.clone())
    }
    
    /// Get block by height
    /// Note: This is O(n) operation. In production, use an index for O(1) lookup.
    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        for entry in self.blocks.iter() {
            if entry.value().header.height == height {
                return Some(entry.value().clone());
            }
        }
        None
    }

    /// Get current height
    pub fn current_height(&self) -> u64 {
        *self.current_height.read()
    }

    /// Apply block to state
    pub fn apply_block(&self, block: &Block) -> Result<()> {
        // Process block rewards and inflation
        let block_reward = self.tokenomics.process_block_rewards(block.header.height)?;
        
        // Distribute rewards to validator
        if block_reward > 0 {
            self.tokenomics.distribute_rewards(block_reward, block.header.validator)?;
        }
        
        // Validate block
        // Apply transactions
        for tx in &block.transactions {
            self.apply_transaction(tx)?;
        }

        // Store block
        let block_hash = block.header.hash;
        self.blocks.insert(block_hash, block.clone());
        
        // Update height
        *self.current_height.write() = block.header.height;

        Ok(())
    }

    /// Apply transaction to state
    fn apply_transaction(&self, tx: &Transaction) -> Result<()> {
        match tx {
            Transaction::Transfer { from, to, amount, fee, nonce, .. } => {
                let mut from_account = self.accounts
                    .entry(*from)
                    .or_insert_with(|| AccountState {
                        balance: 0,
                        nonce: 0,
                        staked: 0,
                    });
                
                // Verify nonce is sequential
                let expected_nonce = from_account.nonce;
                if *nonce != expected_nonce {
                    return Err(HazeError::InvalidTransaction(
                        format!(
                            "Invalid nonce in transaction: expected {}, got {}",
                            expected_nonce, nonce
                        )
                    ));
                }
                
                if from_account.balance < *amount + *fee {
                    return Err(HazeError::InvalidTransaction("Insufficient balance".to_string()));
                }

                from_account.balance -= amount + fee;
                from_account.nonce = *nonce + 1; // Update to next expected nonce

                let mut to_account = self.accounts
                    .entry(*to)
                    .or_insert_with(|| AccountState {
                        balance: 0,
                        nonce: 0,
                        staked: 0,
                    });
                
                to_account.balance += amount;
                
                // Process gas fee (burn 50%)
                let _remaining_fee = self.tokenomics.process_gas_fee(*fee)?;
            }
            Transaction::MistbornAsset { action, asset_id, data, .. } => {
                // Calculate gas cost for this operation
                let gas_cost = crate::assets::calculate_asset_operation_gas(
                    &self.config,
                    action,
                    data,
                    Some(&data.metadata),
                );
                
                // Calculate gas fee (gas_cost * gas_price)
                let gas_fee = gas_cost * self.config.vm.gas_price;
                
                // Check owner balance and deduct gas fee
                let mut owner_account = self.accounts
                    .entry(data.owner)
                    .or_insert_with(|| AccountState {
                        balance: 0,
                        nonce: 0,
                        staked: 0,
                    });
                
                if owner_account.balance < gas_fee {
                    return Err(HazeError::InvalidTransaction(
                        format!("Insufficient balance for gas fee: need {}, have {}", gas_fee, owner_account.balance)
                    ));
                }
                
                owner_account.balance -= gas_fee;
                
                // Process gas fee (burn 50%)
                let _remaining_fee = self.tokenomics.process_gas_fee(gas_fee)?;
                
                match action {
                    crate::types::AssetAction::Create => {
                        // Check if asset already exists
                        if self.assets.contains_key(asset_id) {
                            return Err(HazeError::InvalidTransaction(
                                "Asset already exists".to_string()
                            ));
                        }
                        
                        // Check asset count limit for owner
                        self.check_asset_count_limit(&data.owner)?;
                        
                        // Validate metadata size
                        let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
                        
                        // Check against density limit
                        if metadata_size > data.density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                metadata_size,
                                data.density.max_size()
                            ));
                        }
                        
                        // Check against node quota limit
                        self.check_metadata_size_limit(metadata_size)?;
                        
                        // Validate metadata keys (no empty keys, reasonable length)
                        for (key, value) in &data.metadata {
                            if key.is_empty() || key.len() > 256 {
                                return Err(HazeError::InvalidMetadataFormat(
                                    format!("Invalid metadata key: '{}'", key)
                                ));
                            }
                            if value.len() > 10 * 1024 * 1024 { // 10MB max per value
                                return Err(HazeError::InvalidMetadataFormat(
                                    format!("Metadata value for '{}' exceeds 10MB limit", key)
                                ));
                            }
                        }
                        
                        // Parse blob_refs from metadata if present
                        let mut blob_refs = HashMap::new();
                        if let Some(blob_refs_json) = data.metadata.get("_blob_refs") {
                            match serde_json::from_str::<HashMap<String, String>>(blob_refs_json) {
                                Ok(blob_refs_map) => {
                                    for (key, hash_hex) in blob_refs_map {
                                        if let Ok(hash_bytes) = hex::decode(&hash_hex) {
                                            if hash_bytes.len() == 32 {
                                                let mut hash = [0u8; 32];
                                                hash.copy_from_slice(&hash_bytes);
                                                blob_refs.insert(key, hash);
                                            } else {
                                                return Err(HazeError::InvalidMetadataFormat(
                                                    format!("Invalid blob hash length for key '{}'", key)
                                                ));
                                            }
                                        } else {
                                            return Err(HazeError::InvalidMetadataFormat(
                                                format!("Invalid blob hash hex format for key '{}'", key)
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(HazeError::InvalidMetadataFormat(
                                        format!("Invalid _blob_refs JSON format: {}", e)
                                    ));
                                }
                            }
                        }
                        
                        let mut asset_state = AssetState {
                            owner: data.owner,
                            data: data.clone(),
                            created_at: chrono::Utc::now().timestamp(),
                            updated_at: chrono::Utc::now().timestamp(),
                            blob_refs,
                            history: Vec::new(),
                            versions: Vec::new(),
                            current_version: 0,
                            permissions: Vec::new(),
                            public_read: false,
                        };
                        
                        // Remove special metadata keys before storing
                        asset_state.data.metadata.remove("_blob_refs");
                        
                        // Add creation to history
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Create, HashMap::new());
                        
                        // Create initial snapshot
                        Self::add_asset_snapshot(&mut asset_state);
                        
                        // Add to indexes
                        self.add_asset_to_indexes(asset_id, &asset_state);
                        
                        self.assets.insert(*asset_id, asset_state);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetCreated {
                            asset_id: hex::encode(asset_id),
                            owner: hex::encode(data.owner),
                            density: format!("{:?}", data.density),
                        });
                    }
                    crate::types::AssetAction::Update => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        self.check_asset_write_permission(&asset_state, &data.owner)?;
                        
                        // Save owner before moving asset_state
                        let owner = asset_state.owner;
                        
                        // Validate new metadata size
                        let current_size: usize = asset_state.data.metadata.values().map(|v| v.len()).sum();
                        let new_metadata_size: usize = data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(_, v)| v.len())
                            .sum();
                        
                        let total_metadata_size = current_size + new_metadata_size;
                        
                        if total_metadata_size > asset_state.data.density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                total_metadata_size,
                                asset_state.data.density.max_size()
                            ));
                        }
                        
                        // Check against node quota limit
                        self.check_metadata_size_limit(total_metadata_size)?;
                        
                        // Validate metadata keys
                        for (key, value) in &data.metadata {
                            if !key.starts_with('_') {
                                if key.is_empty() || key.len() > 256 {
                                    return Err(HazeError::InvalidMetadataFormat(
                                        format!("Invalid metadata key: '{}'", key)
                                    ));
                                }
                                if value.len() > 10 * 1024 * 1024 {
                                    return Err(HazeError::InvalidMetadataFormat(
                                        format!("Metadata value for '{}' exceeds 10MB limit", key)
                                    ));
                                }
                            }
                        }
                        
                        // Process blob_refs from metadata if present
                        if let Some(blob_refs_json) = data.metadata.get("_blob_refs") {
                            match serde_json::from_str::<HashMap<String, String>>(blob_refs_json) {
                                Ok(blob_refs_map) => {
                                    for (key, hash_hex) in blob_refs_map {
                                        if let Ok(hash_bytes) = hex::decode(&hash_hex) {
                                            if hash_bytes.len() == 32 {
                                                let mut hash = [0u8; 32];
                                                hash.copy_from_slice(&hash_bytes);
                                                asset_state.blob_refs.insert(key, hash);
                                            } else {
                                                return Err(HazeError::InvalidMetadataFormat(
                                                    format!("Invalid blob hash length for key '{}'", key)
                                                ));
                                            }
                                        } else {
                                            return Err(HazeError::InvalidMetadataFormat(
                                                format!("Invalid blob hash hex format for key '{}'", key)
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(HazeError::InvalidMetadataFormat(
                                        format!("Invalid _blob_refs JSON format: {}", e)
                                    ));
                                }
                            }
                        }
                        
                        // Update metadata and attributes (excluding special keys)
                        for (key, value) in &data.metadata {
                            if !key.starts_with('_') {
                                asset_state.data.metadata.insert(key.clone(), value.clone());
                            }
                        }
                        asset_state.data.attributes = data.attributes.clone();
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        // Update indexes if game_id or density changed
                        let old_game_id = asset_state.data.game_id.clone();
                        let old_density = asset_state.data.density as u8;
                        
                        // Record changes in history
                        let changes: HashMap<String, String> = data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Update, changes);
                        
                        let attr_names: Vec<String> = asset_state.data.attributes.iter().map(|a| a.name.clone()).collect();
                        if !attr_names.is_empty() {
                            self.broadcast_event(WsEvent::AssetAttributeUpdated {
                                asset_id: hex::encode(asset_id),
                                owner: hex::encode(owner),
                                attributes: attr_names,
                            });
                        }
                        
                        // Update indexes if needed
                        let new_game_id = asset_state.data.game_id.clone();
                        let new_density = asset_state.data.density as u8;
                        
                        if old_game_id != new_game_id {
                            // Remove from old game_id index
                            if let Some(ref old_game) = old_game_id {
                                if let Some(mut game_assets) = self.asset_index_by_game_id.get_mut(old_game) {
                                    game_assets.retain(|&id| id != *asset_id);
                                }
                            }
                            // Add to new game_id index
                            if let Some(ref new_game) = new_game_id {
                                self.asset_index_by_game_id
                                    .entry(new_game.clone())
                                    .or_insert_with(Vec::new)
                                    .push(*asset_id);
                            }
                        }
                        
                        if old_density != new_density {
                            // Remove from old density index
                            if let Some(mut density_assets) = self.asset_index_by_density.get_mut(&old_density) {
                                density_assets.retain(|&id| id != *asset_id);
                            }
                            // Add to new density index
                            self.asset_index_by_density
                                .entry(new_density)
                                .or_insert_with(Vec::new)
                                .push(*asset_id);
                        }
                        
                        self.assets.insert(*asset_id, asset_state);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetUpdated {
                            asset_id: hex::encode(asset_id),
                            owner: hex::encode(owner),
                        });
                    }
                    crate::types::AssetAction::Condense => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        self.check_asset_write_permission(&asset_state, &data.owner)?;
                        
                        // Check if condensation is valid (can only increase density by one level)
                        let current_density = asset_state.data.density as u8;
                        let new_density = data.density as u8;
                        if new_density <= current_density {
                            return Err(HazeError::InvalidDensityTransition(
                                format!("{:?}", asset_state.data.density),
                                format!("{:?}", data.density)
                            ));
                        }
                        
                        // Validate density transition (can only go up by one level)
                        let expected_next = match asset_state.data.density {
                            crate::types::DensityLevel::Ethereal => crate::types::DensityLevel::Light,
                            crate::types::DensityLevel::Light => crate::types::DensityLevel::Dense,
                            crate::types::DensityLevel::Dense => crate::types::DensityLevel::Core,
                            crate::types::DensityLevel::Core => {
                                return Err(HazeError::InvalidDensityTransition(
                                    format!("{:?}", asset_state.data.density),
                                    format!("{:?}", data.density)
                                ));
                            }
                        };
                        
                        if data.density != expected_next {
                            return Err(HazeError::InvalidDensityTransition(
                                format!("{:?}", asset_state.data.density),
                                format!("{:?}", data.density)
                            ));
                        }
                        
                        // Validate new metadata size
                        let new_metadata_size: usize = data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(_, v)| v.len())
                            .sum();
                        
                        if new_metadata_size > data.density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                new_metadata_size,
                                data.density.max_size()
                            ));
                        }
                        
                        // Check against node quota limit
                        self.check_metadata_size_limit(new_metadata_size)?;
                        
                        // Update density and merge new data
                        let new_density_str = format!("{:?}", data.density);
                        let old_density = asset_state.data.density;
                        asset_state.data.density = data.density;
                        
                        // Process blob_refs from metadata (special key "_blob_refs" as JSON)
                        if let Some(blob_refs_json) = data.metadata.get("_blob_refs") {
                            if let Ok(blob_refs_map) = serde_json::from_str::<HashMap<String, String>>(blob_refs_json) {
                                let new_blob_refs_count = blob_refs_map.len() as u64;
                                
                                // Check blob files limit
                                self.check_blob_files_limit(asset_id, new_blob_refs_count)?;
                                
                                // Estimate blob storage size (conservative: 1MB per blob)
                                let estimated_blob_size = new_blob_refs_count * 1024 * 1024;
                                self.check_blob_storage_limit(&data.owner, estimated_blob_size)?;
                                
                                for (key, hash_hex) in blob_refs_map {
                                    if let Ok(hash_bytes) = hex::decode(&hash_hex) {
                                        if hash_bytes.len() == 32 {
                                            let mut hash = [0u8; 32];
                                            hash.copy_from_slice(&hash_bytes);
                                            asset_state.blob_refs.insert(key, hash);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Update metadata (excluding special keys)
                        for (key, value) in &data.metadata {
                            if !key.starts_with('_') {
                                asset_state.data.metadata.insert(key.clone(), value.clone());
                            }
                        }
                        asset_state.data.attributes.extend(data.attributes.clone());
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        // Record changes in history
                        let mut changes = HashMap::new();
                        changes.insert("old_density".to_string(), format!("{:?}", old_density));
                        changes.insert("new_density".to_string(), format!("{:?}", data.density));
                        for (key, value) in &data.metadata {
                            if !key.starts_with('_') {
                                changes.insert(format!("metadata.{}", key), value.clone());
                            }
                        }
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Condense, changes);
                        
                        // Create snapshot for important change (condense)
                        Self::add_asset_snapshot(&mut asset_state);
                        
                        // Update density index
                        let old_density = old_density as u8;
                        let new_density = asset_state.data.density as u8;
                        if old_density != new_density {
                            if let Some(mut density_assets) = self.asset_index_by_density.get_mut(&old_density) {
                                density_assets.retain(|&id| id != *asset_id);
                            }
                            self.asset_index_by_density
                                .entry(new_density)
                                .or_insert_with(Vec::new)
                                .push(*asset_id);
                        }
                        
                        self.assets.insert(*asset_id, asset_state);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetCondensed {
                            asset_id: hex::encode(asset_id),
                            new_density: new_density_str,
                        });
                    }
                    crate::types::AssetAction::Evaporate => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        self.check_asset_write_permission(&asset_state, &data.owner)?;
                        
                        // Check if evaporation is valid (can only decrease density by one level)
                        let current_density = asset_state.data.density as u8;
                        let new_density = data.density as u8;
                        if new_density >= current_density {
                            return Err(HazeError::InvalidDensityTransition(
                                format!("{:?}", asset_state.data.density),
                                format!("{:?}", data.density)
                            ));
                        }
                        
                        // Validate density transition (can only go down by one level)
                        let expected_prev = match asset_state.data.density {
                            crate::types::DensityLevel::Core => crate::types::DensityLevel::Dense,
                            crate::types::DensityLevel::Dense => crate::types::DensityLevel::Light,
                            crate::types::DensityLevel::Light => crate::types::DensityLevel::Ethereal,
                            crate::types::DensityLevel::Ethereal => {
                                return Err(HazeError::InvalidDensityTransition(
                                    format!("{:?}", asset_state.data.density),
                                    format!("{:?}", data.density)
                                ));
                            }
                        };
                        
                        if data.density != expected_prev {
                            return Err(HazeError::InvalidDensityTransition(
                                format!("{:?}", asset_state.data.density),
                                format!("{:?}", data.density)
                            ));
                        }
                        
                        // Update density
                        let new_density_str = format!("{:?}", data.density);
                        let old_density = asset_state.data.density;
                        asset_state.data.density = data.density;
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        // Record changes in history
                        let mut changes = HashMap::new();
                        changes.insert("old_density".to_string(), format!("{:?}", old_density));
                        changes.insert("new_density".to_string(), format!("{:?}", data.density));
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Evaporate, changes);
                        
                        // Update density index
                        let old_density = old_density as u8;
                        let new_density = asset_state.data.density as u8;
                        if old_density != new_density {
                            if let Some(mut density_assets) = self.asset_index_by_density.get_mut(&old_density) {
                                density_assets.retain(|&id| id != *asset_id);
                            }
                            self.asset_index_by_density
                                .entry(new_density)
                                .or_insert_with(Vec::new)
                                .push(*asset_id);
                        }
                        
                        self.assets.insert(*asset_id, asset_state);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetEvaporated {
                            asset_id: hex::encode(asset_id),
                            new_density: new_density_str,
                        });
                    }
                    crate::types::AssetAction::Merge => {
                        // Merge requires two assets
                        // Get other_asset_id from metadata (special key "_other_asset_id")
                        let other_asset_id_str = data.metadata.get("_other_asset_id")
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Merge operation requires '_other_asset_id' in metadata".to_string()
                            ))?;
                        
                        let other_asset_id_bytes = hex::decode(other_asset_id_str)
                            .map_err(|_| HazeError::InvalidTransaction(
                                "Invalid '_other_asset_id' format".to_string()
                            ))?;
                        
                        if other_asset_id_bytes.len() != 32 {
                            return Err(HazeError::InvalidTransaction(
                                "Invalid '_other_asset_id' length".to_string()
                            ));
                        }
                        
                        let mut other_asset_id = [0u8; 32];
                        other_asset_id.copy_from_slice(&other_asset_id_bytes);
                        
                        // Get both assets
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Source asset not found".to_string()
                            ))?
                            .clone();
                        
                        let other_asset_state = self.assets.get(&other_asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Other asset not found".to_string()
                            ))?
                            .clone();
                        
                        if asset_state.owner != other_asset_state.owner {
                            return Err(HazeError::AccessDenied(
                                "Cannot merge assets with different owners".to_string()
                            ));
                        }
                        self.check_asset_write_permission(&asset_state, &data.owner)?;
                        
                        // Validate merged size won't exceed Core density limit
                        let current_size: usize = asset_state.data.metadata.values().map(|v| v.len()).sum();
                        let other_size: usize = other_asset_state.data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(_, v)| v.len())
                            .sum();
                        let merged_metadata_size = current_size + other_size;
                        
                        let max_density = if asset_state.data.density as u8 > other_asset_state.data.density as u8 {
                            asset_state.data.density
                        } else {
                            other_asset_state.data.density
                        };
                        
                        if merged_metadata_size > max_density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                merged_metadata_size,
                                max_density.max_size()
                            ));
                        }
                        
                        // Merge metadata (excluding special keys)
                        for (key, value) in &other_asset_state.data.metadata {
                            if !key.starts_with('_') && !asset_state.data.metadata.contains_key(key) {
                                asset_state.data.metadata.insert(key.clone(), value.clone());
                            }
                        }
                        
                        // Merge attributes with conflict resolution
                        // If attribute with same name exists, keep the one with higher rarity
                        // If both have same rarity or both are None, keep the source asset's attribute
                        for other_attr in &other_asset_state.data.attributes {
                            if let Some(existing) = asset_state.data.attributes.iter_mut().find(|a| a.name == other_attr.name) {
                                // Conflict: attribute with same name exists
                                // Resolve by comparing rarity (higher rarity wins)
                                let should_replace = match (existing.rarity, other_attr.rarity) {
                                    (Some(existing_rarity), Some(other_rarity)) => other_rarity > existing_rarity,
                                    (None, Some(_)) => true, // Other has rarity, existing doesn't
                                    (Some(_), None) => false, // Existing has rarity, other doesn't
                                    (None, None) => false, // Both have no rarity, keep existing
                                };
                                
                                if should_replace {
                                    existing.value = other_attr.value.clone();
                                    existing.rarity = other_attr.rarity;
                                }
                            } else {
                                // No conflict, add the attribute
                                asset_state.data.attributes.push(other_attr.clone());
                            }
                        }
                        
                        // Merge blob_refs
                        for (key, hash) in &other_asset_state.blob_refs {
                            if !asset_state.blob_refs.contains_key(key) {
                                asset_state.blob_refs.insert(key.clone(), *hash);
                            }
                        }
                        
                        // Increase density if needed
                        if other_asset_state.data.density as u8 > asset_state.data.density as u8 {
                            asset_state.data.density = other_asset_state.data.density;
                        }
                        
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        // Record changes in history
                        let mut changes = HashMap::new();
                        changes.insert("merged_asset_id".to_string(), hex::encode(other_asset_id));
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Merge, changes);
                        
                        // Create snapshot for important change (merge)
                        Self::add_asset_snapshot(&mut asset_state);
                        
                        // Update source asset
                        self.assets.insert(*asset_id, asset_state.clone());
                        
                        // Update indexes for merged asset (density might have changed)
                        let old_density = asset_state.data.density as u8;
                        let new_density = asset_state.data.density as u8;
                        if old_density != new_density {
                            if let Some(mut density_assets) = self.asset_index_by_density.get_mut(&old_density) {
                                density_assets.retain(|&id| id != *asset_id);
                            }
                            self.asset_index_by_density
                                .entry(new_density)
                                .or_insert_with(Vec::new)
                                .push(*asset_id);
                        }
                        
                        // Remove merged asset from indexes and state
                        if let Some(other_state) = self.assets.get(&other_asset_id) {
                            self.remove_asset_from_indexes(&other_asset_id, &other_state);
                        }
                        self.assets.remove(&other_asset_id);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetMerged {
                            asset_id: hex::encode(asset_id),
                            merged_asset_id: hex::encode(other_asset_id),
                        });
                    }
                    crate::types::AssetAction::Split => {
                        // Split creates new assets from components
                        // Get components list from metadata (special key "_components")
                        let components_str = data.metadata.get("_components")
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Split operation requires '_components' in metadata".to_string()
                            ))?;
                        
                        // Parse components (comma-separated list)
                        let components: Vec<String> = components_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        
                        if components.is_empty() {
                            return Err(HazeError::InvalidTransaction(
                                "Split requires at least one component".to_string()
                            ));
                        }
                        
                        // Get source asset
                        let source_asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Source asset not found".to_string()
                            ))?
                            .clone();
                        
                        self.check_asset_write_permission(&source_asset_state, &data.owner)?;
                        
                        // Validate component count (reasonable limit)
                        if components.len() > 100 {
                            return Err(HazeError::InvalidTransaction(
                                "Split operation cannot create more than 100 components".to_string()
                            ));
                        }
                        
                        // Create new assets for each component
                        let mut created_asset_ids = Vec::new();
                        
                        for component_name in &components {
                            let mut component_data = crate::types::AssetData {
                                density: crate::types::DensityLevel::Ethereal, // Start with minimum density
                                metadata: std::collections::HashMap::new(),
                                attributes: vec![],
                                game_id: source_asset_state.data.game_id.clone(),
                                owner: source_asset_state.owner,
                            };
                            
                            // Extract component-specific metadata
                            if let Some(value) = source_asset_state.data.metadata.get(component_name) {
                                component_data.metadata.insert(component_name.clone(), value.clone());
                            }
                            
                            // Distribute attributes to components
                            // Attributes with names matching component pattern go to that component
                            // Other attributes are copied to all components (shared attributes)
                            for attr in &source_asset_state.data.attributes {
                                // If attribute name contains component name, assign to this component
                                if attr.name.contains(component_name) || attr.name == *component_name {
                                    component_data.attributes.push(attr.clone());
                                } else if attr.name.starts_with("shared_") || attr.name == "rarity" || attr.name == "power" {
                                    // Shared attributes (like rarity, power) go to all components
                                    component_data.attributes.push(attr.clone());
                                }
                                // Otherwise, attribute is not assigned to this component
                            }
                            
                            // If no component-specific attributes were found, copy all attributes
                            // This ensures components have at least some attributes
                            if component_data.attributes.is_empty() {
                                component_data.attributes = source_asset_state.data.attributes.clone();
                            }
                            
                            // Generate component asset ID
                            let component_asset_id = crate::types::sha256(&[
                                asset_id.as_ref(),
                                component_name.as_bytes(),
                            ].concat());
                            
                            // Create component asset state
                            let mut component_asset_state = AssetState {
                                owner: source_asset_state.owner,
                                data: component_data,
                                created_at: chrono::Utc::now().timestamp(),
                                updated_at: chrono::Utc::now().timestamp(),
                                blob_refs: HashMap::new(), // Components start with empty blob_refs
                                history: Vec::new(),
                                versions: Vec::new(),
                                current_version: 0,
                                permissions: Vec::new(),
                                public_read: false,
                            };
                            
                            // Add creation to history
                            let mut changes = HashMap::new();
                            changes.insert("source_asset_id".to_string(), hex::encode(asset_id));
                            changes.insert("component_name".to_string(), component_name.clone());
                            Self::add_asset_history(&mut component_asset_state, crate::types::AssetAction::Split, changes);
                            
                            // Create initial snapshot for component
                            Self::add_asset_snapshot(&mut component_asset_state);
                            
                            // Add component to indexes
                            self.add_asset_to_indexes(&component_asset_id, &component_asset_state);
                            
                            self.assets.insert(component_asset_id, component_asset_state);
                            created_asset_ids.push(hex::encode(component_asset_id));
                        }
                        
                        // Record split in source asset history before removing
                        if let Some(mut source_state) = self.assets.get_mut(asset_id) {
                            let mut changes = HashMap::new();
                            changes.insert("components".to_string(), components_str.clone());
                            changes.insert("created_assets".to_string(), created_asset_ids.join(","));
                            Self::add_asset_history(&mut source_state, crate::types::AssetAction::Split, changes);
                        }
                        
                        // Remove source asset from indexes and state
                        if let Some(source_state) = self.assets.get(asset_id) {
                            self.remove_asset_from_indexes(asset_id, &source_state);
                        }
                        self.assets.remove(asset_id);
                        
                        // Broadcast WebSocket event
                        self.broadcast_event(WsEvent::AssetSplit {
                            asset_id: hex::encode(asset_id),
                            created_assets: created_asset_ids,
                        });
                    }
                }
            }
            Transaction::Stake { validator, amount, .. } => {
                let mut account = self.accounts
                    .entry(*validator)
                    .or_insert_with(|| AccountState {
                        balance: 0,
                        nonce: 0,
                        staked: 0,
                    });
                
                if account.balance < *amount {
                    return Err(HazeError::InvalidTransaction("Insufficient balance for staking".to_string()));
                }

                account.balance -= amount;
                account.staked += amount;
                
                // Register stake in tokenomics
                self.tokenomics.stake(*validator, *validator, *amount)?;
            }
            Transaction::SetAssetPermissions { asset_id, permissions, public_read, owner, .. } => {
                let mut asset_state = self.assets.get(asset_id)
                    .ok_or_else(|| HazeError::InvalidTransaction("Asset not found".to_string()))?
                    .clone();
                if asset_state.owner != *owner {
                    return Err(HazeError::AccessDenied(
                        "Only asset owner can set permissions".to_string()
                    ));
                }
                asset_state.permissions = permissions.clone();
                asset_state.public_read = *public_read;
                asset_state.updated_at = chrono::Utc::now().timestamp();
                self.assets.insert(*asset_id, asset_state);
                self.broadcast_event(WsEvent::AssetPermissionChanged {
                    asset_id: hex::encode(asset_id),
                    owner: hex::encode(owner),
                });
            }
            _ => {
                // Contract calls handled by VM
            }
        }
        
        Ok(())
    }
    
    /// Apply multiple transactions in batch (optimized)
    ///
    /// # Arguments
    /// * `transactions` - Vector of transactions to apply
    ///
    /// # Returns
    /// `Ok(())` if all transactions were applied successfully, `Err` with first error otherwise
    ///
    /// # Performance
    /// This method is optimized for batch operations by reducing index updates overhead.
    pub fn apply_transactions_batch(&self, transactions: &[Transaction]) -> Result<()> {
        // Apply all transactions
        for tx in transactions {
            self.apply_transaction(tx)?;
        }
        
        Ok(())
    }
    
    /// Batch create assets (optimized for multiple asset creation)
    ///
    /// # Arguments
    /// * `assets` - Vector of (asset_id, asset_data) tuples
    ///
    /// # Returns
    /// `Ok(())` if all assets were created successfully, `Err` with first error otherwise
    ///
    /// # Performance
    /// This method is optimized for batch creation by batching index updates.
    pub fn batch_create_assets(&self, assets: Vec<(Hash, AssetState)>) -> Result<()> {
        // Validate all assets first
        for (asset_id, asset_state) in &assets {
            // Check if asset already exists
            if self.assets.contains_key(asset_id) {
                return Err(HazeError::InvalidTransaction(
                    format!("Asset {} already exists", hex::encode(asset_id))
                ));
            }
            
            // Check asset count limit
            self.check_asset_count_limit(&asset_state.owner)?;
            
            // Check metadata size limit
            let metadata_size: usize = asset_state.data.metadata.values().map(|v| v.len()).sum();
            self.check_metadata_size_limit(metadata_size)?;
        }
        
        // Apply all assets in batch
        for (asset_id, asset_state) in assets {
            // Add to indexes
            self.add_asset_to_indexes(&asset_id, &asset_state);
            
            // Insert asset
            self.assets.insert(asset_id, asset_state);
        }
        
        Ok(())
    }

    /// Get tokenomics instance
    pub fn tokenomics(&self) -> &Arc<Tokenomics> {
        &self.tokenomics
    }

    /// Get economy instance
    pub fn economy(&self) -> &Arc<FogEconomy> {
        &self.economy
    }

    /// Get assets map (for API access)
    pub fn assets(&self) -> &Arc<DashMap<Hash, AssetState>> {
        &self.assets
    }

    /// Get blocks map (for API access)
    pub fn blocks(&self) -> &Arc<DashMap<Hash, Block>> {
        &self.blocks
    }

    /// Create test account (for testing only)
    /// 
    /// # Safety
    /// This method bypasses normal transaction validation and should only be used in tests.
    /// Available in test builds and integration tests.
    pub fn create_test_account(&self, address: Address, balance: u64, nonce: u64) {
        let account = AccountState {
            balance,
            nonce,
            staked: 0,
        };
        self.accounts.insert(address, account);
    }

    /// Compute state root hash
    /// This creates a hash of the current state (accounts + assets)
    pub fn compute_state_root(&self) -> Hash {
        use crate::types::sha256;
        use bincode;
        
        // Collect all account states
        let mut account_data = Vec::new();
        for entry in self.accounts.iter() {
            let account_bytes = bincode::serialize(&(*entry.key(), entry.value()))
                .unwrap_or_default();
            account_data.push(account_bytes);
        }
        account_data.sort();
        
        // Collect all asset states
        let mut asset_data = Vec::new();
        for entry in self.assets.iter() {
            let asset_bytes = bincode::serialize(&(*entry.key(), entry.value()))
                .unwrap_or_default();
            asset_data.push(asset_bytes);
        }
        asset_data.sort();
        
        // Combine and hash
        let mut combined = Vec::new();
        combined.extend(bincode::serialize(&account_data).unwrap_or_default());
        combined.extend(bincode::serialize(&asset_data).unwrap_or_default());
        combined.extend(bincode::serialize(&self.current_height()).unwrap_or_default());
        
        sha256(&combined)
    }
}

impl Clone for StateManager {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            config: self.config.clone(),
            accounts: self.accounts.clone(),
            assets: self.assets.clone(),
            blocks: self.blocks.clone(),
            current_height: self.current_height.clone(),
            tokenomics: self.tokenomics.clone(),
            economy: self.economy.clone(),
            ws_tx: self.ws_tx.clone(),
            asset_index_by_owner: self.asset_index_by_owner.clone(),
            asset_index_by_game_id: self.asset_index_by_game_id.clone(),
            asset_index_by_density: self.asset_index_by_density.clone(),
            asset_access_count: self.asset_access_count.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Address;
    use std::path::PathBuf;

    fn create_test_address(seed: u8) -> Address {
        let mut addr = [0u8; 32];
        addr[0] = seed;
        addr
    }

    fn create_test_config(test_name: &str) -> Config {
        let mut config = Config::default();
        // Use unique database path for each test
        let test_db_path = format!("./haze_db_test_{}", test_name);
        config.storage.db_path = PathBuf::from(test_db_path);
        config
    }

    #[test]
    fn test_state_manager_new() {
        let config = create_test_config("new");
        let state_manager = StateManager::new(&config).unwrap();
        
        assert_eq!(state_manager.current_height(), 0);
        assert_eq!(state_manager.tokenomics().total_supply(), crate::tokenomics::INITIAL_SUPPLY);
    }

    #[test]
    fn test_get_account_nonexistent() {
        let config = create_test_config("get_account");
        let state_manager = StateManager::new(&config).unwrap();
        let address = create_test_address(1);
        
        // Account should not exist
        assert!(state_manager.get_account(&address).is_none());
    }

    #[test]
    fn test_compute_state_root() {
        let config = create_test_config("state_root");
        let state_manager = StateManager::new(&config).unwrap();
        
        // Compute state root for empty state
        let state_root1 = state_manager.compute_state_root();
        assert_ne!(state_root1, [0u8; 32]);
        
        // State root should be consistent for same state
        let state_root2 = state_manager.compute_state_root();
        assert_eq!(state_root1, state_root2);
    }

    #[test]
    fn test_current_height() {
        let config = create_test_config("height");
        let state_manager = StateManager::new(&config).unwrap();
        
        // Initial height should be 0
        assert_eq!(state_manager.current_height(), 0);
    }

    #[test]
    fn test_merge_assets() {
        let config = create_test_config("merge");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        // Create first asset
        let asset_id_1 = crate::types::sha256(b"asset1");
        let tx1 = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id: asset_id_1,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("name".to_string(), "Asset 1".to_string());
                    m.insert("type".to_string(), "sword".to_string());
                    m
                },
                attributes: vec![crate::types::Attribute {
                    name: "damage".to_string(),
                    value: "10".to_string(),
                    rarity: None,
                }],
                game_id: Some("game1".to_string()),
                owner,
            },
            signature: vec![1; 64], // Dummy signature for test
        };
        
        state_manager.apply_transaction(&tx1).unwrap();
        assert!(state_manager.get_asset(&asset_id_1).is_some());
        
        // Create second asset
        let asset_id_2 = crate::types::sha256(b"asset2");
        let tx2 = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id: asset_id_2,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Light,
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("name".to_string(), "Asset 2".to_string());
                    m.insert("rarity".to_string(), "epic".to_string());
                    m
                },
                attributes: vec![crate::types::Attribute {
                    name: "defense".to_string(),
                    value: "5".to_string(),
                    rarity: None,
                }],
                game_id: Some("game1".to_string()),
                owner,
            },
            signature: vec![2; 64],
        };
        
        state_manager.apply_transaction(&tx2).unwrap();
        assert!(state_manager.get_asset(&asset_id_2).is_some());
        
        // Merge assets
        let mut merge_metadata = std::collections::HashMap::new();
        merge_metadata.insert("_other_asset_id".to_string(), hex::encode(asset_id_2));
        
        let merge_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Merge,
            asset_id: asset_id_1,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: merge_metadata,
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![3; 64],
        };
        
        state_manager.apply_transaction(&merge_tx).unwrap();
        
        // Check that merged asset exists and has combined data
        let merged_asset = state_manager.get_asset(&asset_id_1).unwrap();
        assert_eq!(merged_asset.owner, owner);
        assert!(merged_asset.data.metadata.contains_key("name"));
        assert!(merged_asset.data.metadata.contains_key("type"));
        assert!(merged_asset.data.metadata.contains_key("rarity"));
        // Density should be increased to Light (from asset 2)
        assert_eq!(merged_asset.data.density, crate::types::DensityLevel::Light);
        // Should have both attributes
        assert_eq!(merged_asset.data.attributes.len(), 2);
        
        // Check that other asset is removed
        assert!(state_manager.get_asset(&asset_id_2).is_none());
    }

    #[test]
    fn test_merge_assets_different_owners() {
        let config = create_test_config("merge_different_owners");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner1 = create_test_address(1);
        let owner2 = create_test_address(2);
        // Create accounts with balance for gas fees
        state_manager.create_test_account(owner1, 100_000, 0);
        state_manager.create_test_account(owner2, 100_000, 0);
        
        // Create first asset
        let asset_id_1 = crate::types::sha256(b"asset1");
        let tx1 = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id: asset_id_1,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner: owner1,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx1).unwrap();
        
        // Create second asset with different owner
        let asset_id_2 = crate::types::sha256(b"asset2");
        let tx2 = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id: asset_id_2,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner: owner2,
            },
            signature: vec![2; 64],
        };
        
        state_manager.apply_transaction(&tx2).unwrap();
        
        // Try to merge - should fail
        let mut merge_metadata = std::collections::HashMap::new();
        merge_metadata.insert("_other_asset_id".to_string(), hex::encode(asset_id_2));
        
        let merge_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Merge,
            asset_id: asset_id_1,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: merge_metadata,
                attributes: vec![],
                game_id: None,
                owner: owner1,
            },
            signature: vec![3; 64],
        };
        
        let result = state_manager.apply_transaction(&merge_tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different owners"));
        
        // Both assets should still exist
        assert!(state_manager.get_asset(&asset_id_1).is_some());
        assert!(state_manager.get_asset(&asset_id_2).is_some());
    }

    #[test]
    fn test_split_asset() {
        let config = create_test_config("split");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        // Create asset with multiple components
        let asset_id = crate::types::sha256(b"composite_asset");
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("component1".to_string(), "sword_data".to_string());
        metadata.insert("component2".to_string(), "shield_data".to_string());
        metadata.insert("component3".to_string(), "armor_data".to_string());
        
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Dense,
                metadata: metadata.clone(),
                attributes: vec![
                    crate::types::Attribute {
                        name: "power".to_string(),
                        value: "100".to_string(),
                        rarity: None,
                    },
                ],
                game_id: Some("game1".to_string()),
                owner,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx).unwrap();
        assert!(state_manager.get_asset(&asset_id).is_some());
        
        // Split asset into components
        let mut split_metadata = std::collections::HashMap::new();
        split_metadata.insert("_components".to_string(), "component1,component2,component3".to_string());
        
        let split_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Split,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: split_metadata,
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![2; 64],
        };
        
        state_manager.apply_transaction(&split_tx).unwrap();
        
        // Check that source asset is removed
        assert!(state_manager.get_asset(&asset_id).is_none());
        
        // Check that component assets were created
        let component1_id = crate::types::sha256(&[asset_id.as_ref(), b"component1"].concat());
        let component2_id = crate::types::sha256(&[asset_id.as_ref(), b"component2"].concat());
        let component3_id = crate::types::sha256(&[asset_id.as_ref(), b"component3"].concat());
        
        let comp1 = state_manager.get_asset(&component1_id).unwrap();
        assert_eq!(comp1.owner, owner);
        assert_eq!(comp1.data.metadata.get("component1"), Some(&"sword_data".to_string()));
        assert_eq!(comp1.data.density, crate::types::DensityLevel::Ethereal);
        
        let comp2 = state_manager.get_asset(&component2_id).unwrap();
        assert_eq!(comp2.owner, owner);
        assert_eq!(comp2.data.metadata.get("component2"), Some(&"shield_data".to_string()));
        
        let comp3 = state_manager.get_asset(&component3_id).unwrap();
        assert_eq!(comp3.owner, owner);
        assert_eq!(comp3.data.metadata.get("component3"), Some(&"armor_data".to_string()));
    }

    #[test]
    fn test_split_asset_invalid_owner() {
        let config = create_test_config("split_invalid_owner");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner1 = create_test_address(1);
        let owner2 = create_test_address(2);
        // Create accounts with balance for gas fees
        state_manager.create_test_account(owner1, 100_000, 0);
        state_manager.create_test_account(owner2, 100_000, 0);
        
        // Create asset
        let asset_id = crate::types::sha256(b"asset");
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("component1".to_string(), "data".to_string());
                    m
                },
                attributes: vec![],
                game_id: None,
                owner: owner1,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx).unwrap();
        
        // Try to split with wrong owner - should fail
        let mut split_metadata = std::collections::HashMap::new();
        split_metadata.insert("_components".to_string(), "component1".to_string());
        
        let split_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Split,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: split_metadata,
                attributes: vec![],
                game_id: None,
                owner: owner2, // Wrong owner
            },
            signature: vec![2; 64],
        };
        
        let result = state_manager.apply_transaction(&split_tx);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not owner") || err_msg.contains("Access denied"),
            "expected permission error, got: {}",
            err_msg
        );
        
        // Asset should still exist
        assert!(state_manager.get_asset(&asset_id).is_some());
    }

    #[test]
    fn test_asset_history() {
        let config = create_test_config("history");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        let asset_id = crate::types::sha256(b"test_asset");
        
        // Create asset
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx).unwrap();
        
        // Check history
        let history = state_manager.get_asset_history(&asset_id, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert!(matches!(history[0].action, crate::types::AssetAction::Create));
    }

    #[test]
    fn test_search_assets_by_owner() {
        let config = create_test_config("search_owner");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner1 = create_test_address(1);
        let owner2 = create_test_address(2);
        // Create accounts with balance for gas fees
        state_manager.create_test_account(owner1, 100_000, 0);
        state_manager.create_test_account(owner2, 100_000, 0);
        
        // Create assets for owner1
        for i in 0..3 {
            let asset_id = crate::types::sha256(&format!("asset1_{}", i).into_bytes());
            let tx = Transaction::MistbornAsset {
                action: crate::types::AssetAction::Create,
                asset_id,
                data: crate::types::AssetData {
                    density: crate::types::DensityLevel::Ethereal,
                    metadata: std::collections::HashMap::new(),
                    attributes: vec![],
                    game_id: None,
                    owner: owner1,
                },
                signature: vec![1; 64],
            };
            state_manager.apply_transaction(&tx).unwrap();
        }
        
        // Create asset for owner2
        let asset_id2 = crate::types::sha256(b"asset2");
        let tx2 = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id: asset_id2,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner: owner2,
            },
            signature: vec![2; 64],
        };
        state_manager.apply_transaction(&tx2).unwrap();
        
        // Search by owner1
        let results = state_manager.search_assets_by_owner(&owner1);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_assets_by_game_id() {
        let config = create_test_config("search_game");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        // Create assets with game_id
        for i in 0..2 {
            let asset_id = crate::types::sha256(&format!("game_asset_{}", i).into_bytes());
            let tx = Transaction::MistbornAsset {
                action: crate::types::AssetAction::Create,
                asset_id,
                data: crate::types::AssetData {
                    density: crate::types::DensityLevel::Ethereal,
                    metadata: std::collections::HashMap::new(),
                    attributes: vec![],
                    game_id: Some("game1".to_string()),
                    owner,
                },
                signature: vec![1; 64],
            };
            state_manager.apply_transaction(&tx).unwrap();
        }
        
        // Search by game_id
        let results = state_manager.search_assets_by_game_id("game1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_asset_versions() {
        let config = create_test_config("versions");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        let asset_id = crate::types::sha256(b"test_asset");
        
        // Create asset
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx).unwrap();
        
        // Check initial version (after create, version 1 should be created)
        let asset_state = state_manager.get_asset(&asset_id).unwrap();
        assert_eq!(asset_state.current_version, 1);
        
        let versions = state_manager.get_asset_versions(&asset_id).unwrap();
        // Should have 1 version (version 1 from initial snapshot)
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, 1);
        
        // Create manual snapshot
        let version = state_manager.create_asset_snapshot(&asset_id).unwrap();
        assert_eq!(version, 2);
        
        // Check versions (should have 2 now)
        let versions = state_manager.get_asset_versions(&asset_id).unwrap();
        assert_eq!(versions.len(), 2);
        assert!(versions.iter().any(|v| v.version == 1));
        assert!(versions.iter().any(|v| v.version == 2));
        
        // Get specific version
        let v1 = state_manager.get_asset_version(&asset_id, 1).unwrap();
        assert_eq!(v1.version, 1);
        
        let v2 = state_manager.get_asset_version(&asset_id, 2).unwrap();
        assert_eq!(v2.version, 2);
        
        // Get current version (version 0 means current)
        let v_current = state_manager.get_asset_version(&asset_id, 0).unwrap();
        assert_eq!(v_current.version, 2);
    }

    #[test]
    fn test_asset_versions_on_condense() {
        let config = create_test_config("versions_condense");
        let state_manager = StateManager::new(&config).unwrap();
        
        let owner = create_test_address(1);
        // Create account with balance for gas fees
        state_manager.create_test_account(owner, 100_000, 0);
        
        let asset_id = crate::types::sha256(b"test_asset");
        
        // Create asset
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        
        state_manager.apply_transaction(&tx).unwrap();
        
        // Condense (should create snapshot)
        let mut condense_metadata = std::collections::HashMap::new();
        condense_metadata.insert("new_data".to_string(), "value".to_string());
        
        let condense_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Condense,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Light,
                metadata: condense_metadata,
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![2; 64],
        };
        
        state_manager.apply_transaction(&condense_tx).unwrap();
        
        // Should have 2 versions (initial + condense)
        let asset_state = state_manager.get_asset(&asset_id).unwrap();
        assert_eq!(asset_state.current_version, 2); // After condense, version should be 2
        
        let versions = state_manager.get_asset_versions(&asset_id).unwrap();
        assert_eq!(versions.len(), 2);
        // Versions should be sorted, so version 1 first, then version 2
        let sorted_versions: Vec<u64> = versions.iter().map(|v| v.version).collect();
        assert_eq!(sorted_versions, vec![1, 2]);
    }

    #[test]
    fn test_create_asset() {
        let config = create_test_config("create_only");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        state_manager.create_test_account(owner, 100_000, 0);

        let asset_id = crate::types::sha256(b"create_test");
        let mut meta = std::collections::HashMap::new();
        meta.insert("name".to_string(), "Test Asset".to_string());

        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: meta,
                attributes: vec![],
                game_id: Some("g1".to_string()),
                owner,
            },
            signature: vec![1; 64],
        };
        state_manager.apply_transaction(&tx).unwrap();

        let asset = state_manager.get_asset(&asset_id).unwrap();
        assert_eq!(asset.owner, owner);
        assert_eq!(asset.data.density, crate::types::DensityLevel::Ethereal);
        assert_eq!(asset.data.metadata.get("name"), Some(&"Test Asset".to_string()));
        assert_eq!(asset.data.game_id, Some("g1".to_string()));
    }

    #[test]
    fn test_evaporate_asset() {
        let config = create_test_config("evaporate");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        state_manager.create_test_account(owner, 100_000, 0);

        let asset_id = crate::types::sha256(b"evap_asset");
        let create_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Light,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        state_manager.apply_transaction(&create_tx).unwrap();
        assert_eq!(state_manager.get_asset(&asset_id).unwrap().data.density, crate::types::DensityLevel::Light);

        let evap_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Evaporate,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![2; 64],
        };
        state_manager.apply_transaction(&evap_tx).unwrap();

        let asset = state_manager.get_asset(&asset_id).unwrap();
        assert_eq!(asset.data.density, crate::types::DensityLevel::Ethereal);
    }

    #[test]
    fn test_metadata_size_exceeded() {
        let config = create_test_config("meta_size");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        state_manager.create_test_account(owner, 100_000, 0);

        let asset_id = crate::types::sha256(b"oversized");
        let mut meta = std::collections::HashMap::new();
        meta.insert("big".to_string(), "x".to_string().repeat(6 * 1024)); // Ethereal max 5KB

        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: meta,
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        let res = state_manager.apply_transaction(&tx);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("Asset size exceeded") || err.contains("size"));
    }

    #[test]
    fn test_set_asset_permissions() {
        let config = create_test_config("perms");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        let other = create_test_address(2);
        state_manager.create_test_account(owner, 100_000, 0);

        let asset_id = crate::types::sha256(b"perm_asset");
        let create_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        state_manager.apply_transaction(&create_tx).unwrap();

        let set_tx = Transaction::SetAssetPermissions {
            asset_id,
            permissions: vec![crate::types::AssetPermission {
                grantee: other,
                level: crate::types::PermissionLevel::PublicRead,
                game_id: None,
                expires_at: None,
            }],
            public_read: true,
            owner,
            signature: vec![2; 64],
        };
        state_manager.apply_transaction(&set_tx).unwrap();

        let asset = state_manager.get_asset(&asset_id).unwrap();
        assert!(asset.public_read);
        assert_eq!(asset.permissions.len(), 1);
        assert_eq!(asset.permissions[0].grantee, other);
        assert_eq!(asset.permissions[0].level, crate::types::PermissionLevel::PublicRead);
    }

    #[test]
    fn test_search_assets_by_density() {
        let config = create_test_config("search_density");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        state_manager.create_test_account(owner, 100_000, 0);

        let id_e = crate::types::sha256(b"e");
        let id_l = crate::types::sha256(b"l");
        for (id, density) in [
            (id_e, crate::types::DensityLevel::Ethereal),
            (id_l, crate::types::DensityLevel::Light),
        ] {
            let tx = Transaction::MistbornAsset {
                action: crate::types::AssetAction::Create,
                asset_id: id,
                data: crate::types::AssetData {
                    density,
                    metadata: std::collections::HashMap::new(),
                    attributes: vec![],
                    game_id: None,
                    owner,
                },
                signature: vec![id[0]; 64],
            };
            state_manager.apply_transaction(&tx).unwrap();
        }

        let ethereal = state_manager.search_assets_by_density(crate::types::DensityLevel::Ethereal);
        let light = state_manager.search_assets_by_density(crate::types::DensityLevel::Light);
        assert!(ethereal.contains(&id_e));
        assert!(light.contains(&id_l));
        assert!(!ethereal.contains(&id_l));
        assert!(!light.contains(&id_e));
    }

    #[test]
    fn test_write_permission_game_contract() {
        let config = create_test_config("game_contract");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        let grantee = create_test_address(2);
        state_manager.create_test_account(owner, 100_000, 0);
        state_manager.create_test_account(grantee, 100_000, 0);

        let asset_id = crate::types::sha256(b"game_asset");
        let create_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: Some("game1".to_string()),
                owner,
            },
            signature: vec![1; 64],
        };
        state_manager.apply_transaction(&create_tx).unwrap();

        let set_tx = Transaction::SetAssetPermissions {
            asset_id,
            permissions: vec![crate::types::AssetPermission {
                grantee,
                level: crate::types::PermissionLevel::GameContract,
                game_id: Some("game1".to_string()),
                expires_at: None,
            }],
            public_read: false,
            owner,
            signature: vec![2; 64],
        };
        state_manager.apply_transaction(&set_tx).unwrap();

        let mut upd_meta = std::collections::HashMap::new();
        upd_meta.insert("updated".to_string(), "by_grantee".to_string());
        let update_tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Update,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: upd_meta,
                attributes: vec![],
                game_id: Some("game1".to_string()),
                owner: grantee,
            },
            signature: vec![3; 64],
        };
        state_manager.apply_transaction(&update_tx).unwrap();

        let asset = state_manager.get_asset(&asset_id).unwrap();
        assert_eq!(asset.data.metadata.get("updated"), Some(&"by_grantee".to_string()));
    }

    #[test]
    fn test_get_quota_usage() {
        let config = create_test_config("quota");
        let state_manager = StateManager::new(&config).unwrap();
        let owner = create_test_address(1);
        state_manager.create_test_account(owner, 100_000, 0);

        let usage_empty = state_manager.get_quota_usage(&owner);
        assert_eq!(usage_empty.assets_count, 0);
        assert!(usage_empty.assets_limit > 0);
        assert_eq!(usage_empty.blob_files_count, 0);

        let asset_id = crate::types::sha256(b"quota_asset");
        let tx = Transaction::MistbornAsset {
            action: crate::types::AssetAction::Create,
            asset_id,
            data: crate::types::AssetData {
                density: crate::types::DensityLevel::Ethereal,
                metadata: std::collections::HashMap::new(),
                attributes: vec![],
                game_id: None,
                owner,
            },
            signature: vec![1; 64],
        };
        state_manager.apply_transaction(&tx).unwrap();

        let usage = state_manager.get_quota_usage(&owner);
        assert_eq!(usage.assets_count, 1);
        assert!(usage.assets_limit > 0);
        assert!(usage.metadata_size_limit > 0);
    }
}