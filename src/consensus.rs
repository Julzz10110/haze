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
use chrono::Utc;

/// Consensus engine implementing Fog Consensus
pub struct ConsensusEngine {
    config: Config,
    state: Arc<StateManager>,
    
    // DAG structure
    dag: Arc<RwLock<DAG>>,
    
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
struct DAG {
    vertices: HashMap<Hash, DAGVertex>,
    edges: HashMap<Hash, Vec<Hash>>,
}

struct DAGVertex {
    block: Block,
    references: Vec<Hash>,
    wave: u64,
}

/// Haze Committee - dynamic validator group
struct Committee {
    id: u64,
    validators: Vec<Address>,
    weights: HashMap<Address, u64>, // Haze weights
    created_at: i64,
    expires_at: i64,
}

/// Wave for finalization
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
            dag: Arc::new(RwLock::new(DAG {
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
    pub fn add_transaction(&self, tx: Transaction) -> Result<()> {
        let tx_hash = tx.hash();
        self.tx_pool.insert(tx_hash, tx);
        Ok(())
    }

    /// Create new block
    pub fn create_block(&self, validator: Address) -> Result<Block> {
        // Check committee rotation
        // TODO: Make this mutable or use interior mutability
        
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
            state_root: [0; 32], // TODO: Compute state root
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
        // TODO: Get from latest finalized block
        Ok([0; 32]) // Genesis for now
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
            let vertex = DAGVertex {
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