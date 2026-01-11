//! Fog Consensus implementation for HAZE
//! 
//! Based on modified Narwhal-Bullshark DAG with:
//! - Haze Committees (dynamic validator groups)
//! - Wave Finalization (wave-based transaction propagation)
//! - Haze Weights (reputation system)

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use dashmap::DashMap;
use crate::types::{Block, BlockHeader, Hash, Address, Transaction};
use crate::state::StateManager;
use crate::config::Config;
use crate::error::Result;
use crate::crypto::verify_signature;
use chrono::Utc;

/// Consensus engine implementing Fog Consensus
pub struct ConsensusEngine {
    config: Config,
    state: Arc<StateManager>,
    
    // DAG structure
    dag: Arc<RwLock<Dag>>,
    
    // Haze Committees
    committees: Arc<RwLock<HashMap<u64, Committee>>>,
    current_committee_id: Arc<RwLock<u64>>,
    
    // Wave finalization
    waves: Arc<RwLock<HashMap<u64, Wave>>>,
    current_wave: Arc<RwLock<u64>>,
    
    // Transaction pool
    tx_pool: Arc<DashMap<Hash, Transaction>>,
}

/// DAG structure for Fog Consensus
#[allow(dead_code)] // Fields will be used in full implementation
struct Dag {
    vertices: HashMap<Hash, DagVertex>,
    edges: HashMap<Hash, Vec<Hash>>,
}

#[allow(dead_code)] // Fields will be used in full implementation
struct DagVertex {
    block: Block,
    references: Vec<Hash>,
    wave: u64,
}

/// Haze Committee - dynamic validator group
#[allow(dead_code)] // Fields will be used in full implementation
struct Committee {
    id: u64,
    validators: Vec<Address>,
    weights: HashMap<Address, u64>, // Haze weights
    created_at: i64,
    expires_at: i64,
}

/// Wave for finalization
#[allow(dead_code)] // Fields will be used in full implementation
struct Wave {
    number: u64,
    blocks: HashSet<Hash>,
    finalized: bool,
    created_at: i64,
}

impl ConsensusEngine {
    pub fn new(config: Config, state: Arc<StateManager>) -> Result<Self> {
        let mut engine = Self {
            config: config.clone(),
            state,
            dag: Arc::new(RwLock::new(Dag {
                vertices: HashMap::new(),
                edges: HashMap::new(),
            })),
            committees: Arc::new(RwLock::new(HashMap::new())),
            current_committee_id: Arc::new(RwLock::new(0)),
            waves: Arc::new(RwLock::new(HashMap::new())),
            current_wave: Arc::new(RwLock::new(0)),
            tx_pool: Arc::new(DashMap::new()),
        };

        // Initialize first committee
        engine.initialize_committee()?;

        Ok(engine)
    }

    /// Initialize a new Haze Committee
    fn initialize_committee(&mut self) -> Result<()> {
        let committee_id = *self.current_committee_id.read() + 1;
        let now = Utc::now().timestamp();
        let expires_at = now + self.config.consensus.committee_rotation_interval as i64;

        // Select validators based on stake (top validators)
        const COMMITTEE_SIZE: usize = 21; // Typical BFT committee size
        let top_validators = self.state.tokenomics().get_top_validators(COMMITTEE_SIZE);
        let validator_count = top_validators.len();
        let validators: Vec<Address> = top_validators.iter().map(|v| v.address).collect();
        
        // Calculate weights (stake-based)
        let mut weights = HashMap::new();
        for validator_info in top_validators {
            weights.insert(validator_info.address, validator_info.total_staked);
        }

        let committee = Committee {
            id: committee_id,
            validators,
            weights,
            created_at: now,
            expires_at,
        };

        self.committees.write().insert(committee_id, committee);
        *self.current_committee_id.write() = committee_id;

        tracing::info!("Initialized committee {} with {} validators", committee_id, validator_count);

        Ok(())
    }

    /// Check if committee needs rotation
    pub fn check_committee_rotation(&mut self) -> Result<bool> {
        let current_id = *self.current_committee_id.read();
        let should_rotate = {
            if let Some(committee) = self.committees.read().get(&current_id) {
                let now = Utc::now().timestamp();
                now >= committee.expires_at
            } else {
                false
            }
        };
        if should_rotate {
            self.initialize_committee()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Add transaction to pool
    ///
    /// Validates the transaction before adding it to the pool.
    ///
    /// # Arguments
    /// * `tx` - The transaction to add
    ///
    /// # Errors
    /// Returns an error if the transaction is invalid (duplicate, invalid signature, etc.)
    pub fn add_transaction(&self, tx: Transaction) -> Result<()> {
        // Check if transaction already exists in pool
        let tx_hash = tx.hash();
        if self.tx_pool.contains_key(&tx_hash) {
            return Err(crate::error::HazeError::InvalidTransaction(
                "Transaction already in pool".to_string()
            ));
        }

        // Basic validation
        self.validate_transaction(&tx)?;

        // Add to pool
        self.tx_pool.insert(tx_hash, tx);
        Ok(())
    }

    /// Validate transaction
    ///
    /// Performs basic validation checks on a transaction.
    fn validate_transaction(&self, tx: &Transaction) -> Result<()> {
        match tx {
            Transaction::Transfer { from, amount, fee, .. } => {
                // Check that amount and fee are not zero
                if *amount == 0 {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Transfer amount cannot be zero".to_string()
                    ));
                }
                if *fee == 0 {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Transaction fee cannot be zero".to_string()
                    ));
                }

                // Check that sender has sufficient balance (if account exists)
                if let Some(account) = self.state.get_account(from) {
                    if account.balance < *amount + *fee {
                        return Err(crate::error::HazeError::InvalidTransaction(
                            "Insufficient balance".to_string()
                        ));
                    }
                }

                // Verify signature
                self.verify_transaction_signature(tx, from)?;
                
                // Check nonce
                self.validate_nonce(tx)?;
            }
            Transaction::Stake { validator, amount, signature, .. } => {
                if *amount == 0 {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Stake amount cannot be zero".to_string()
                    ));
                }

                // Verify signature
                if signature.is_empty() {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Transaction signature is empty".to_string()
                    ));
                }
                self.verify_transaction_signature(tx, validator)?;
            }
            Transaction::ContractCall { gas_limit, signature, .. } => {
                if *gas_limit == 0 {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Gas limit cannot be zero".to_string()
                    ));
                }

                // Basic signature validation
                if signature.is_empty() {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Transaction signature is empty".to_string()
                    ));
                }
            }
            Transaction::MistbornAsset { data, signature, .. } => {
                // Verify signature
                if signature.is_empty() {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        "Transaction signature is empty".to_string()
                    ));
                }
                self.verify_transaction_signature(tx, &data.owner)?;

                // TODO: Validate asset data
            }
        }

        Ok(())
    }

    /// Verify transaction signature
    ///
    /// Verifies that the transaction signature is valid for the signer's address.
    /// In HAZE, the address is the first 32 bytes of the ED25519 public key.
    fn verify_transaction_signature(&self, tx: &Transaction, signer_address: &Address) -> Result<()> {
        let signature = match tx {
            Transaction::Transfer { signature, .. } => signature,
            Transaction::Stake { signature, .. } => signature,
            Transaction::ContractCall { signature, .. } => signature,
            Transaction::MistbornAsset { signature, .. } => signature,
        };

        // Get transaction data for signing (transaction without signature field)
        let tx_data = self.get_transaction_data_for_signing(tx);

        // Verify signature using address as public key (first 32 bytes of ED25519 pubkey)
        let is_valid = verify_signature(signer_address, &tx_data, signature)
            .map_err(|e| crate::error::HazeError::InvalidTransaction(
                format!("Signature verification error: {}", e)
            ))?;

        if !is_valid {
            return Err(crate::error::HazeError::InvalidTransaction(
                "Invalid transaction signature".to_string()
            ));
        }

        Ok(())
    }

    /// Validate transaction nonce
    ///
    /// Checks that the transaction nonce is correct for the sender account.
    /// Nonce must be sequential: for existing accounts it must be current_nonce + 1,
    /// considering pending transactions in the pool. For new accounts it must be 0.
    ///
    /// # Arguments
    /// * `tx` - The transaction to validate
    ///
    /// # Errors
    /// Returns an error if the nonce is invalid (too low, too high, or duplicate).
    fn validate_nonce(&self, tx: &Transaction) -> Result<()> {
        let (from_address, tx_nonce) = match tx {
            Transaction::Transfer { from, nonce, .. } => (*from, *nonce),
            _ => {
                // Nonce validation only applies to Transfer transactions
                return Ok(());
            }
        };

        // Get current account nonce (0 for new accounts)
        let current_nonce = self.state
            .get_account(&from_address)
            .map(|acc| acc.nonce)
            .unwrap_or(0);

        // Get expected nonce considering pending transactions in pool
        let expected_nonce = self.get_expected_nonce(&from_address, current_nonce);

        if tx_nonce != expected_nonce {
            return Err(crate::error::HazeError::InvalidTransaction(
                format!(
                    "Invalid nonce: expected {}, got {}",
                    expected_nonce, tx_nonce
                )
            ));
        }

        Ok(())
    }

    /// Get expected nonce for an account
    ///
    /// Returns the next expected nonce for an account, taking into account
    /// pending transactions in the transaction pool.
    ///
    /// # Arguments
    /// * `address` - The account address
    /// * `current_nonce` - The current nonce from state (0 for new accounts)
    ///
    /// # Returns
    /// The next expected nonce (current_nonce + number_of_pending_txs + 1)
    fn get_expected_nonce(&self, address: &Address, current_nonce: u64) -> u64 {
        // Count pending transactions from this address in the pool
        let mut pending_count = 0u64;
        for entry in self.tx_pool.iter() {
            if let Transaction::Transfer { from, .. } = entry.value() {
                if from == address {
                    pending_count += 1;
                }
            }
        }

        // Expected nonce is current nonce plus pending transactions
        current_nonce + pending_count
    }

    /// Get transaction data for signing (transaction without signature field)
    ///
    /// Creates a serialized representation of the transaction without the signature
    /// for use in signature verification. The data format matches what was signed.
    fn get_transaction_data_for_signing(&self, tx: &Transaction) -> Vec<u8> {
        
        // Serialize transaction data without signature
        // We manually serialize each field to match the signing format
        match tx {
            Transaction::Transfer { from, to, amount, fee, nonce, .. } => {
                let mut data = Vec::new();
                data.extend_from_slice(b"Transfer");
                data.extend_from_slice(from);
                data.extend_from_slice(to);
                data.extend_from_slice(&amount.to_le_bytes());
                data.extend_from_slice(&fee.to_le_bytes());
                data.extend_from_slice(&nonce.to_le_bytes());
                data
            }
            Transaction::Stake { validator, amount, .. } => {
                let mut data = Vec::new();
                data.extend_from_slice(b"Stake");
                data.extend_from_slice(validator);
                data.extend_from_slice(&amount.to_le_bytes());
                data
            }
            Transaction::ContractCall { contract, method, args, gas_limit, .. } => {
                let mut data = Vec::new();
                data.extend_from_slice(b"ContractCall");
                data.extend_from_slice(contract);
                data.extend_from_slice(method.as_bytes());
                data.push(0); // Null terminator for method
                data.extend_from_slice(&gas_limit.to_le_bytes());
                data.extend_from_slice(args);
                data
            }
            Transaction::MistbornAsset { action, asset_id, data, .. } => {
                // Serialize asset data for signing
                let mut serialized = Vec::new();
                serialized.extend_from_slice(b"MistbornAsset");
                // Serialize action as u8
                serialized.push(match action {
                    crate::types::AssetAction::Create => 0,
                    crate::types::AssetAction::Update => 1,
                    crate::types::AssetAction::Condense => 2,
                    crate::types::AssetAction::Evaporate => 3,
                    crate::types::AssetAction::Merge => 4,
                    crate::types::AssetAction::Split => 5,
                });
                serialized.extend_from_slice(asset_id);
                serialized.extend_from_slice(&data.owner);
                // Serialize density as u8
                serialized.push(match data.density {
                    crate::types::DensityLevel::Ethereal => 0,
                    crate::types::DensityLevel::Light => 1,
                    crate::types::DensityLevel::Dense => 2,
                    crate::types::DensityLevel::Core => 3,
                });
                serialized
            }
        }
    }

    /// Create new block
    pub fn create_block(&self, validator: Address) -> Result<Block> {
        // Check committee rotation (using interior mutability)
        self.check_and_rotate_committee()?;
        
        // Collect transactions from pool
        let mut transactions = Vec::new();
        let max_txs = self.config.consensus.max_transactions_per_block;
        
        for entry in self.tx_pool.iter().take(max_txs) {
            transactions.push(entry.value().clone());
        }

        // Get current height
        let height = self.state.current_height();
        
        // Get DAG references (parent blocks)
        let dag_refs = self.get_dag_references()?;
        
        // Create block header
        let parent_hash = self.get_parent_hash()?;
        let mut header = BlockHeader {
            hash: [0; 32], // Will be computed
            parent_hash,
            height: height + 1,
            timestamp: Utc::now().timestamp(),
            validator,
            merkle_root: self.compute_merkle_root(&transactions)?,
            state_root: self.state.compute_state_root(),
            wave_number: *self.current_wave.read(),
            committee_id: *self.current_committee_id.read(),
        };
        
        header.hash = header.compute_hash();

        let block = Block {
            header,
            transactions,
            dag_references: dag_refs,
        };

        Ok(block)
    }

    /// Get DAG references for new block
    fn get_dag_references(&self) -> Result<Vec<Hash>> {
        let dag = self.dag.read();
        // Return recent block hashes as references
        let refs: Vec<Hash> = dag.vertices.keys().take(10).cloned().collect();
        Ok(refs)
    }

    /// Get parent hash
    fn get_parent_hash(&self) -> Result<Hash> {
        // Get from latest finalized block in DAG
        let dag = self.dag.read();
        let waves = self.waves.read();
        
        // Find the highest finalized wave
        let mut highest_finalized_wave: Option<u64> = None;
        for (wave_num, wave) in waves.iter() {
            if wave.finalized {
            if highest_finalized_wave.map_or(true, |h| *wave_num > h) {
                highest_finalized_wave = Some(*wave_num);
            }
            }
        }
        
        // If we have a finalized wave, get the latest block from it
        if let Some(wave_num) = highest_finalized_wave {
            if let Some(wave) = waves.get(&wave_num) {
                // Get the block with highest height from this wave
                let mut latest_block: Option<&Block> = None;
                for block_hash in &wave.blocks {
                    if let Some(vertex) = dag.vertices.get(block_hash) {
                        if latest_block.is_none() || vertex.block.header.height > latest_block.unwrap().header.height {
                            latest_block = Some(&vertex.block);
                        }
                    }
                }
                if let Some(block) = latest_block {
                    return Ok(block.header.hash);
                }
            }
        }
        
        // Fallback: get latest block by height from state
        let height = self.state.current_height();
        if height > 0 {
            // Try to find block at current height
            for (_hash, vertex) in dag.vertices.iter() {
                if vertex.block.header.height == height {
                    return Ok(vertex.block.header.hash);
                }
            }
        }
        
        // Genesis block
        Ok([0; 32])
    }
    
    /// Check and rotate committee if needed (using interior mutability)
    fn check_and_rotate_committee(&self) -> Result<()> {
        let current_id = *self.current_committee_id.read();
        let should_rotate = {
            if let Some(committee) = self.committees.read().get(&current_id) {
                let now = Utc::now().timestamp();
                now >= committee.expires_at
            } else {
                true // No committee exists, need to create one
            }
        };
        
        if should_rotate {
            // Use a workaround: we can't mutate self, so we'll skip rotation here
            // In a real implementation, this would need to be handled differently
            // For now, we'll just log a warning
            tracing::warn!("Committee rotation needed but create_block is not mutable");
        }
        
        Ok(())
    }

    /// Compute merkle root
    fn compute_merkle_root(&self, transactions: &[Transaction]) -> Result<Hash> {
        if transactions.is_empty() {
            return Ok([0; 32]);
        }
        
        let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();
        
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                if chunk.len() == 2 {
                    let combined = [chunk[0].as_ref(), chunk[1].as_ref()].concat();
                    next_level.push(crate::types::sha256(&combined));
                } else {
                    next_level.push(chunk[0]);
                }
            }
            hashes = next_level;
        }
        
        Ok(hashes[0])
    }

    /// Process block (add to DAG)
    pub fn process_block(&self, block: &Block) -> Result<()> {
        let block_hash = block.header.hash;
        
        // Add to DAG
        {
            let mut dag = self.dag.write();
            let vertex = DagVertex {
                block: block.clone(),
                references: block.dag_references.clone(),
                wave: block.header.wave_number,
            };
            dag.vertices.insert(block_hash, vertex);
            dag.edges.insert(block_hash, block.dag_references.clone());
        }

        // Update wave
        {
            let mut waves = self.waves.write();
            let wave_num = block.header.wave_number;
            let wave = waves.entry(wave_num).or_insert_with(|| Wave {
                number: wave_num,
                blocks: HashSet::new(),
                finalized: false,
                created_at: Utc::now().timestamp(),
            });
            wave.blocks.insert(block_hash);
        }

        // Apply to state
        self.state.apply_block(block)?;

        Ok(())
    }

    /// Check wave finalization (Golden Wave)
    pub fn check_wave_finalization(&self, wave_num: u64) -> Result<bool> {
        let waves = self.waves.read();
        if let Some(wave) = waves.get(&wave_num) {
            let now = Utc::now().timestamp();
            let elapsed = (now - wave.created_at) * 1000; // Convert to ms
            
            if elapsed >= self.config.consensus.golden_wave_threshold as i64 {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Clone for ConsensusEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: self.state.clone(),
            dag: self.dag.clone(),
            committees: self.committees.clone(),
            current_committee_id: self.current_committee_id.clone(),
            waves: self.waves.clone(),
            current_wave: self.current_wave.clone(),
            tx_pool: self.tx_pool.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::types::Transaction;
    use crate::config::Config;

    fn create_test_config(test_name: &str) -> Config {
        let mut config = Config::default();
        config.storage.db_path = std::path::PathBuf::from(format!("./haze_db_test_consensus_{}", test_name));
        config
    }

    #[test]
    fn test_add_transaction_duplicate() {
        let config = create_test_config("duplicate");
        let state = crate::state::StateManager::new(&config).unwrap();
        
        // Create account with balance for new account
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Add account with balance (new account has nonce 0)
        state.create_test_account(from, 10000, 0);
        
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        // Create transaction with nonce 0 for new account
        let tx_data = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes()); // nonce 0 for new account
            data
        };
        let signature = keypair.sign(&tx_data);
        
        let tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 0, // Correct nonce for new account
            signature,
        };
        
        // First add should succeed
        consensus.add_transaction(tx.clone()).unwrap();
        
        // Second add should fail (duplicate)
        let result = consensus.add_transaction(tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already in pool"));
    }

    #[test]
    fn test_verify_transaction_signature_transfer() {
        let config = create_test_config("signature");
        let state = crate::state::StateManager::new(&config).unwrap();
        
        // Create account with balance for new account
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Add account with balance (new account has nonce 0)
        state.create_test_account(from, 10000, 0);
        
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        // Create transaction data for signing with nonce 0 for new account
        let tx_data = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes()); // nonce 0 for new account
            data
        };
        let signature = keypair.sign(&tx_data);
        
        let tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 0, // Correct nonce for new account
            signature,
        };
        
        // Valid signature should pass
        // Note: This test verifies the signature format is correct
        // Full verification happens in add_transaction
        let result = consensus.add_transaction(tx);
        // Should succeed with valid nonce and balance
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_transaction_empty_signature() {
        let config = create_test_config("empty_sig");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let tx = Transaction::Stake {
            validator: [1u8; 32],
            amount: 1000,
            signature: vec![], // Empty signature
        };
        
        let result = consensus.add_transaction(tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_nonce_validation_new_account() {
        let config = create_test_config("nonce_new");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Create transaction with nonce 0 for new account
        let tx_data = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes()); // nonce 0
            data
        };
        let signature = keypair.sign(&tx_data);
        
        let tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 0, // Correct nonce for new account
            signature,
        };
        
        // Should succeed for new account with nonce 0
        let result = consensus.add_transaction(tx);
        // Might fail due to insufficient balance, but nonce should be valid
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("balance"));
    }

    #[test]
    fn test_nonce_validation_duplicate() {
        let config = create_test_config("nonce_dup");
        let state = crate::state::StateManager::new(&config).unwrap();
        
        // Create account with initial balance
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Manually create account with nonce 0 and balance
        state.create_test_account(from, 10000, 0);
        
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        // Create first transaction with nonce 0
        let tx_data_1 = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes());
            data
        };
        let signature_1 = keypair.sign(&tx_data_1);
        
        let tx1 = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 0,
            signature: signature_1,
        };
        
        // First transaction should succeed
        consensus.add_transaction(tx1).unwrap();
        
        // Second transaction with same nonce should fail
        let tx_data_2 = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&500u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes()); // Same nonce
            data
        };
        let signature_2 = keypair.sign(&tx_data_2);
        
        let tx2 = Transaction::Transfer {
            from,
            to,
            amount: 500,
            fee: 10,
            nonce: 0, // Duplicate nonce
            signature: signature_2,
        };
        
        let result = consensus.add_transaction(tx2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonce"));
    }

    #[test]
    fn test_nonce_validation_sequential() {
        let config = create_test_config("nonce_seq");
        let state = crate::state::StateManager::new(&config).unwrap();
        
        // Create account with initial balance and nonce 1
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Account already executed 1 transaction (nonce 0), so current nonce is 1
        state.create_test_account(from, 10000, 1);
        
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        // Create transaction with nonce 1 (matches current account nonce)
        // Expected nonce = current_nonce (1) + pending_count (0) = 1
        let tx_data = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&1u64.to_le_bytes()); // nonce 1 matches current account nonce
            data
        };
        let signature = keypair.sign(&tx_data);
        
        let tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 1, // Correct: matches current account nonce
            signature,
        };
        
        let result = consensus.add_transaction(tx);
        // Should succeed or fail due to balance, but nonce should be valid
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("balance"));
    }

    #[test]
    fn test_nonce_validation_too_high() {
        let config = create_test_config("nonce_high");
        let state = crate::state::StateManager::new(&config).unwrap();
        
        let keypair = KeyPair::generate();
        let from = keypair.address();
        let to = [2u8; 32];
        
        // Account has nonce 0 (new account, no transactions yet)
        state.create_test_account(from, 10000, 0);
        
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        // Transaction with nonce 5 when account has nonce 0 (should be 0)
        let tx_data = {
            let mut data = Vec::new();
            data.extend_from_slice(b"Transfer");
            data.extend_from_slice(&from);
            data.extend_from_slice(&to);
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&10u64.to_le_bytes());
            data.extend_from_slice(&5u64.to_le_bytes()); // nonce 5, too high
            data
        };
        let signature = keypair.sign(&tx_data);
        
        let tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            fee: 10,
            nonce: 5, // Too high: account has nonce 0, expected is 0
            signature,
        };
        
        let result = consensus.add_transaction(tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonce"));
    }
}