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

#[derive(Debug, Clone)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
    pub staked: u64,
}

#[derive(Debug, Clone)]
pub struct AssetState {
    pub owner: Address,
    pub data: crate::types::AssetData,
    pub created_at: i64,
    pub updated_at: i64,
}

impl StateManager {
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

    /// Get account state
    pub fn get_account(&self, address: &Address) -> Option<AccountState> {
        self.accounts.get(address).map(|v| v.clone())
    }

    /// Get asset state
    pub fn get_asset(&self, asset_id: &Hash) -> Option<AssetState> {
        self.assets.get(asset_id).map(|v| v.clone())
    }

    /// Get block by hash
    pub fn get_block(&self, hash: &Hash) -> Option<Block> {
        self.blocks.get(hash).map(|v| v.clone())
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
            Transaction::Transfer { from, to, amount, fee, .. } => {
                let mut from_account = self.accounts
                    .entry(*from)
                    .or_insert_with(|| AccountState {
                        balance: 0,
                        nonce: 0,
                        staked: 0,
                    });
                
                if from_account.balance < *amount + *fee {
                    return Err(HazeError::InvalidTransaction("Insufficient balance".to_string()));
                }

                from_account.balance -= amount + fee;
                from_account.nonce += 1;

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
            Transaction::MistbornAsset { asset_id, data, .. } => {
                let asset_state = AssetState {
                    owner: data.owner,
                    data: data.clone(),
                    created_at: chrono::Utc::now().timestamp(),
                    updated_at: chrono::Utc::now().timestamp(),
                };
                self.assets.insert(*asset_id, asset_state);
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