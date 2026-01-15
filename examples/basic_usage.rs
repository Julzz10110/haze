//! HAZE API Usage Examples

use haze::{KeyPair, Tokenomics, FogEconomy, MistbornAsset, DensityLevel};
use haze::types::sha256;
use std::collections::HashMap;

fn main() {
    println!("HAZE Blockchain - Usage Examples\n");

    // Example 1: Creating a key pair
    println!("=== Example 1: Creating a Key Pair ===");
    let keypair = KeyPair::generate();
    let address = keypair.address();
    println!("Address: {}", hex::encode(address));
    println!();

    // Example 2: Working with tokenomics
    println!("=== Example 2: HAZE Tokenomics ===");
    let tokenomics = Tokenomics::new();
    println!("Initial supply: {} HAZE", tokenomics.total_supply());
    println!("Current inflation: {}%", tokenomics.inflation_rate() as f64 / 100.0);
    
    // Staking
    let validator = address;
    tokenomics.stake(validator, validator, 1_000_000_000_000_000_000).unwrap(); // 1000 HAZE
    println!("Staked: 1000 HAZE");
    
    if let Some(stake) = tokenomics.get_stake(&validator) {
        println!("Stake information: {} HAZE", stake.amount);
    }
    println!();

    // Example 3: Creating a Mistborn NFT
    println!("=== Example 3: Creating Mistborn Asset ===");
    let asset_id = sha256(b"example_asset");
    let mut asset = MistbornAsset::create(
        asset_id,
        address,
        DensityLevel::Ethereal,
        HashMap::from([
            ("name".to_string(), "Legendary Sword".to_string()),
            ("rarity".to_string(), "legendary".to_string()),
        ]),
    );
    println!("NFT created: {}", hex::encode(&asset_id[..8]));
    println!("Density level: Ethereal (5KB)");
    
    // Condensing NFT (increasing density)
    let mut new_data = HashMap::new();
    new_data.insert("texture".to_string(), "sword_texture.png".to_string());
    new_data.insert("model".to_string(), "sword_model.gltf".to_string());
    asset.condense(new_data, None).unwrap(); // None = no blob storage needed for Light density
    println!("NFT condensed: Light (50KB)");
    println!();

    // Example 4: Fog Economics
    println!("=== Example 4: Fog Economics ===");
    let economy = FogEconomy::new();
    
    // Update game activity
    economy.update_game_activity(
        "game_1".to_string(),
        1_000_000_000,
        address,
    ).unwrap();
    println!("Game activity updated for game_1");
    
    // Create liquidity pool
    let pool_id = economy.create_liquidity_pool(
        "HAZE".to_string(),
        "GOLD".to_string(),
        1_000_000_000_000_000_000, // 1M HAZE
        10_000_000_000_000_000_000, // 10M GOLD
        30, // 0.3% fee
    ).unwrap();
    println!("Liquidity pool created: {}", pool_id);
    
    // Create Vortex Market
    let market_id = economy.create_vortex_market(
        "game_1".to_string(),
        vec![
            ("HAZE".to_string(), "GOLD".to_string()),
            ("HAZE".to_string(), "SILVER".to_string()),
        ],
        haze::MarketConditions::ArbitrageOpportunity { discount: 5 },
        24, // 24 hours
    ).unwrap();
    println!("Vortex Market created: {}", market_id);
    println!();

    println!("All examples executed successfully!");
}
