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
    /// Set asset permissions (owner only)
    SetAssetPermissions {
        asset_id: Hash,
        permissions: Vec<AssetPermission>,
        public_read: bool,
        owner: Address,
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

/// Permission level for asset access (granted to non-owners)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Limited access: asset operations allowed only for matching game_id
    GameContract,
    /// Read-only access
    PublicRead,
}

/// Permission grant for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPermission {
    /// Address that receives the permission
    pub grantee: Address,
    /// Permission level
    pub level: PermissionLevel,
    /// For GameContract: restricts access to this game_id only
    pub game_id: Option<String>,
    /// Optional expiration timestamp (Unix seconds)
    pub expires_at: Option<i64>,
}

/// Transaction hash
impl Transaction {
    pub fn hash(&self) -> Hash {
        let data = bincode::serialize(self).unwrap();
        sha256(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash() {
        let data = b"Hello, HAZE!";
        let hash = sha256(data);
        
        // Hash should not be all zeros
        assert_ne!(hash, [0u8; 32]);
        
        // Same input should produce same hash
        let hash2 = sha256(data);
        assert_eq!(hash, hash2);
        
        // Different input should produce different hash
        let hash3 = sha256(b"Different data");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_hash_to_hex_and_back() {
        let original_hash = sha256(b"test data");
        let hex_string = hash_to_hex(&original_hash);
        
        // Should be able to convert back
        let restored_hash = hex_to_hash(&hex_string).unwrap();
        assert_eq!(original_hash, restored_hash);
    }

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            amount: 1000,
            fee: 10,
            nonce: 1,
            signature: vec![],
        };
        
        let hash1 = tx.hash();
        let hash2 = tx.hash();
        
        // Same transaction should have same hash
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, [0u8; 32]);
    }

    #[test]
    fn test_density_level_max_size() {
        assert_eq!(DensityLevel::Ethereal.max_size(), 5 * 1024);
        assert_eq!(DensityLevel::Light.max_size(), 50 * 1024);
        assert_eq!(DensityLevel::Dense.max_size(), 5 * 1024 * 1024);
        assert_eq!(DensityLevel::Core.max_size(), 50 * 1024 * 1024);
    }

    #[test]
    fn test_block_header_compute_hash() {
        let header = BlockHeader {
            hash: [0; 32],
            parent_hash: [1; 32],
            height: 1,
            timestamp: 1000,
            validator: [2; 32],
            merkle_root: [3; 32],
            state_root: [4; 32],
            wave_number: 0,
            committee_id: 1,
        };
        
        let hash = header.compute_hash();
        assert_ne!(hash, [0u8; 32]);
        
        // Hash should be consistent
        let hash2 = header.compute_hash();
        assert_eq!(hash, hash2);
    }
}