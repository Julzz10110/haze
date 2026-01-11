//! Mistborn Assets - Dynamic NFT system for HAZE
//! 
//! Features:
//! - Density levels (Ethereal, Light, Dense, Core)
//! - Dynamic mechanisms (Condensation, Evaporation, Merge, Split)

use crate::types::{Hash, Address, AssetData, DensityLevel, AssetAction};
use crate::error::{HazeError, Result};
use std::collections::HashMap;

/// Mistborn Asset manager
pub struct MistbornAsset {
    pub asset_id: Hash,
    pub data: AssetData,
    pub history: Vec<AssetHistoryEntry>,
}

/// History entry for asset
pub struct AssetHistoryEntry {
    pub timestamp: i64,
    pub action: AssetAction,
    pub changes: HashMap<String, String>,
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
        }
    }

    /// Condense asset (increase density)
    pub fn condense(&mut self, new_data: HashMap<String, String>) -> Result<()> {
        // Check if condensation is possible
        let next_level = match self.data.density {
            DensityLevel::Ethereal => DensityLevel::Light,
            DensityLevel::Light => DensityLevel::Dense,
            DensityLevel::Dense => DensityLevel::Core,
            DensityLevel::Core => {
                return Err(HazeError::Asset("Asset already at maximum density".to_string()));
            }
        };

        // Check data size
        let total_size: usize = new_data.values().map(|v| v.len()).sum();
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

        // Update density and data
        self.data.density = next_level;
        self.data.metadata.extend(new_data);

        // Record history
        self.history.push(AssetHistoryEntry {
            timestamp: chrono::Utc::now().timestamp(),
            action: AssetAction::Condense,
            changes: HashMap::new(),
        });

        Ok(())
    }

    /// Evaporate asset (decrease density, archive unused data)
    pub fn evaporate(&mut self) -> Result<()> {
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
        
        for (key, value) in &self.data.metadata {
            if !essential_keys.contains(&key.as_str()) {
                archived.insert(key.clone(), value.clone());
                // In real implementation, this would be moved to cold storage
            }
        }

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
}