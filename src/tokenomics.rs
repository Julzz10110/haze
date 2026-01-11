//! HAZE Tokenomics - token and economics system
//! 
//! Features:
//! - HAZE token emission and distribution
//! - Staking system for validators
//! - Gas fee burning (50%)
//! - Treasury management
//! - Inflation control

use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use crate::types::Address;
use crate::error::{HazeError, Result};

/// HAZE token constants
pub const INITIAL_SUPPLY: u64 = 1_000_000_000_000_000_000; // 1 billion HAZE (18 decimals)
pub const INITIAL_INFLATION_RATE: u64 = 3; // 3% per year
pub const INFLATION_DECAY: u64 = 5; // Decreases by 0.5% each year (in basis points)
pub const STAKER_REWARD_RATIO: u64 = 70; // 70% to stakers
pub const TREASURY_RATIO: u64 = 30; // 30% to treasury
pub const GAS_BURN_RATIO: u64 = 50; // 50% of gas fees are burned
pub const BLOCKS_PER_YEAR: u64 = 31_536_000; // ~365 days * 24 hours * 60 minutes * 60 seconds (1 second blocks)

/// Tokenomics manager
pub struct Tokenomics {
    /// Total supply
    total_supply: Arc<RwLock<u64>>,
    
    /// Circulating supply
    circulating_supply: Arc<RwLock<u64>>,
    
    /// Burned tokens
    burned_supply: Arc<RwLock<u64>>,
    
    /// Current inflation rate (in basis points, e.g., 300 = 3%)
    current_inflation_rate: Arc<RwLock<u64>>,
    
    /// Year counter for inflation decay
    current_year: Arc<RwLock<u64>>,
    
    /// Treasury balance
    treasury: Arc<RwLock<u64>>,
    
    /// Staking records
    stakes: Arc<DashMap<Address, StakeRecord>>,
    
    /// Validator set
    validators: Arc<DashMap<Address, ValidatorInfo>>,
}

/// Stake record
#[derive(Debug, Clone)]
pub struct StakeRecord {
    pub validator: Address,
    pub amount: u64,
    pub staked_at: DateTime<Utc>,
    pub last_reward: DateTime<Utc>,
    pub accumulated_rewards: u64,
}

/// Validator information
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub address: Address,
    pub total_staked: u64,
    pub self_stake: u64,
    pub delegator_count: u64,
    pub reputation_score: u64,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
}

impl Tokenomics {
    pub fn new() -> Self {
        Self {
            total_supply: Arc::new(RwLock::new(INITIAL_SUPPLY)),
            circulating_supply: Arc::new(RwLock::new(INITIAL_SUPPLY)),
            burned_supply: Arc::new(RwLock::new(0)),
            current_inflation_rate: Arc::new(RwLock::new(INITIAL_INFLATION_RATE * 100)), // In basis points
            current_year: Arc::new(RwLock::new(0)),
            treasury: Arc::new(RwLock::new(0)),
            stakes: Arc::new(DashMap::new()),
            validators: Arc::new(DashMap::new()),
        }
    }

    /// Get total supply
    pub fn total_supply(&self) -> u64 {
        *self.total_supply.read()
    }

    /// Get circulating supply
    pub fn circulating_supply(&self) -> u64 {
        *self.circulating_supply.read()
    }

    /// Get burned supply
    pub fn burned_supply(&self) -> u64 {
        *self.burned_supply.read()
    }

    /// Get current inflation rate (in basis points)
    pub fn inflation_rate(&self) -> u64 {
        *self.current_inflation_rate.read()
    }

    /// Process block rewards and inflation
    pub fn process_block_rewards(&self, block_height: u64) -> Result<u64> {
        let blocks_since_start = block_height;
        let current_year_num = blocks_since_start / BLOCKS_PER_YEAR;
        
        // Update inflation rate if year changed
        {
            let mut year = self.current_year.write();
            if current_year_num > *year {
                let years_passed = current_year_num - *year;
                let mut inflation = self.current_inflation_rate.write();
                
                // Decay inflation by 0.5% per year (50 basis points)
                for _ in 0..years_passed {
                    if *inflation > 0 {
                        *inflation = (*inflation).saturating_sub(INFLATION_DECAY * 100);
                    }
                }
                
                *year = current_year_num;
            }
        }

        // Calculate annual inflation amount
        let inflation_rate = *self.current_inflation_rate.read();
        let annual_inflation = self.circulating_supply() * inflation_rate / 10_000;
        
        // Per-block inflation
        let block_inflation = annual_inflation / BLOCKS_PER_YEAR;
        
        // Update total supply
        *self.total_supply.write() += block_inflation;
        *self.circulating_supply.write() += block_inflation;

        Ok(block_inflation)
    }

    /// Distribute block rewards
    pub fn distribute_rewards(&self, block_reward: u64, validator: Address) -> Result<()> {
        let staker_reward = block_reward * STAKER_REWARD_RATIO / 100;
        let treasury_reward = block_reward * TREASURY_RATIO / 100;

        // Distribute to stakers
        self.distribute_staker_rewards(staker_reward, validator)?;

        // Add to treasury
        *self.treasury.write() += treasury_reward;

        Ok(())
    }

    /// Distribute rewards to stakers
    fn distribute_staker_rewards(&self, total_reward: u64, validator: Address) -> Result<()> {
        if let Some(validator_info) = self.validators.get(&validator) {
            let total_staked = validator_info.total_staked;
            
            if total_staked == 0 {
                return Ok(());
            }

            // Distribute rewards proportionally
            for mut stake in self.stakes.iter_mut() {
                if stake.value().validator == validator {
                    let reward_share = total_reward * stake.value().amount / total_staked;
                    stake.value_mut().accumulated_rewards += reward_share;
                    stake.value_mut().last_reward = Utc::now();
                }
            }

            Ok(())
        } else {
            Err(HazeError::State("Validator not found".to_string()))
        }
    }

    /// Stake tokens
    pub fn stake(&self, staker: Address, validator: Address, amount: u64) -> Result<()> {
        if amount == 0 {
            return Err(HazeError::State("Cannot stake zero amount".to_string()));
        }

        // Get or create stake record
        let mut stake = self.stakes.entry(staker).or_insert_with(|| StakeRecord {
            validator,
            amount: 0,
            staked_at: Utc::now(),
            last_reward: Utc::now(),
            accumulated_rewards: 0,
        });

        if stake.validator != validator {
            return Err(HazeError::State("Cannot stake to different validator".to_string()));
        }

        stake.amount += amount;

        // Update validator info
        let mut validator_info = self.validators.entry(validator)
            .or_insert_with(|| ValidatorInfo {
                address: validator,
                total_staked: 0,
                self_stake: 0,
                delegator_count: 0,
                reputation_score: 0,
                is_active: false,
                joined_at: Utc::now(),
            });

        if staker == validator {
            validator_info.self_stake += amount;
        } else {
            validator_info.delegator_count += 1;
        }
        validator_info.total_staked += amount;

        Ok(())
    }

    /// Unstake tokens
    pub fn unstake(&self, staker: Address, amount: u64) -> Result<u64> {
        let mut stake = self.stakes.get_mut(&staker)
            .ok_or_else(|| HazeError::State("Stake record not found".to_string()))?;

        if stake.amount < amount {
            return Err(HazeError::State("Insufficient staked amount".to_string()));
        }

        stake.amount -= amount;

        // Update validator info
        if let Some(mut validator_info) = self.validators.get_mut(&stake.validator) {
            if staker == stake.validator {
                validator_info.self_stake = validator_info.self_stake.saturating_sub(amount);
            }
            validator_info.total_staked = validator_info.total_staked.saturating_sub(amount);
            
            if staker != stake.validator && stake.amount == 0 {
                validator_info.delegator_count = validator_info.delegator_count.saturating_sub(1);
            }
        }

        // Return accumulated rewards
        let rewards = stake.accumulated_rewards;
        stake.accumulated_rewards = 0;

        Ok(rewards)
    }

    /// Process gas fee (burn 50%)
    pub fn process_gas_fee(&self, gas_fee: u64) -> Result<u64> {
        let burn_amount = gas_fee * GAS_BURN_RATIO / 100;
        let remaining = gas_fee - burn_amount;

        // Burn tokens
        *self.burned_supply.write() += burn_amount;
        *self.circulating_supply.write() = self.circulating_supply().saturating_sub(burn_amount);

        Ok(remaining)
    }

    /// Get stake record
    pub fn get_stake(&self, staker: &Address) -> Option<StakeRecord> {
        self.stakes.get(staker).map(|s| s.clone())
    }

    /// Get validator info
    pub fn get_validator(&self, validator: &Address) -> Option<ValidatorInfo> {
        self.validators.get(validator).map(|v| v.clone())
    }

    /// Get treasury balance
    pub fn treasury_balance(&self) -> u64 {
        *self.treasury.read()
    }

    /// Claim rewards from treasury
    pub fn claim_from_treasury(&self, amount: u64) -> Result<()> {
        let mut treasury = self.treasury.write();
        if *treasury < amount {
            return Err(HazeError::State("Insufficient treasury balance".to_string()));
        }
        *treasury -= amount;
        Ok(())
    }

    /// Calculate validator reputation based on game activity
    pub fn update_validator_reputation(&self, validator: Address, game_activity: u64) -> Result<()> {
        if let Some(mut validator_info) = self.validators.get_mut(&validator) {
            // Reputation increases with game activity
            validator_info.reputation_score = validator_info.reputation_score.saturating_add(game_activity);
            Ok(())
        } else {
            Err(HazeError::State("Validator not found".to_string()))
        }
    }

    /// Get top validators by stake
    pub fn get_top_validators(&self, limit: usize) -> Vec<ValidatorInfo> {
        let mut validators: Vec<ValidatorInfo> = self.validators.iter()
            .map(|v| v.value().clone())
            .collect();
        
        validators.sort_by(|a, b| b.total_staked.cmp(&a.total_staked));
        validators.truncate(limit);
        
        validators
    }
}

impl Default for Tokenomics {
    fn default() -> Self {
        Self::new()
    }
}
