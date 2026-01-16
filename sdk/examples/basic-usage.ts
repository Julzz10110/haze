/**
 * Basic usage examples for HAZE SDK
 */

import {
  HazeClient,
  KeyPair,
  TransactionBuilder,
  MistbornAsset,
  FogEconomy,
  DensityLevel,
  DEFAULT_API_URL,
} from '../src/index';

async function main() {
  console.log('HAZE Blockchain SDK - Basic Usage Examples\n');

  // Initialize client
  const client = new HazeClient({
    baseUrl: process.env.HAZE_API_URL || DEFAULT_API_URL,
  });

  try {
    // Example 1: Health check
    console.log('=== Example 1: Health Check ===');
    const health = await client.healthCheck();
    console.log('Health:', health);
    console.log();

    // Example 2: Get blockchain info
    console.log('=== Example 2: Blockchain Info ===');
    const info = await client.getBlockchainInfo();
    console.log('Current height:', info.current_height);
    console.log('Total supply:', info.total_supply.toString(), 'HAZE');
    console.log('Current wave:', info.current_wave);
    console.log();

    // Example 3: Generate key pair
    console.log('=== Example 3: Generate Key Pair ===');
    const keyPair = await KeyPair.generate();
    const address = keyPair.getAddressHex();
    console.log('Address:', address);
    console.log('Public key:', keyPair.getPublicKeyHex());
    console.log();

    // Example 4: Get account balance
    console.log('=== Example 4: Get Account Balance ===');
    try {
      const balance = await client.getBalance(address);
      console.log('Balance:', balance.toString(), 'HAZE');
    } catch (error: any) {
      console.log('Account not found or error:', error.message);
    }
    console.log();

    // Example 5: Create and sign a transfer transaction
    console.log('=== Example 5: Create Transfer Transaction ===');
    const recipient = await KeyPair.generate();
    const recipientAddress = recipient.getAddress();
    
    const transferTx = TransactionBuilder.createTransfer(
      keyPair.getAddress(),
      recipientAddress,
      BigInt(1000000000000000000), // 1000 HAZE (with 18 decimals)
      BigInt(1000000000000000),    // 0.001 HAZE fee
      0 // nonce
    );

    const signedTx = await TransactionBuilder.sign(transferTx, keyPair);
    const txHash = TransactionBuilder.getHashHex(signedTx);
    console.log('Transaction hash:', txHash);
    console.log('Transaction type:', signedTx.type);
    console.log('Amount:', signedTx.amount.toString(), 'HAZE');
    console.log();

    // Example 6: Create Mistborn Asset
    console.log('=== Example 6: Create Mistborn Asset ===');
    const assetId = MistbornAsset.createAssetId('legendary_sword_001');
    const assetTx = MistbornAsset.createCreateTransaction(
      assetId,
      keyPair.getAddress(),
      DensityLevel.Ethereal,
      {
        name: 'Legendary Sword',
        rarity: 'legendary',
        game_id: 'fantasy_rpg',
      },
      [
        { name: 'attack', value: '100', rarity: 0.95 },
        { name: 'defense', value: '50', rarity: 0.85 },
      ],
      'fantasy_rpg'
    );

    const signedAssetTx = await MistbornAsset.sign(assetTx, keyPair);
    console.log('Asset ID:', MistbornAsset.assetIdToHex(assetId));
    console.log('Density:', signedAssetTx.data.density);
    console.log('Metadata:', signedAssetTx.data.metadata);
    console.log();

    // Example 7: Fog Economy - Get liquidity pools
    console.log('=== Example 7: Fog Economy ===');
    const economy = new FogEconomy(client);
    try {
      const pools = await economy.getLiquidityPools();
      console.log('Liquidity pools:', pools.length);
      if (pools.length > 0) {
        const pool = pools[0];
        console.log('Pool ID:', pool.pool_id);
        console.log('Assets:', pool.asset1, '/', pool.asset2);
        console.log('Reserves:', pool.reserve1.toString(), '/', pool.reserve2.toString());
      }
    } catch (error: any) {
      console.log('No liquidity pools found or error:', error.message);
    }
    console.log();

    // Example 8: Condense asset (increase density)
    console.log('=== Example 8: Condense Asset ===');
    const condenseTx = MistbornAsset.createCondenseTransaction(
      assetId,
      keyPair.getAddress(),
      DensityLevel.Light,
      {
        texture: 'sword_texture.png',
        model: 'sword_model.gltf',
      },
      [
        { name: 'glow_effect', value: 'true', rarity: 0.9 },
      ]
    );
    const signedCondenseTx = await MistbornAsset.sign(condenseTx, keyPair);
    console.log('Condensed to:', signedCondenseTx.data.density);
    console.log('New metadata:', signedCondenseTx.data.metadata);
    console.log();

    console.log('All examples completed successfully!');
  } catch (error: any) {
    console.error('Error:', error.message);
    if (error.stack) {
      console.error('Stack:', error.stack);
    }
    process.exit(1);
  }
}

// Run examples
if (require.main === module) {
  main().catch(console.error);
}
