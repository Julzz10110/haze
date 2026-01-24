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

        // Merge attributes with conflict resolution
        // If attribute with same name exists, keep the one with higher rarity
        // If both have same rarity or both are None, keep the source asset's attribute
        for other_attr in &other.data.attributes {
            if let Some(existing) = self.data.attributes.iter_mut().find(|a| a.name == other_attr.name) {
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
                self.data.attributes.push(other_attr.clone());
            }
        }

        // Merge blob_refs
        for (key, hash) in &other.blob_refs {
            if !self.blob_refs.contains_key(key) {
                self.blob_refs.insert(key.clone(), *hash);
            }
        }

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

            // Extract component-specific metadata
            if let Some(value) = self.data.metadata.get(&component_name) {
                component_data.metadata.insert(component_name.clone(), value.clone());
            }

            // Distribute attributes to components
            // Attributes with names matching component pattern go to that component
            // Other attributes are copied to all components (shared attributes)
            for attr in &self.data.attributes {
                // If attribute name contains component name, assign to this component
                if attr.name.contains(&component_name) || attr.name == component_name {
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
                component_data.attributes = self.data.attributes.clone();
            }

            let component_asset = MistbornAsset {
                asset_id: crate::types::sha256(&[
                    self.asset_id.as_ref(),
                    component_name.as_bytes(),
                ].concat()),
                data: component_data,
                history: vec![],
                blob_refs: HashMap::new(), // Components start with empty blob_refs
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

    /// Add or update an attribute
    pub fn add_attribute(&mut self, name: String, value: String, rarity: Option<f64>) {
        // Check if attribute already exists
        if let Some(existing) = self.data.attributes.iter_mut().find(|a| a.name == name) {
            existing.value = value;
            existing.rarity = rarity;
        } else {
            self.data.attributes.push(crate::types::Attribute {
                name,
                value,
                rarity,
            });
        }
    }

    /// Update an existing attribute's value
    pub fn update_attribute(&mut self, name: &str, value: String) -> Result<()> {
        let attr = self.data.attributes.iter_mut()
            .find(|a| a.name == name)
            .ok_or_else(|| HazeError::Asset(format!("Attribute '{}' not found", name)))?;
        
        attr.value = value;
        Ok(())
    }

    /// Remove an attribute
    pub fn remove_attribute(&mut self, name: &str) -> Result<()> {
        let index = self.data.attributes.iter()
            .position(|a| a.name == name)
            .ok_or_else(|| HazeError::Asset(format!("Attribute '{}' not found", name)))?;
        
        self.data.attributes.remove(index);
        Ok(())
    }

    /// Get an attribute by name
    pub fn get_attribute(&self, name: &str) -> Option<&crate::types::Attribute> {
        self.data.attributes.iter().find(|a| a.name == name)
    }

    /// Get all attributes
    pub fn get_attributes(&self) -> &[crate::types::Attribute] {
        &self.data.attributes
    }
}

/// Calculate gas cost for asset operations
pub fn calculate_asset_operation_gas(
    config: &crate::config::Config,
    action: &AssetAction,
    data: &AssetData,
    additional_data: Option<&HashMap<String, String>>,
) -> u64 {
    let gas_config = &config.asset_gas;
    
    match action {
        AssetAction::Create => {
            let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
            let metadata_kb = (metadata_size as u64 + 1023) / 1024; // Round up
            gas_config.create_base + (gas_config.create_per_kb * metadata_kb)
        }
        AssetAction::Update => {
            let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
            let metadata_kb = (metadata_size as u64 + 1023) / 1024; // Round up
            gas_config.update_base + (gas_config.update_per_kb * metadata_kb)
        }
        AssetAction::Condense => {
            let metadata_size: usize = data.metadata.values().map(|v| v.len()).sum();
            let metadata_kb = (metadata_size as u64 + 1023) / 1024; // Round up
            
            // Calculate density multiplier
            let density_multiplier = match data.density {
                DensityLevel::Light => 1,   // Ethereal -> Light
                DensityLevel::Dense => 2,    // Light -> Dense
                DensityLevel::Core => 5,     // Dense -> Core
                DensityLevel::Ethereal => 1, // Shouldn't happen, but default to 1
            };
            
            gas_config.condense_base * density_multiplier + (gas_config.condense_per_kb * metadata_kb)
        }
        AssetAction::Evaporate => {
            gas_config.evaporate_base // Minimal cost for archiving
        }
        AssetAction::Merge => {
            // Calculate combined size from current asset and other asset
            let current_size: usize = data.metadata.values().map(|v| v.len()).sum();
            
            // Try to get other asset size from additional_data
            let other_size = if let Some(additional) = additional_data {
                if additional.get("_other_asset_id").is_some() {
                    // We can't access the other asset here, so use a conservative estimate
                    // based on current asset size
                    current_size
                } else {
                    0
                }
            } else {
                0
            };
            
            let combined_size = current_size + other_size;
            let combined_kb = (combined_size as u64 + 1023) / 1024; // Round up
            gas_config.merge_base + (gas_config.merge_per_kb * combined_kb)
        }
        AssetAction::Split => {
            // Get number of components from additional_data
            let component_count = if let Some(additional) = additional_data {
                if let Some(components_str) = additional.get("_components") {
                    components_str.split(',').filter(|s| !s.trim().is_empty()).count() as u64
                } else {
                    1 // Default to 1 if not specified
                }
            } else {
                1
            };
            
            // Estimate component size (split current asset size by component count)
            let current_size: usize = data.metadata.values().map(|v| v.len()).sum();
            let estimated_component_size = current_size / component_count.max(1) as usize;
            let component_kb = (estimated_component_size as u64 + 1023) / 1024; // Round up
            
            gas_config.split_base 
                + (gas_config.split_per_component * component_count)
                + (gas_config.split_per_kb * component_kb * component_count)
        }
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

    #[test]
    fn test_add_and_get_attribute() {
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );

        asset.add_attribute("damage".to_string(), "10".to_string(), Some(0.5));
        
        let attr = asset.get_attribute("damage").unwrap();
        assert_eq!(attr.value, "10");
        assert_eq!(attr.rarity, Some(0.5));
    }

    #[test]
    fn test_update_attribute() {
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );

        asset.add_attribute("damage".to_string(), "10".to_string(), None);
        asset.update_attribute("damage", "15".to_string()).unwrap();
        
        let attr = asset.get_attribute("damage").unwrap();
        assert_eq!(attr.value, "15");
    }

    #[test]
    fn test_remove_attribute() {
        let asset_id = sha256(b"test_asset");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );

        asset.add_attribute("damage".to_string(), "10".to_string(), None);
        assert!(asset.get_attribute("damage").is_some());
        
        asset.remove_attribute("damage").unwrap();
        assert!(asset.get_attribute("damage").is_none());
    }

    #[test]
    fn test_merge_attributes_conflict_resolution() {
        let asset_id_1 = sha256(b"asset1");
        let asset_id_2 = sha256(b"asset2");
        let owner = [0u8; 32];
        
        let mut asset1 = MistbornAsset::create(
            asset_id_1,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );
        asset1.add_attribute("power".to_string(), "10".to_string(), Some(0.3));

        let mut asset2 = MistbornAsset::create(
            asset_id_2,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );
        asset2.add_attribute("power".to_string(), "20".to_string(), Some(0.8)); // Higher rarity

        asset1.merge(&asset2).unwrap();
        
        // Should keep attribute with higher rarity
        let attr = asset1.get_attribute("power").unwrap();
        assert_eq!(attr.value, "20");
        assert_eq!(attr.rarity, Some(0.8));
    }

    #[test]
    fn test_split_attributes_distribution() {
        let asset_id = sha256(b"composite");
        let owner = [0u8; 32];
        let mut asset = MistbornAsset::create(
            asset_id,
            owner,
            DensityLevel::Ethereal,
            HashMap::new(),
        );

        asset.add_attribute("component1_power".to_string(), "10".to_string(), None);
        asset.add_attribute("shared_rarity".to_string(), "epic".to_string(), Some(0.9));
        asset.add_attribute("power".to_string(), "100".to_string(), None);

        let components = asset.split(vec!["component1".to_string(), "component2".to_string()]).unwrap();
        
        // Component 1 should have component1_power and shared attributes
        let comp1 = &components[0];
        assert!(comp1.get_attribute("component1_power").is_some());
        assert!(comp1.get_attribute("shared_rarity").is_some());
        assert!(comp1.get_attribute("power").is_some());
    }

    #[test]
    fn test_calculate_asset_operation_gas() {
        use crate::types::{AssetAction, AssetData, DensityLevel};
        use std::collections::HashMap;

        let config = Config::default();
        let mut meta = HashMap::new();
        meta.insert("k".to_string(), "v".to_string());
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: meta.clone(),
            attributes: vec![],
            game_id: None,
            owner: [1u8; 32],
        };

        assert!(calculate_asset_operation_gas(&config, &AssetAction::Create, &data, None) > 0);
        assert!(calculate_asset_operation_gas(&config, &AssetAction::Update, &data, None) > 0);
        assert!(calculate_asset_operation_gas(&config, &AssetAction::Evaporate, &data, None) > 0);

        let mut condense_data = data.clone();
        condense_data.density = DensityLevel::Light;
        assert!(calculate_asset_operation_gas(&config, &AssetAction::Condense, &condense_data, None) > 0);

        assert!(calculate_asset_operation_gas(&config, &AssetAction::Merge, &data, None) > 0);

        let mut add = HashMap::new();
        add.insert("_components".to_string(), "a,b".to_string());
        assert!(calculate_asset_operation_gas(&config, &AssetAction::Split, &data, Some(&add)) > 0);
    }
}