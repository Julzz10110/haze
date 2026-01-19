//! State management for HAZE blockchain

use std::sync::Arc;
use parking_lot::RwLock;
use sled::Db;
use crate::types::{Address, Hash, Block, Transaction};
use crate::config::Config;
use crate::error::{HazeError, Result};
use crate::tokenomics::Tokenomics;
use crate::economy::FogEconomy;
use dashmap::DashMap;

/// State manager for blockchain state
pub struct StateManager {
    db: Arc<Db>,
    accounts: Arc<DashMap<Address, AccountState>>,
    assets: Arc<DashMap<Hash, AssetState>>,
    blocks: Arc<DashMap<Hash, Block>>,
    current_height: Arc<RwLock<u64>>,
    tokenomics: Arc<Tokenomics>,
    economy: Arc<FogEconomy>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
    pub staked: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetState {
    pub owner: Address,
    pub data: crate::types::AssetData,
    pub created_at: i64,
    pub updated_at: i64,
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
        })
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
                        
                        let asset_state = AssetState {
                            owner: data.owner,
                            data: data.clone(),
                            created_at: chrono::Utc::now().timestamp(),
                            updated_at: chrono::Utc::now().timestamp(),
                        };
                        self.assets.insert(*asset_id, asset_state);
                    }
                    crate::types::AssetAction::Update => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::InvalidTransaction(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
                        // Update metadata and attributes
                        asset_state.data.metadata = data.metadata.clone();
                        asset_state.data.attributes = data.attributes.clone();
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        self.assets.insert(*asset_id, asset_state);
                    }
                    crate::types::AssetAction::Condense => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::InvalidTransaction(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
                        // Check if condensation is valid (can only increase density)
                        let current_density = asset_state.data.density as u8;
                        let new_density = data.density as u8;
                        if new_density <= current_density {
                            return Err(HazeError::InvalidTransaction(
                                "Condensation must increase density".to_string()
                            ));
                        }
                        
                        // Update density and merge new data
                        asset_state.data.density = data.density;
                        for (key, value) in &data.metadata {
                            asset_state.data.metadata.insert(key.clone(), value.clone());
                        }
                        asset_state.data.attributes.extend(data.attributes.clone());
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        self.assets.insert(*asset_id, asset_state);
                    }
                    crate::types::AssetAction::Evaporate => {
                        let mut asset_state = self.assets.get(asset_id)
                            .ok_or_else(|| HazeError::InvalidTransaction(
                                "Asset not found".to_string()
                            ))?
                            .clone();
                        
                        // Verify ownership
                        if asset_state.owner != data.owner {
                            return Err(HazeError::InvalidTransaction(
                                "Asset ownership mismatch".to_string()
                            ));
                        }
                        
                        // Check if evaporation is valid (can only decrease density)
                        let current_density = asset_state.data.density as u8;
                        let new_density = data.density as u8;
                        if new_density >= current_density {
                            return Err(HazeError::InvalidTransaction(
                                "Evaporation must decrease density".to_string()
                            ));
                        }
                        
                        // Update density
                        asset_state.data.density = data.density;
                        asset_state.updated_at = chrono::Utc::now().timestamp();
                        
                        self.assets.insert(*asset_id, asset_state);
                    }
                    crate::types::AssetAction::Merge => {
                        // Merge requires two assets - this is handled differently
                        // For now, we'll handle it as an update
                        return Err(HazeError::InvalidTransaction(
                            "Merge operation requires special handling with two asset IDs".to_string()
                        ));
                    }
                    crate::types::AssetAction::Split => {
                        // Split creates new assets - handled differently
                        return Err(HazeError::InvalidTransaction(
                            "Split operation creates multiple assets and requires special handling".to_string()
                        ));
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
}