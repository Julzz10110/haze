//! Mistborn Assets - Dynamic NFT system for HAZE
//! 
//! Features:
//! - Density levels (Ethereal, Light, Dense, Core)
//! - Dynamic mechanisms (Condensation, Evaporation, Merge, Split)
//! - Blob storage for large files (Core density)
//! - WASM contract integration

use crate::types::{Hash, Address, AssetData, DensityLevel, AssetAction};
use crate::error::{HazeError, Result};
use crate::vm::{HazeVM, ExecutionContext};
use crate::config::Config;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

/// Mistborn Asset manager
pub struct MistbornAsset {
    pub asset_id: Hash,
    pub data: AssetData,
    pub history: Vec<AssetHistoryEntry>,
    /// Blob references for large files (Core density)
    /// Maps blob key to blob hash
    pub blob_refs: HashMap<String, Hash>,
}

/// History entry for asset
pub struct AssetHistoryEntry {
    pub timestamp: i64,
    pub action: AssetAction,
    pub changes: HashMap<String, String>,
}

/// Blob storage for large files (Core density assets)
pub struct BlobStorage {
    storage_path: PathBuf,
    chunk_size: usize,
    max_size: usize,
}

impl BlobStorage {
    /// Create new blob storage
    pub fn new(config: &Config) -> Result<Self> {
        let storage_path = config.storage.blob_storage_path.clone();
        
        // Create blob storage directory if it doesn't exist
        fs::create_dir_all(&storage_path)
            .map_err(|e| HazeError::Asset(format!("Failed to create blob storage: {}", e)))?;
        
        Ok(Self {
            storage_path,
            chunk_size: config.storage.blob_chunk_size,
            max_size: config.storage.max_blob_size,
        })
    }
    
    /// Store blob data and return hash
    pub fn store_blob(&self, blob_key: &str, data: &[u8]) -> Result<Hash> {
        if data.len() > self.max_size {
            return Err(HazeError::Asset(format!(
                "Blob size {} exceeds maximum {} bytes",
                data.len(),
                self.max_size
            )));
        }
        
        // Compute hash of blob data
        let blob_hash = crate::types::sha256(data);
        
        // Ensure storage directory exists
        fs::create_dir_all(&self.storage_path)
            .map_err(|e| HazeError::Asset(format!("Failed to create storage directory: {}", e)))?;
        
        // Store blob in chunks if it's large
        if data.len() > self.chunk_size {
            self.store_blob_chunked(blob_key, data, &blob_hash)?;
        } else {
            let blob_path = self.get_blob_path(blob_key, &blob_hash);
            // Ensure parent directory exists
            if let Some(parent) = blob_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| HazeError::Asset(format!("Failed to create blob directory: {}", e)))?;
            }
            fs::write(&blob_path, data)
                .map_err(|e| HazeError::Asset(format!("Failed to write blob: {}", e)))?;
        }
        
        Ok(blob_hash)
    }
    
    /// Store blob in chunks for large files
    fn store_blob_chunked(&self, blob_key: &str, data: &[u8], blob_hash: &Hash) -> Result<()> {
        let base_path = self.get_blob_path(blob_key, blob_hash);
        let chunk_dir = base_path.with_extension("chunks");
        
        // Ensure parent directory exists
        if let Some(parent) = chunk_dir.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| HazeError::Asset(format!("Failed to create parent directory: {}", e)))?;
        }
        
        fs::create_dir_all(&chunk_dir)
            .map_err(|e| HazeError::Asset(format!("Failed to create chunk directory: {}", e)))?;
        
        let mut offset = 0;
        let mut chunk_index = 0;
        
        while offset < data.len() {
            let chunk_end = std::cmp::min(offset + self.chunk_size, data.len());
            let chunk_data = &data[offset..chunk_end];
            
            let chunk_path = chunk_dir.join(format!("chunk_{:08}", chunk_index));
            fs::write(&chunk_path, chunk_data)
                .map_err(|e| HazeError::Asset(format!("Failed to write chunk: {}", e)))?;
            
            offset = chunk_end;
            chunk_index += 1;
        }
        
        Ok(())
    }
    
    /// Retrieve blob data
    pub fn get_blob(&self, blob_key: &str, blob_hash: &Hash) -> Result<Vec<u8>> {
        let blob_path = self.get_blob_path(blob_key, blob_hash);
        
        // Check if it's chunked
        let chunk_dir = blob_path.with_extension("chunks");
        if chunk_dir.exists() {
            self.get_blob_chunked(&chunk_dir)
        } else {
            fs::read(&blob_path)
                .map_err(|e| HazeError::Asset(format!("Failed to read blob: {}", e)))
        }
    }
    
    /// Retrieve chunked blob
    fn get_blob_chunked(&self, chunk_dir: &PathBuf) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut chunk_index = 0;
        
        loop {
            let chunk_path = chunk_dir.join(format!("chunk_{:08}", chunk_index));
            if !chunk_path.exists() {
                break;
            }
            
            let mut chunk_data = fs::read(&chunk_path)
                .map_err(|e| HazeError::Asset(format!("Failed to read chunk: {}", e)))?;
            data.append(&mut chunk_data);
            chunk_index += 1;
        }
        
        Ok(data)
    }
    
    /// Delete blob
    pub fn delete_blob(&self, blob_key: &str, blob_hash: &Hash) -> Result<()> {
        let blob_path = self.get_blob_path(blob_key, blob_hash);
        let chunk_dir = blob_path.with_extension("chunks");
        
        if chunk_dir.exists() {
            fs::remove_dir_all(&chunk_dir)
                .map_err(|e| HazeError::Asset(format!("Failed to remove chunks: {}", e)))?;
        }
        
        if blob_path.exists() {
            fs::remove_file(&blob_path)
                .map_err(|e| HazeError::Asset(format!("Failed to remove blob: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Get blob path
    fn get_blob_path(&self, blob_key: &str, blob_hash: &Hash) -> PathBuf {
        let hash_hex = hex::encode(blob_hash);
        self.storage_path.join(format!("{}_{}", blob_key, &hash_hex[..16]))
    }
}

impl MistbornAsset {
    /// Create new Mistborn asset
    pub fn create(
        asset_id: Hash,
        owner: Address,
        initial_density: DensityLevel,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            asset_id,
            data: AssetData {
                density: initial_density,
                metadata,
                attributes: vec![],
                game_id: None,
                owner,
            },
            history: vec![],
            blob_refs: HashMap::new(),
        }
    }

    /// Condense asset (increase density) with optimized blob handling
    pub fn condense(
        &mut self,
        new_data: HashMap<String, String>,
        blob_storage: Option<&BlobStorage>,
    ) -> Result<()> {
        // Check if condensation is possible
        let next_level = match self.data.density {
            DensityLevel::Ethereal => DensityLevel::Light,
            DensityLevel::Light => DensityLevel::Dense,
            DensityLevel::Dense => DensityLevel::Core,
            DensityLevel::Core => {
                return Err(HazeError::Asset("Asset already at maximum density".to_string()));
            }
        };

        // Calculate total size of new data
        let total_size: usize = new_data.values().map(|v| v.len()).sum();
        
        // For Core density, check if we need blob storage
        let needs_blob_storage = next_level == DensityLevel::Core && total_size > DensityLevel::Dense.max_size();
        
        if needs_blob_storage {
            if blob_storage.is_none() {
                return Err(HazeError::Asset("Blob storage required for Core density assets".to_string()));
            }
            
            // Store large files in blob storage
            let blob_storage = blob_storage.unwrap();
            for (key, value) in &new_data {
                // If value is large (e.g., file path or large data), store as blob
                if value.len() > 1024 * 1024 { // 1MB threshold
                    let blob_hash = blob_storage.store_blob(
                        &format!("{}_{}", hex::encode(&self.asset_id[..8]), key),
                        value.as_bytes(),
                    )?;
                    
                    // Store blob reference instead of full data
                    self.blob_refs.insert(key.clone(), blob_hash);
                    
                    // Update metadata with blob reference
                    self.data.metadata.insert(
                        key.clone(),
                        format!("blob:{}", hex::encode(&blob_hash[..16])),
                    );
                } else {
                    self.data.metadata.insert(key.clone(), value.clone());
                }
            }
        } else {
            // For lower densities, check size limits
            if total_size > next_level.max_size() {
                return Err(HazeError::Asset(format!(
                    "Data size {} exceeds limit for {} level",
                    total_size, 
                    match next_level {
                        DensityLevel::Ethereal => "Ethereal",
                        DensityLevel::Light => "Light",
                        DensityLevel::Dense => "Dense",
                        DensityLevel::Core => "Core",
                    }
                )));
            }
            
            // Update metadata directly
            self.data.metadata.extend(new_data);
        }

        // Update density
        self.data.density = next_level;

        // Record history
        self.history.push(AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action: AssetAction::Condense,
            changes: HashMap::new(),
        });

        Ok(())
    }

    /// Evaporate asset (decrease density, archive unused data) with blob optimization
    pub fn evaporate(&mut self, blob_storage: Option<&BlobStorage>) -> Result<()> {
        let prev_level = match self.data.density {
            DensityLevel::Core => DensityLevel::Dense,
            DensityLevel::Dense => DensityLevel::Light,
            DensityLevel::Light => DensityLevel::Ethereal,
            DensityLevel::Ethereal => {
                return Err(HazeError::Asset("Asset already at minimum density".to_string()));
            }
        };

        // Archive non-essential data
        // Keep only essential metadata
        let essential_keys = vec!["name", "id", "owner", "game_id"];
        let mut archived = HashMap::new();
        let mut blobs_to_archive = Vec::new();
        
        for (key, value) in &self.data.metadata {
            if !essential_keys.contains(&key.as_str()) {
                // Check if this is a blob reference
                if value.starts_with("blob:") {
                    if let Some(blob_hash) = self.blob_refs.get(key) {
                        blobs_to_archive.push((key.clone(), *blob_hash));
                    }
                } else {
                    archived.insert(key.clone(), value.clone());
                }
            }
        }

        // Archive blobs if moving from Core density
        if self.data.density == DensityLevel::Core && blob_storage.is_some() {
            for (key, _blob_hash) in &blobs_to_archive {
                // In production, this would move to cold storage
                // For now, we just remove from active blob_refs
                self.blob_refs.remove(key);
            }
        }

        // Remove non-essential metadata
        self.data.metadata.retain(|k, _| essential_keys.contains(&k.as_str()));

        self.data.density = prev_level;

        // Record history
        self.history.push(AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action: AssetAction::Evaporate,
            changes: archived,
        });

        Ok(())
    }

    /// Merge two assets
    pub fn merge(&mut self, other: &MistbornAsset) -> Result<()> {
        // Check if merge is possible
        if self.data.owner != other.data.owner {
            return Err(HazeError::Asset("Cannot merge assets with different owners".to_string()));
        }

        // Combine metadata
        for (key, value) in &other.data.metadata {
            if !self.data.metadata.contains_key(key) {
                self.data.metadata.insert(key.clone(), value.clone());
            }
        }

        // Combine attributes
        self.data.attributes.extend(other.data.attributes.clone());

        // Increase density if needed
        if other.data.density as u8 > self.data.density as u8 {
            self.data.density = other.data.density;
        }

        // Record history
        self.history.push(AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action: AssetAction::Merge,
            changes: HashMap::new(),
        });

        Ok(())
    }

    /// Split asset into components
    pub fn split(&self, components: Vec<String>) -> Result<Vec<MistbornAsset>> {
        let mut result = Vec::new();

        for component_name in components {
            let mut component_data = AssetData {
                density: DensityLevel::Ethereal, // Start with minimum density
                metadata: HashMap::new(),
                attributes: vec![],
                game_id: self.data.game_id.clone(),
                owner: self.data.owner,
            };

            // Extract component-specific data
            if let Some(value) = self.data.metadata.get(&component_name) {
                component_data.metadata.insert(component_name.clone(), value.clone());
            }

            let component_asset = MistbornAsset {
                asset_id: crate::types::sha256(&[
                    self.asset_id.as_ref(),
                    component_name.as_bytes(),
                ].concat()),
                data: component_data,
                history: vec![],
                blob_refs: HashMap::new(),
            };

            result.push(component_asset);
        }

        Ok(result)
    }

    /// Update asset
    pub fn update(&mut self, updates: HashMap<String, String>) -> Result<()> {
        // Check if updates fit within current density
        let current_size: usize = self.data.metadata.values().map(|v| v.len()).sum();
        let update_size: usize = updates.values().map(|v| v.len()).sum();
        
        if current_size + update_size > self.data.density.max_size() {
            return Err(HazeError::Asset("Update exceeds density limit".to_string()));
        }

        // Apply updates
        for (key, value) in updates {
            self.data.metadata.insert(key, value);
        }

        // Record history
        self.history.push(AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action: AssetAction::Update,
            changes: HashMap::new(),
        });

        Ok(())
    }
    
    /// Execute condense operation via WASM contract
    pub fn condense_via_wasm(
        &mut self,
        vm: &HazeVM,
        wasm_code: &[u8],
        new_data: HashMap<String, String>,
        blob_storage: Option<&BlobStorage>,
        context: ExecutionContext,
    ) -> Result<()> {
        // Serialize new_data for WASM call
        let args = bincode::serialize(&new_data)
            .map_err(|e| HazeError::Asset(format!("Failed to serialize data: {}", e)))?;
        
        // Execute WASM contract
        let result = vm.execute_contract(wasm_code, "condense", &args, context)?;
        
        // Deserialize result
        let success: bool = bincode::deserialize(&result)
            .map_err(|e| HazeError::Asset(format!("Failed to deserialize result: {}", e)))?;
        
        if !success {
            return Err(HazeError::Asset("WASM contract condense failed".to_string()));
        }
        
        // Apply condense locally
        self.condense(new_data, blob_storage)
    }
    
    /// Execute evaporate operation via WASM contract
    pub fn evaporate_via_wasm(
        &mut self,
        vm: &HazeVM,
        wasm_code: &[u8],
        blob_storage: Option<&BlobStorage>,
        context: ExecutionContext,
    ) -> Result<()> {
        // Serialize asset_id for WASM call
        let args = bincode::serialize(&self.asset_id)
            .map_err(|e| HazeError::Asset(format!("Failed to serialize asset_id: {}", e)))?;
        
        // Execute WASM contract
        let result = vm.execute_contract(wasm_code, "evaporate", &args, context)?;
        
        // Deserialize result
        let success: bool = bincode::deserialize(&result)
            .map_err(|e| HazeError::Asset(format!("Failed to deserialize result: {}", e)))?;
        
        if !success {
            return Err(HazeError::Asset("WASM contract evaporate failed".to_string()));
        }
        
        // Apply evaporate locally
        self.evaporate(blob_storage)
    }
    
    /// Store large file as blob
    pub fn store_blob_file(
        &mut self,
        blob_key: String,
        file_data: &[u8],
        blob_storage: &BlobStorage,
    ) -> Result<Hash> {
        let blob_hash = blob_storage.store_blob(&blob_key, file_data)?;
        let blob_key_for_refs = blob_key.clone();
        self.blob_refs.insert(blob_key_for_refs, blob_hash);
        
        // Update metadata with blob reference
        self.data.metadata.insert(
            blob_key,
            format!("blob:{}", hex::encode(&blob_hash[..16])),
        );
        
        Ok(blob_hash)
    }
    
    /// Retrieve blob file
    pub fn get_blob_file(
        &self,
        blob_key: &str,
        blob_storage: &BlobStorage,
    ) -> Result<Vec<u8>> {
        let blob_hash = self.blob_refs.get(blob_key)
            .ok_or_else(|| HazeError::Asset(format!("Blob key {} not found", blob_key)))?;
        
        blob_storage.get_blob(blob_key, blob_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::types::sha256;
    
    fn create_test_config() -> Config {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        
        let mut config = Config::default();
        config.storage.blob_storage_path = std::path::PathBuf::from(format!("./test_blobs_{}", test_id));
        config
    }
    
    #[test]
    fn test_blob_storage_create() {
        let config = create_test_config();
        let blob_storage = BlobStorage::new(&config).unwrap();
        assert_eq!(blob_storage.chunk_size, config.storage.blob_chunk_size);
        
        // Cleanup
        std::fs::remove_dir_all(&config.storage.blob_storage_path).ok();
    }
    
    #[test]
    fn test_blob_storage_store_and_retrieve() {
        let config = create_test_config();
        let blob_storage = BlobStorage::new(&config).unwrap();
        
        let test_data = b"Test blob data for Mistborn NFT";
        let blob_hash = blob_storage.store_blob("test_blob", test_data).unwrap();
        
        let retrieved = blob_storage.get_blob("test_blob", &blob_hash).unwrap();
        assert_eq!(retrieved, test_data);
        
        // Cleanup
        blob_storage.delete_blob("test_blob", &blob_hash).unwrap();
        std::fs::remove_dir_all(&config.storage.blob_storage_path).ok();
    }
    
    #[test]
    fn test_condense_with_blob_storage() {
        let config = create_test_config();
        let blob_storage = BlobStorage::new(&config).unwrap();
        
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Dense,
            HashMap::new(),
        );
        
        // Create large data that requires blob storage
        let large_data = vec![0u8; 6 * 1024 * 1024]; // 6MB - exceeds Dense limit
        let mut new_data = HashMap::new();
        new_data.insert("large_file".to_string(), String::from_utf8_lossy(&large_data).to_string());
        
        // Should work with blob storage
        asset.condense(new_data, Some(&blob_storage)).unwrap();
        assert_eq!(asset.data.density, DensityLevel::Core);
        
        // Cleanup
        std::fs::remove_dir_all(&config.storage.blob_storage_path).ok();
    }
    
    #[test]
    fn test_evaporate_with_blob_storage() {
        let config = create_test_config();
        let blob_storage = BlobStorage::new(&config).unwrap();
        
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Core,
            HashMap::from([
                ("name".to_string(), "Test Asset".to_string()),
                ("id".to_string(), "123".to_string()),
            ]),
        );
        
        // Evaporate should work
        asset.evaporate(Some(&blob_storage)).unwrap();
        assert_eq!(asset.data.density, DensityLevel::Dense);
        
        // Cleanup
        std::fs::remove_dir_all(&config.storage.blob_storage_path).ok();
    }
    
    #[test]
    fn test_store_blob_file() {
        let config = create_test_config();
        let blob_storage = BlobStorage::new(&config).unwrap();
        
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Core,
            HashMap::new(),
        );
        
        let file_data = b"Large file content for Core density asset";
        let _blob_hash = asset.store_blob_file(
            "model_3d".to_string(),
            file_data,
            &blob_storage,
        ).unwrap();
        
        assert!(asset.blob_refs.contains_key("model_3d"));
        assert!(asset.data.metadata.contains_key("model_3d"));
        
        // Retrieve blob
        let retrieved = asset.get_blob_file("model_3d", &blob_storage).unwrap();
        assert_eq!(retrieved, file_data);
        
        // Cleanup
        std::fs::remove_dir_all(&config.storage.blob_storage_path).ok();
    }
}