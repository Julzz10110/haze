//! Core types for HAZE blockchain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};

/// Unique identifier for blocks, transactions, and assets
pub type Hash = [u8; 32];
pub type Address = [u8; 32];
pub type Timestamp = i64;

/// Convert bytes to hex string
pub fn hash_to_hex(hash: &Hash) -> String {
    hex::encode(hash)
}

/// Convert hex string to hash
pub fn hex_to_hash(s: &str) -> Option<Hash> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() == 32 {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Some(hash)
    } else {
        None
    }
}

/// Compute SHA256 hash
pub fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Block in the HAZE blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub dag_references: Vec<Hash>, // DAG structure for Fog Consensus
}

/// Block header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub hash: Hash,
    pub parent_hash: Hash,
    pub height: u64,
    pub timestamp: Timestamp,
    pub validator: Address,
    pub merkle_root: Hash,
    pub state_root: Hash,
    pub wave_number: u64, // Wave finalization number
    pub committee_id: u64, // Haze Committee ID
}

impl BlockHeader {
    pub fn compute_hash(&self) -> Hash {
        let data = bincode::serialize(self).unwrap();
        sha256(&data)
    }
}

/// Transaction types in HAZE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    /// Transfer HAZE tokens
    Transfer {
        from: Address,
        to: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
        signature: Vec<u8>,
    },
    /// Create or update Mistborn NFT
    MistbornAsset {
        action: AssetAction,
        asset_id: Hash,
        data: AssetData,
        signature: Vec<u8>,
    },
    /// Execute smart contract
    ContractCall {
        contract: Address,
        method: String,
        args: Vec<u8>,
        gas_limit: u64,
        signature: Vec<u8>,
    },
    /// Stake tokens for validation
    Stake {
        validator: Address,
        amount: u64,
        signature: Vec<u8>,
    },
}

/// Actions for Mistborn assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetAction {
    Create,
    Update,
    Condense, // Increase density
    Evaporate, // Decrease density
    Merge,
    Split,
}

/// Asset data with density levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetData {
    pub density: DensityLevel,
    pub metadata: HashMap<String, String>,
    pub attributes: Vec<Attribute>,
    pub game_id: Option<String>,
    pub owner: Address,
}

/// Density levels for Mistborn assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityLevel {
    Ethereal, // 5KB - basic metadata
    Light,    // 50KB - main attributes + textures
    Dense,    // 5MB - full set + 3D model
    Core,     // 50MB+ - all data + history
}

impl DensityLevel {
    pub fn max_size(&self) -> usize {
        match self {
            DensityLevel::Ethereal => 5 * 1024,
            DensityLevel::Light => 50 * 1024,
            DensityLevel::Dense => 5 * 1024 * 1024,
            DensityLevel::Core => 50 * 1024 * 1024,
        }
    }
}

/// Attribute for NFT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub rarity: Option<f64>,
}

/// Transaction hash
impl Transaction {
    pub fn hash(&self) -> Hash {
        let data = bincode::serialize(self).unwrap();
        sha256(&data)
    }
}