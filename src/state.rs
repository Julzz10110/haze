//! State management for HAZE blockchain

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use sled::Db;
use tokio::sync::broadcast;
use crate::types::{Address, Hash, Block, Transaction, AssetAction};
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
    accounts: Arc<DashMap<Address, AccountState>>,
    assets: Arc<DashMap<Hash, AssetState>>,
    blocks: Arc<DashMap<Hash, Block>>,
    current_height: Arc<RwLock<u64>>,
    tokenomics: Arc<Tokenomics>,
    economy: Arc<FogEconomy>,
    ws_tx: Arc<RwLock<Option<broadcast::Sender<WsEvent>>>>,
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
            accounts: Arc::new(DashMap::new()),
            assets: Arc::new(DashMap::new()),
            blocks: Arc::new(DashMap::new()),
            current_height: Arc::new(RwLock::new(0)),
            tokenomics: Arc::new(Tokenomics::new()),
            economy: Arc::new(FogEconomy::new()),
            ws_tx: Arc::new(RwLock::new(None)),
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
    pub fn get_asset(&self, asset_id: &Hash) -> Option<AssetState> {
        self.assets.get(asset_id).map(|v| v.clone())
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
                match action {
                    crate::types::AssetAction::Create => {
                        // Check if asset already exists
                        if self.assets.contains_key(asset_id) {
                            return Err(HazeError::InvalidTransaction(
                                "Asset already exists".to_string()
                            ));
                        }
                        
                        // Validate metadata size
                        let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
                        if metadata_size > data.density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                metadata_size,
                                data.density.max_size()
                            ));
                        }
                        
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
                        };
                        
                        // Remove special metadata keys before storing
                        asset_state.data.metadata.remove("_blob_refs");
                        
                        // Add creation to history
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Create, HashMap::new());
                        
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
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::AccessDenied(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
                        // Save owner before moving asset_state
                        let owner = asset_state.owner;
                        
                        // Validate new metadata size
                        let current_size: usize = asset_state.data.metadata.values().map(|v| v.len()).sum();
                        let new_metadata_size: usize = data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(_, v)| v.len())
                            .sum();
                        
                        if current_size + new_metadata_size > asset_state.data.density.max_size() {
                            return Err(HazeError::AssetSizeExceeded(
                                current_size + new_metadata_size,
                                asset_state.data.density.max_size()
                            ));
                        }
                        
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
                        
                        // Record changes in history
                        let changes: HashMap<String, String> = data.metadata.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Self::add_asset_history(&mut asset_state, crate::types::AssetAction::Update, changes);
                        
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
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::AccessDenied(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
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
                        
                        // Update density and merge new data
                        let new_density_str = format!("{:?}", data.density);
                        let old_density = asset_state.data.density;
                        asset_state.data.density = data.density;
                        
                        // Process blob_refs from metadata (special key "_blob_refs" as JSON)
                        if let Some(blob_refs_json) = data.metadata.get("_blob_refs") {
                            if let Ok(blob_refs_map) = serde_json::from_str::<HashMap<String, String>>(blob_refs_json) {
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
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::AccessDenied(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
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
                        
                        // Verify both assets have the same owner
                        if asset_state.owner != other_asset_state.owner || asset_state.owner != data.owner {
                            return Err(HazeError::AccessDenied(
                                "Cannot merge assets with different owners".to_string()
                            ));
                        }
                        
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
                        
                        // Merge attributes
                        asset_state.data.attributes.extend(other_asset_state.data.attributes.clone());
                        
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
                        
                        // Update source asset
                        self.assets.insert(*asset_id, asset_state.clone());
                        
                        // Remove merged asset
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
                        
                        // Verify ownership
                        if source_asset_state.owner != data.owner {
                            return Err(HazeError::AccessDenied(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
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
                            };
                            
                            // Add creation to history
                            let mut changes = HashMap::new();
                            changes.insert("source_asset_id".to_string(), hex::encode(asset_id));
                            changes.insert("component_name".to_string(), component_name.clone());
                            Self::add_asset_history(&mut component_asset_state, crate::types::AssetAction::Split, changes);
                            
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
                        
                        // Remove or update source asset (for now, we remove it)
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
            _ => {
                // Contract calls handled by VM
            }
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

    #[cfg(test)]
    /// Create test account (for testing only)
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
            accounts: self.accounts.clone(),
            assets: self.assets.clone(),
            blocks: self.blocks.clone(),
            current_height: self.current_height.clone(),
            tokenomics: self.tokenomics.clone(),
            economy: self.economy.clone(),
            ws_tx: self.ws_tx.clone(),
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
        assert!(result.unwrap_err().to_string().contains("ownership mismatch"));
        
        // Asset should still exist
        assert!(state_manager.get_asset(&asset_id).is_some());
    }
}