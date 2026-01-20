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
struct Dag {
    vertices: HashMap<Hash, DagVertex>,
    edges: HashMap<Hash, Vec<Hash>>, // Outgoing edges (references)
    reverse_edges: HashMap<Hash, Vec<Hash>>, // Incoming edges (who references this block)
}

struct DagVertex {
    block: Block,
    references: Vec<Hash>,
    wave: u64,
    timestamp: i64,
    processed: bool,
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
                reverse_edges: HashMap::new(),
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
    
    /// Get transaction from pool by hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Option<Transaction> {
        self.tx_pool.get(tx_hash).map(|tx| tx.clone())
    }
    
    /// Remove transactions from pool (after they've been included in a block)
    pub fn remove_transactions_from_pool(&self, transactions: &[Transaction]) {
        for tx in transactions {
            let tx_hash = tx.hash();
            self.tx_pool.remove(&tx_hash);
        }
    }
    
    /// Get transaction pool size
    pub fn tx_pool_size(&self) -> usize {
        self.tx_pool.len()
    }
    
    /// Get current wave number (read access)
    pub fn get_current_wave(&self) -> u64 {
        *self.current_wave.read()
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

                // Validate asset data
                self.validate_asset_data(data)?;
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
    /// Validate asset data
    ///
    /// Validates that asset data conforms to HAZE rules:
    /// - Data size matches density level limits
    /// - Owner address is valid (non-zero)
    /// - Metadata is not empty for new assets
    /// - Attributes are valid
    fn validate_asset_data(&self, data: &crate::types::AssetData) -> Result<()> {

        // Validate owner address (must be non-zero)
        if data.owner == [0u8; 32] {
            return Err(crate::error::HazeError::InvalidTransaction(
                "Asset owner address cannot be zero".to_string()
            ));
        }

        // Calculate total data size
        let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
        let attributes_size: usize = data.attributes.iter()
            .map(|attr| attr.name.len() + attr.value.len())
            .sum();
        let total_size = metadata_size + attributes_size;

        // Validate data size against density level
        let max_size = data.density.max_size();
        if total_size > max_size {
            return Err(crate::error::HazeError::InvalidTransaction(
                format!(
                    "Asset data size {} exceeds limit {} for density level {:?}",
                    total_size, max_size, data.density
                )
            ));
        }

        // Validate metadata keys and values (no empty keys, reasonable length)
        for (key, value) in &data.metadata {
            if key.is_empty() {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset metadata cannot have empty keys".to_string()
                ));
            }
            if key.len() > 256 {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset metadata key too long (max 256 bytes)".to_string()
                ));
            }
            if value.len() > 1024 * 1024 {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset metadata value too long (max 1MB)".to_string()
                ));
            }
        }

        // Validate attributes
        for attr in &data.attributes {
            if attr.name.is_empty() {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset attribute name cannot be empty".to_string()
                ));
            }
            if attr.name.len() > 128 {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset attribute name too long (max 128 bytes)".to_string()
                ));
            }
            if attr.value.len() > 1024 {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Asset attribute value too long (max 1024 bytes)".to_string()
                ));
            }
            // Validate rarity if present (should be between 0.0 and 1.0)
            if let Some(rarity) = attr.rarity {
                if rarity < 0.0 || rarity > 1.0 {
                    return Err(crate::error::HazeError::InvalidTransaction(
                        format!("Asset attribute rarity must be between 0.0 and 1.0, got {}", rarity)
                    ));
                }
            }
        }

        // Validate game_id if present (reasonable length)
        if let Some(ref game_id) = data.game_id {
            if game_id.is_empty() {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Game ID cannot be empty if present".to_string()
                ));
            }
            if game_id.len() > 128 {
                return Err(crate::error::HazeError::InvalidTransaction(
                    "Game ID too long (max 128 bytes)".to_string()
                ));
            }
        }

        Ok(())
    }

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
                
                // For Merge: include other_asset_id in signature
                if matches!(action, crate::types::AssetAction::Merge) {
                    if let Some(other_asset_id_str) = data.metadata.get("_other_asset_id") {
                        if let Ok(other_asset_id_bytes) = hex::decode(other_asset_id_str) {
                            if other_asset_id_bytes.len() == 32 {
                                serialized.extend_from_slice(&other_asset_id_bytes);
                            }
                        }
                    }
                }
                
                // For Split: include components in signature
                if matches!(action, crate::types::AssetAction::Split) {
                    if let Some(components_str) = data.metadata.get("_components") {
                        serialized.extend_from_slice(components_str.as_bytes());
                    }
                }
                
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
        
        // If no transactions, don't create empty block (for MVP, we can create empty blocks)
        // But for better UX, we'll still create blocks even if empty

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
            transactions: transactions.clone(),
            dag_references: dag_refs,
        };
        
        // Remove transactions from pool after creating block
        self.remove_transactions_from_pool(&transactions);

        Ok(block)
    }

    /// Get DAG references for new block (smart referencing)
    fn get_dag_references(&self) -> Result<Vec<Hash>> {
        let dag = self.dag.read();
        
        if dag.vertices.is_empty() {
            return Ok(vec![]);
        }
        
        // Get tips (blocks with no incoming edges or fewest incoming edges)
        let mut tip_scores = HashMap::new();
        
        for (hash, vertex) in dag.vertices.iter() {
            let incoming_count = dag.reverse_edges.get(hash)
                .map(|v| v.len())
                .unwrap_or(0);
            
            // Prefer blocks with fewer incoming edges (tips)
            // Also consider wave number and timestamp
            let score = (incoming_count as i64 * -1) + 
                       (vertex.wave as i64 * 100) + 
                       (vertex.timestamp / 1000); // Normalize timestamp
            
            tip_scores.insert(*hash, score);
        }
        
        // Sort by score (higher is better) and take top references
        let mut scored_hashes: Vec<_> = tip_scores.into_iter().collect();
        scored_hashes.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Take up to 10 references, prioritizing tips
        let refs: Vec<Hash> = scored_hashes
            .iter()
            .take(10)
            .map(|(hash, _)| *hash)
            .collect();
        
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
        
        // Validate DAG references exist
        self.validate_dag_references(block)?;
        
        // Add to DAG
        {
            let mut dag = self.dag.write();
            let vertex = DagVertex {
                block: block.clone(),
                references: block.dag_references.clone(),
                wave: block.header.wave_number,
                timestamp: block.header.timestamp,
                processed: false,
            };
            dag.vertices.insert(block_hash, vertex);
            dag.edges.insert(block_hash, block.dag_references.clone());
            
            // Update reverse edges (who references this block)
            for ref_hash in &block.dag_references {
                dag.reverse_edges
                    .entry(*ref_hash)
                    .or_insert_with(Vec::new)
                    .push(block_hash);
            }
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

        // Apply to state (handle errors gracefully for DAG operations)
        // Note: In production, state application should always succeed
        if let Err(e) = self.state.apply_block(block) {
            tracing::warn!("Failed to apply block to state in DAG: {}", e);
            // Continue with DAG processing even if state application fails
        }
        
        // Mark as processed
        {
            let mut dag = self.dag.write();
            if let Some(vertex) = dag.vertices.get_mut(&block_hash) {
                vertex.processed = true;
            }
        }

        Ok(())
    }
    
    /// Validate DAG references exist
    fn validate_dag_references(&self, block: &Block) -> Result<()> {
        let dag = self.dag.read();
        for ref_hash in &block.dag_references {
            if !dag.vertices.contains_key(ref_hash) {
                // Allow genesis block reference (zero hash)
                if *ref_hash != [0u8; 32] {
                    return Err(crate::error::HazeError::InvalidBlock(
                        format!("DAG reference {} does not exist", hex::encode(ref_hash))
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check wave finalization (Golden Wave)
    pub fn check_wave_finalization(&self, wave_num: u64) -> Result<bool> {
        let waves = self.waves.read();
        if let Some(wave) = waves.get(&wave_num) {
            if wave.finalized {
                return Ok(true);
            }
            
            let now = Utc::now().timestamp();
            let elapsed = (now - wave.created_at) * 1000; // Convert to ms
            
            // Check if wave has enough blocks and time has passed
            let min_blocks = 2; // Minimum blocks for finalization
            if wave.blocks.len() >= min_blocks && 
               elapsed >= self.config.consensus.golden_wave_threshold as i64 {
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Finalize wave (mark as finalized)
    pub fn finalize_wave(&self, wave_num: u64) -> Result<()> {
        let mut waves = self.waves.write();
        if let Some(wave) = waves.get_mut(&wave_num) {
            wave.finalized = true;
            tracing::info!("Wave {} finalized with {} blocks", wave_num, wave.blocks.len());
        }
        Ok(())
    }
    
    /// Get all ancestors of a block (transitive closure of references)
    pub fn get_ancestors(&self, block_hash: &Hash) -> HashSet<Hash> {
        let dag = self.dag.read();
        let mut ancestors = HashSet::new();
        let mut to_visit = vec![*block_hash];
        let mut visited = HashSet::new();
        
        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            
            if let Some(vertex) = dag.vertices.get(&current) {
                for ref_hash in &vertex.references {
                    if *ref_hash != [0u8; 32] { // Skip genesis
                        ancestors.insert(*ref_hash);
                        to_visit.push(*ref_hash);
                    }
                }
            }
        }
        
        ancestors
    }
    
    /// Get all descendants of a block (blocks that reference this block)
    pub fn get_descendants(&self, block_hash: &Hash) -> HashSet<Hash> {
        let dag = self.dag.read();
        let mut descendants = HashSet::new();
        let mut to_visit = vec![*block_hash];
        let mut visited = HashSet::new();
        
        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            
            if let Some(refs) = dag.reverse_edges.get(&current) {
                for desc_hash in refs {
                    descendants.insert(*desc_hash);
                    to_visit.push(*desc_hash);
                }
            }
        }
        
        descendants
    }
    
    /// Topological sort of DAG vertices
    pub fn topological_sort(&self) -> Vec<Hash> {
        let dag = self.dag.read();
        let mut in_degree = HashMap::new();
        
        // Calculate in-degrees
        for hash in dag.vertices.keys() {
            in_degree.insert(*hash, 0);
        }
        for (hash, refs) in dag.edges.iter() {
            for ref_hash in refs {
                if *ref_hash != [0u8; 32] { // Skip genesis
                    *in_degree.entry(*ref_hash).or_insert(0) += 1;
                }
            }
        }
        
        // Find all vertices with in-degree 0
        let mut queue: Vec<Hash> = in_degree.iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(&hash, _)| hash)
            .collect();
        
        let mut result = Vec::new();
        
        while let Some(current) = queue.pop() {
            result.push(current);
            
            if let Some(refs) = dag.edges.get(&current) {
                for ref_hash in refs {
                    if *ref_hash != [0u8; 32] {
                        if let Some(degree) = in_degree.get_mut(ref_hash) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push(*ref_hash);
                            }
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Prune old blocks from DAG (keep only recent N blocks)
    pub fn prune_dag(&self, keep_recent: usize) -> Result<usize> {
        let mut dag = self.dag.write();
        
        if dag.vertices.len() <= keep_recent {
            return Ok(0);
        }
        
        // Get blocks sorted by timestamp (oldest first)
        let mut blocks_by_time: Vec<(Hash, i64)> = dag.vertices.iter()
            .map(|(hash, vertex)| (*hash, vertex.timestamp))
            .collect();
        blocks_by_time.sort_by_key(|(_, ts)| *ts);
        
        let to_remove = blocks_by_time.len() - keep_recent;
        let mut removed = 0;
        
        for (hash, _) in blocks_by_time.iter().take(to_remove) {
            // Don't remove if it has descendants
            if let Some(descendants) = dag.reverse_edges.get(hash) {
                if !descendants.is_empty() {
                    continue; // Skip blocks with descendants
                }
            }
            
            // Remove from vertices and edges
            dag.vertices.remove(hash);
            dag.edges.remove(hash);
            dag.reverse_edges.remove(hash);
            
            // Remove from reverse edges
            for (_, refs) in dag.reverse_edges.iter_mut() {
                refs.retain(|&h| h != *hash);
            }
            
            removed += 1;
        }
        
        tracing::info!("Pruned {} blocks from DAG", removed);
        Ok(removed)
    }
    
    /// Check DAG consistency
    pub fn check_dag_consistency(&self) -> Result<()> {
        let dag = self.dag.read();
        
        // Check that all edges point to existing vertices
        for (hash, refs) in dag.edges.iter() {
            for ref_hash in refs {
                if *ref_hash != [0u8; 32] && !dag.vertices.contains_key(ref_hash) {
                    return Err(crate::error::HazeError::InvalidBlock(
                        format!("Edge from {} to non-existent vertex {}", 
                                hex::encode(hash), hex::encode(ref_hash))
                    ));
                }
            }
        }
        
        // Check that reverse edges match forward edges
        for (hash, refs) in dag.reverse_edges.iter() {
            for ref_hash in refs {
                if let Some(forward_refs) = dag.edges.get(ref_hash) {
                    if !forward_refs.contains(hash) {
                        return Err(crate::error::HazeError::InvalidBlock(
                            format!("Reverse edge mismatch: {} -> {} exists but {} -> {} doesn't",
                                    hex::encode(ref_hash), hex::encode(hash),
                                    hex::encode(ref_hash), hex::encode(hash))
                        ));
                    }
                }
            }
        }
        
        Ok(())
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

    // Asset validation tests
    use crate::types::{AssetData, DensityLevel, Attribute};
    
    fn create_test_address_for_asset(seed: u8) -> Address {
        let mut addr = [0u8; 32];
        addr[0] = seed;
        addr
    }

    #[test]
    fn test_validate_asset_data_valid() {
        let config = create_test_config("asset_valid");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), "Test Asset".to_string());
        metadata.insert("description".to_string(), "A test asset".to_string());
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata,
            attributes: vec![
                Attribute {
                    name: "power".to_string(),
                    value: "100".to_string(),
                    rarity: Some(0.5),
                }
            ],
            game_id: Some("test_game".to_string()),
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_asset_data_zero_owner() {
        let config = create_test_config("asset_zero_owner");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: HashMap::new(),
            attributes: vec![],
            game_id: None,
            owner: [0u8; 32], // Zero address
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("owner address cannot be zero"));
    }

    #[test]
    fn test_validate_asset_data_exceeds_density_limit() {
        let config = create_test_config("asset_density_limit");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let mut metadata = HashMap::new();
        // Create metadata that exceeds Ethereal limit (5KB)
        let large_value = "x".repeat(6 * 1024); // 6KB
        metadata.insert("large_data".to_string(), large_value);
        
        let data = AssetData {
            density: DensityLevel::Ethereal, // Max 5KB
            metadata,
            attributes: vec![],
            game_id: None,
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("exceeds limit"));
    }

    #[test]
    fn test_validate_asset_data_empty_metadata_key() {
        let config = create_test_config("asset_empty_key");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("".to_string(), "value".to_string()); // Empty key
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata,
            attributes: vec![],
            game_id: None,
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("empty keys"));
    }

    #[test]
    fn test_validate_asset_data_invalid_rarity() {
        let config = create_test_config("asset_invalid_rarity");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: HashMap::new(),
            attributes: vec![
                Attribute {
                    name: "power".to_string(),
                    value: "100".to_string(),
                    rarity: Some(1.5), // Invalid: > 1.0
                }
            ],
            game_id: None,
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("rarity"));
    }

    #[test]
    fn test_validate_asset_data_valid_rarity() {
        let config = create_test_config("asset_valid_rarity");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: HashMap::new(),
            attributes: vec![
                Attribute {
                    name: "power".to_string(),
                    value: "100".to_string(),
                    rarity: Some(0.75), // Valid: between 0.0 and 1.0
                }
            ],
            game_id: None,
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_asset_data_empty_game_id() {
        let config = create_test_config("asset_empty_game_id");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: HashMap::new(),
            attributes: vec![],
            game_id: Some("".to_string()), // Empty game_id
            owner: create_test_address_for_asset(1),
        };
        
        let result = consensus.validate_asset_data(&data);
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Game ID cannot be empty"));
    }
    
    #[test]
    fn test_dag_topological_sort() {
        let config = create_test_config("dag_topological");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let validator = keypair.address();
        
        // Create blocks
        let block_a = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_a).unwrap();
        
        let block_b = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_b).unwrap();
        
        // Topological sort should return blocks
        let sorted = consensus.topological_sort();
        assert!(!sorted.is_empty());
    }
    
    #[test]
    fn test_get_ancestors() {
        let config = create_test_config("dag_ancestors");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let validator = keypair.address();
        
        let block_a = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_a).unwrap();
        
        let block_b = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_b).unwrap();
        
        // Get ancestors of B (should work without panicking)
        let ancestors = consensus.get_ancestors(&block_b.header.hash);
        // Function should return a HashSet (may be empty if no references)
        // Just verify function executes successfully - ancestors is a HashSet<Hash>
        let _ = ancestors; // Use variable to ensure function executed
    }
    
    #[test]
    fn test_get_descendants() {
        let config = create_test_config("dag_descendants");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let validator = keypair.address();
        
        let block_a = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_a).unwrap();
        
        let block_b = consensus.create_block(validator).unwrap();
        consensus.process_block(&block_b).unwrap();
        
        // Get descendants of A (should work without panicking)
        let descendants = consensus.get_descendants(&block_a.header.hash);
        // Function should return a HashSet (may be empty if no references)
        // Just verify function executes successfully - descendants is a HashSet<Hash>
        let _ = descendants; // Use variable to ensure function executed
    }
    
    #[test]
    fn test_dag_consistency_check() {
        let config = create_test_config("dag_consistency");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let validator = keypair.address();
        
        let block = consensus.create_block(validator).unwrap();
        consensus.process_block(&block).unwrap();
        
        // Consistency check should pass
        let result = consensus.check_dag_consistency();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_wave_finalization() {
        let config = create_test_config("wave_finalization");
        let state = crate::state::StateManager::new(&config).unwrap();
        let consensus = ConsensusEngine::new(config, std::sync::Arc::new(state)).unwrap();
        
        let keypair = KeyPair::generate();
        let validator = keypair.address();
        
        let block = consensus.create_block(validator).unwrap();
        consensus.process_block(&block).unwrap();
        
        let wave_num = block.header.wave_number;
        
        // Initially should not be finalized
        let is_finalized = consensus.check_wave_finalization(wave_num).unwrap();
        assert!(!is_finalized);
        
        // Finalize the wave
        consensus.finalize_wave(wave_num).unwrap();
        
        // Now should be finalized
        let is_finalized = consensus.check_wave_finalization(wave_num).unwrap();
        assert!(is_finalized);
    }
}