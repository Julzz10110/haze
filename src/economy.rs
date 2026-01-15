//! Fog Economics - HAZE gaming economy
//! 
//! Features:
//! - Fog liquidity (dynamic liquidity based on gaming activity)
//! - Vortex markets (spontaneous trading points)
//! - Fog treasury (automatic revenue distribution)

use std::sync::Arc;
use dashmap::DashMap;
use chrono::{DateTime, Utc, Duration};
use crate::types::Address;
use crate::error::{HazeError, Result};

/// Fog Economics manager
pub struct FogEconomy {
    /// Regional economic zones (by game ID)
    economic_zones: Arc<DashMap<String, EconomicZone>>,
    
    /// Vortex markets (spontaneous trading points)
    vortex_markets: Arc<DashMap<String, VortexMarket>>,
    
    /// Asset liquidity pools
    liquidity_pools: Arc<DashMap<String, LiquidityPool>>,
    
    /// Game activity tracking
    game_activity: Arc<DashMap<String, GameActivity>>,
}

/// Economic zone within a game
#[derive(Debug, Clone)]
pub struct EconomicZone {
    pub game_id: String,
    pub zone_id: String,
    pub liquidity: u64,
    pub demand_factor: f64, // 0.0 to 2.0, affects prices
    pub supply_factor: f64, // 0.0 to 2.0, affects availability
    pub last_activity: DateTime<Utc>,
    pub activity_score: u64,
}

/// Vortex market - spontaneous trading point
#[derive(Debug, Clone)]
pub struct VortexMarket {
    pub market_id: String,
    pub game_id: String,
    pub asset_pairs: Vec<(String, String)>, // (asset1, asset2)
    pub special_conditions: MarketConditions,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
    pub volume_24h: u64,
}

/// Market conditions for vortex markets
#[derive(Debug, Clone)]
pub enum MarketConditions {
    ArbitrageOpportunity { discount: u64 }, // Percentage discount
    LimitedTimeAuction,
    FlashSale { duration_seconds: u64 },
    CommunityEvent,
}

/// Liquidity pool for assets
#[derive(Debug, Clone)]
pub struct LiquidityPool {
    pub pool_id: String,
    pub asset1: String,
    pub asset2: String,
    pub reserve1: u64,
    pub reserve2: u64,
    pub k: u128, // Constant product (reserve1 * reserve2)
    pub fee_rate: u64, // Basis points (e.g., 30 = 0.3%)
    pub total_liquidity: u64,
}

/// Game activity tracking
#[derive(Debug, Clone)]
pub struct GameActivity {
    pub game_id: String,
    pub transactions_24h: u64,
    pub unique_players_24h: u64,
    pub volume_24h: u64,
    pub last_update: DateTime<Utc>,
    pub activity_trend: f64, // Positive = increasing, Negative = decreasing
}

impl FogEconomy {
    pub fn new() -> Self {
        Self {
            economic_zones: Arc::new(DashMap::new()),
            vortex_markets: Arc::new(DashMap::new()),
            liquidity_pools: Arc::new(DashMap::new()),
            game_activity: Arc::new(DashMap::new()),
        }
    }

    /// Update game activity
    pub fn update_game_activity(
        &self,
        game_id: String,
        transaction_value: u64,
        _player_address: Address,
    ) -> Result<()> {
        let now = Utc::now();
        let mut activity = self.game_activity.entry(game_id.clone())
            .or_insert_with(|| GameActivity {
                game_id: game_id.clone(),
                transactions_24h: 0,
                unique_players_24h: 0,
                volume_24h: 0,
                last_update: now,
                activity_trend: 0.0,
            });

        // Update 24h window
        let day_ago = now - Duration::days(1);
        if activity.last_update < day_ago {
            // Reset 24h metrics
            activity.transactions_24h = 1;
            activity.volume_24h = transaction_value;
            activity.unique_players_24h = 1;
            activity.activity_trend = 0.0;
        } else {
            activity.transactions_24h += 1;
            activity.volume_24h += transaction_value;
        }

        activity.last_update = now;

        // Update economic zone liquidity based on activity
        self.update_zone_liquidity(&game_id, transaction_value)?;

        Ok(())
    }

    /// Update zone liquidity based on activity
    fn update_zone_liquidity(&self, game_id: &str, activity_value: u64) -> Result<()> {
        // Find or create economic zone
        let zone_key = format!("{}:main", game_id);
        let mut zone = self.economic_zones.entry(zone_key.clone())
            .or_insert_with(|| EconomicZone {
                game_id: game_id.to_string(),
                zone_id: "main".to_string(),
                liquidity: 1_000_000, // Initial liquidity
                demand_factor: 1.0,
                supply_factor: 1.0,
                last_activity: Utc::now(),
                activity_score: 0,
            });

        // Increase liquidity with activity
        zone.liquidity += activity_value / 100; // Small increment to prevent inflation
        zone.activity_score += activity_value;
        zone.last_activity = Utc::now();

        // Adjust demand/supply factors based on activity
        let activity_rate = zone.activity_score as f64 / 1_000_000.0;
        if activity_rate > 1.0 {
            zone.demand_factor = 1.0 + (activity_rate - 1.0).min(1.0);
            zone.supply_factor = 1.0 - (activity_rate - 1.0).min(0.5) * 0.5; // Limited supply when high demand
        } else {
            zone.demand_factor = activity_rate.max(0.5);
            zone.supply_factor = 1.0 + (1.0 - activity_rate).min(0.5);
        }

        Ok(())
    }

    /// Create vortex market (spontaneous trading point)
    pub fn create_vortex_market(
        &self,
        game_id: String,
        asset_pairs: Vec<(String, String)>,
        conditions: MarketConditions,
        duration_hours: u64,
    ) -> Result<String> {
        let market_id = format!("vortex:{}:{}", game_id, Utc::now().timestamp());
        let now = Utc::now();
        
        let market = VortexMarket {
            market_id: market_id.clone(),
            game_id,
            asset_pairs,
            special_conditions: conditions,
            created_at: now,
            expires_at: now + Duration::hours(duration_hours as i64),
            is_active: true,
            volume_24h: 0,
        };

        self.vortex_markets.insert(market_id.clone(), market);

        Ok(market_id)
    }

    /// Get active vortex markets for a game
    pub fn get_active_vortex_markets(&self, game_id: &str) -> Vec<VortexMarket> {
        let now = Utc::now();
        self.vortex_markets.iter()
            .filter(|m| {
                m.value().game_id == game_id
                    && m.value().is_active
                    && m.value().expires_at > now
            })
            .map(|m| m.value().clone())
            .collect()
    }

    /// Create liquidity pool
    pub fn create_liquidity_pool(
        &self,
        asset1: String,
        asset2: String,
        initial_reserve1: u64,
        initial_reserve2: u64,
        fee_rate: u64, // Basis points
    ) -> Result<String> {
        let pool_id = format!("pool:{}:{}", asset1, asset2);
        
        if self.liquidity_pools.contains_key(&pool_id) {
            return Err(HazeError::State("Liquidity pool already exists".to_string()));
        }

        let k = initial_reserve1 as u128 * initial_reserve2 as u128;
        let total_liquidity = initial_reserve1 + initial_reserve2;

        let pool = LiquidityPool {
            pool_id: pool_id.clone(),
            asset1,
            asset2,
            reserve1: initial_reserve1,
            reserve2: initial_reserve2,
            k,
            fee_rate,
            total_liquidity,
        };

        self.liquidity_pools.insert(pool_id.clone(), pool);

        Ok(pool_id)
    }
    
    /// Get liquidity pool by ID
    pub fn get_liquidity_pool(&self, pool_id: &str) -> Option<LiquidityPool> {
        self.liquidity_pools.get(pool_id).map(|p| p.clone())
    }
    
    /// Get all liquidity pools (for API access)
    pub fn liquidity_pools(&self) -> &Arc<DashMap<String, LiquidityPool>> {
        &self.liquidity_pools
    }

    /// Swap assets in liquidity pool (constant product formula)
    pub fn swap_assets(
        &self,
        pool_id: &str,
        asset_in: &str,
        amount_in: u64,
    ) -> Result<u64> {
        let mut pool = self.liquidity_pools.get_mut(pool_id)
            .ok_or_else(|| HazeError::State("Liquidity pool not found".to_string()))?;

        // Determine which asset is being swapped
        let (reserve_in, reserve_out) = if asset_in == pool.asset1 {
            (pool.reserve1, pool.reserve2)
        } else if asset_in == pool.asset2 {
            (pool.reserve2, pool.reserve1)
        } else {
            return Err(HazeError::State("Asset not in pool".to_string()));
        };

        // Calculate fee
        let fee = amount_in * pool.fee_rate / 10_000;
        let amount_in_after_fee = amount_in - fee;

        // Constant product formula: k = reserve_in * reserve_out
        // New k must be maintained
        let new_reserve_in = reserve_in + amount_in_after_fee;
        let new_reserve_out = (pool.k / new_reserve_in as u128) as u64;
        let amount_out = reserve_out.saturating_sub(new_reserve_out);

        if amount_out == 0 {
            return Err(HazeError::State("Insufficient liquidity".to_string()));
        }

        // Update reserves
        if asset_in == pool.asset1 {
            pool.reserve1 = new_reserve_in;
            pool.reserve2 = new_reserve_out;
        } else {
            pool.reserve2 = new_reserve_in;
            pool.reserve1 = new_reserve_out;
        }

        // Update k (should be same or slightly larger due to fee)
        pool.k = pool.reserve1 as u128 * pool.reserve2 as u128;

        Ok(amount_out)
    }

    /// Add liquidity to pool
    pub fn add_liquidity(
        &self,
        pool_id: &str,
        amount1: u64,
        amount2: u64,
    ) -> Result<u64> {
        let mut pool = self.liquidity_pools.get_mut(pool_id)
            .ok_or_else(|| HazeError::State("Liquidity pool not found".to_string()))?;

        // Calculate liquidity tokens based on ratio
        let liquidity_tokens = if pool.total_liquidity == 0 {
            amount1 + amount2 // First liquidity provider
        } else {
            let ratio1 = amount1 * pool.total_liquidity / pool.reserve1;
            let ratio2 = amount2 * pool.total_liquidity / pool.reserve2;
            ratio1.min(ratio2) // Take minimum to maintain ratio
        };

        pool.reserve1 += amount1;
        pool.reserve2 += amount2;
        pool.total_liquidity += liquidity_tokens;
        pool.k = pool.reserve1 as u128 * pool.reserve2 as u128;

        Ok(liquidity_tokens)
    }

    /// Get economic zone info
    pub fn get_economic_zone(&self, game_id: &str, zone_id: &str) -> Option<EconomicZone> {
        let key = format!("{}:{}", game_id, zone_id);
        self.economic_zones.get(&key).map(|z| z.clone())
    }

    /// Get game activity
    pub fn get_game_activity(&self, game_id: &str) -> Option<GameActivity> {
        self.game_activity.get(game_id).map(|a| a.clone())
    }
}

impl Default for FogEconomy {
    fn default() -> Self {
        Self::new()
    }
}
