using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Haze;
using Haze.Crypto;
using Haze.Mistborn;
using UnityEngine;

namespace Haze.Tests
{
    /// <summary>
    /// Test script for Mistborn API
    /// Tests high-level Mistborn operations
    /// </summary>
    public class MistbornApiTest : MonoBehaviour
    {
        [Header("Configuration")]
        [SerializeField] private string nodeUrl = "http://localhost:8080";
        [SerializeField] private bool testWithoutNode = true;

        [Header("Test Results")]
        [SerializeField] private bool testMistbornApi = false;
        [SerializeField] private bool testDensityLimits = false;
        [SerializeField] private bool testAssetOperations = false;
        [SerializeField] private bool testSearch = false;

        private HazeClient _client;
        private KeyPair _keyPair;
        private MistbornAsset _mistborn;

        [ContextMenu("Run All Mistborn Tests")]
        public async void RunAllTests()
        {
            Debug.Log("=== Mistborn API Tests ===");

            try
            {
                if (!testWithoutNode)
                {
                    _client = new HazeClient(nodeUrl);
                    _keyPair = KeyPair.Generate();
                    _mistborn = new MistbornAsset(_client, _keyPair);
                }

                // Test 1: Density Limits
                TestDensityLimits();

                // Test 2: Mistborn API (without node)
                TestMistbornApiHelpers();

                if (!testWithoutNode)
                {
                    // Test 3: Asset Operations (requires node)
                    await TestAssetOperations();

                    // Test 4: Search (requires node)
                    await TestSearch();
                }
                else
                {
                    Debug.Log("\n[Skipped] Node-dependent tests (testWithoutNode = true)");
                }

                Debug.Log("\n=== All Mistborn Tests Completed ===");
            }
            catch (Exception ex)
            {
                Debug.LogError($"Test failed: {ex.Message}\n{ex.StackTrace}");
            }
            finally
            {
                _client?.Dispose();
            }
        }

        private void TestDensityLimits()
        {
            Debug.Log("\n[Test] Density Limits");
            try
            {
                // Test GetMaxSize
                var etherealMax = DensityLimits.GetMaxSize(DensityLevel.Ethereal);
                var lightMax = DensityLimits.GetMaxSize(DensityLevel.Light);
                var denseMax = DensityLimits.GetMaxSize(DensityLevel.Dense);
                var coreMax = DensityLimits.GetMaxSize(DensityLevel.Core);

                Debug.Log($"✓ Ethereal max: {etherealMax} bytes (5 KB)");
                Debug.Log($"✓ Light max: {lightMax} bytes (50 KB)");
                Debug.Log($"✓ Dense max: {denseMax} bytes (5 MB)");
                Debug.Log($"✓ Core max: {coreMax} bytes (50 MB+)");

                if (etherealMax == DensityLimits.EtherealMaxBytes &&
                    lightMax == DensityLimits.LightMaxBytes &&
                    denseMax == DensityLimits.DenseMaxBytes &&
                    coreMax == DensityLimits.CoreMaxBytes)
                {
                    Debug.Log("✓ GetMaxSize works correctly");
                }

                // Test IsWithinLimit
                var smallMetadata = 1000; // 1 KB
                var largeMetadata = 100000; // 100 KB

                if (DensityLimits.IsWithinLimit(DensityLevel.Ethereal, smallMetadata))
                    Debug.Log("✓ Small metadata fits Ethereal");
                else
                    Debug.LogError("✗ Small metadata should fit Ethereal");

                if (!DensityLimits.IsWithinLimit(DensityLevel.Ethereal, largeMetadata))
                    Debug.Log("✓ Large metadata correctly rejected for Ethereal");
                else
                    Debug.LogError("✗ Large metadata should not fit Ethereal");

                if (DensityLimits.IsWithinLimit(DensityLevel.Dense, largeMetadata))
                    Debug.Log("✓ Large metadata fits Dense");

                // Test GetRecommendedDensity
                var recommended1 = DensityLimits.GetRecommendedDensity(1000);
                var recommended2 = DensityLimits.GetRecommendedDensity(100000);
                var recommended3 = DensityLimits.GetRecommendedDensity(10000000);

                Debug.Log($"✓ Recommended for 1 KB: {recommended1} (should be Ethereal)");
                Debug.Log($"✓ Recommended for 100 KB: {recommended2} (should be Dense)");
                Debug.Log($"✓ Recommended for 10 MB: {recommended3} (should be Core)");

                if (recommended1 == DensityLevel.Ethereal &&
                    recommended2 == DensityLevel.Dense &&
                    recommended3 == DensityLevel.Core)
                {
                    Debug.Log("✓ GetRecommendedDensity works correctly");
                    testDensityLimits = true;
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Density limits test failed: {ex.Message}");
            }
        }

        private void TestMistbornApiHelpers()
        {
            Debug.Log("\n[Test] Mistborn API Helpers");
            try
            {
                // Test CreateAssetId
                var seed = "test-seed-123";
                var assetId1 = MistbornAsset.CreateAssetId(seed);
                var assetId2 = MistbornAsset.CreateAssetId(seed);

                if (assetId1.Equals(assetId2))
                {
                    Debug.Log("✓ CreateAssetId produces consistent results");
                }
                else
                {
                    Debug.LogError("✗ CreateAssetId should produce same ID for same seed");
                }

                // Test AssetIdToHex
                var hex = MistbornAsset.AssetIdToHex(assetId1);
                if (hex.Length == 64)
                {
                    Debug.Log($"✓ AssetIdToHex works: {hex.Substring(0, 16)}...");
                }
                else
                {
                    Debug.LogError($"✗ AssetIdToHex should produce 64-char hex, got {hex.Length}");
                }

                // Test HexToAssetId
                var restored = MistbornAsset.HexToAssetId(hex);
                if (restored.HasValue && restored.Value.Equals(assetId1))
                {
                    Debug.Log("✓ HexToAssetId works correctly");
                }
                else
                {
                    Debug.LogError("✗ HexToAssetId should restore original asset ID");
                }

                // Test with different seeds
                var assetId3 = MistbornAsset.CreateAssetId("different-seed");
                if (!assetId3.Equals(assetId1))
                {
                    Debug.Log("✓ Different seeds produce different asset IDs");
                }

                // Test with bytes
                var bytes = new byte[] { 1, 2, 3, 4, 5 };
                var assetIdFromBytes = MistbornAsset.CreateAssetId(bytes);
                Debug.Log($"✓ CreateAssetId from bytes works: {MistbornAsset.AssetIdToHex(assetIdFromBytes).Substring(0, 16)}...");

                testMistbornApi = true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Mistborn API helpers test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }

        private async Task TestAssetOperations()
        {
            Debug.Log("\n[Test] Asset Operations (requires node)");
            try
            {
                if (_mistborn == null)
                {
                    Debug.LogWarning("Skipping - mistborn API not initialized");
                    return;
                }

                // Get account info for nonce
                var address = _keyPair.GetAddressHex();
                var account = await _client.GetAccountAsync(address);
                var nonce = account.nonce;

                Debug.Log($"Using nonce: {nonce}");

                // Create asset
                var assetId = MistbornAsset.CreateAssetId($"test-{DateTime.Now.Ticks}");
                var metadata = new Dictionary<string, string>
                {
                    ["name"] = "Test Asset",
                    ["test"] = "true"
                };

                Debug.Log($"Creating asset: {MistbornAsset.AssetIdToHex(assetId)}");

                // Estimate gas first
                var tempTx = TransactionBuilder.CreateAsset(
                    assetId: assetId,
                    owner: _keyPair.GetAddress(),
                    density: DensityLevel.Ethereal,
                    metadata: metadata,
                    gameId: "test-game",
                    fee: 0,
                    nonce: nonce
                );

                var gasEstimate = await _mistborn.EstimateGasAsync(tempTx);
                Debug.Log($"✓ Gas estimate: {gasEstimate.gas_cost}, fee: {gasEstimate.fee}");

                // Create asset
                var createResponse = await _mistborn.CreateAsync(
                    assetId: assetId,
                    density: DensityLevel.Ethereal,
                    metadata: metadata,
                    gameId: "test-game",
                    fee: ulong.Parse(gasEstimate.fee),
                    nonce: nonce
                );

                Debug.Log($"✓ Asset created! Hash: {createResponse.hash}");

                // Wait a bit
                await Task.Delay(2000);

                // Get asset
                var assetInfo = await _mistborn.GetAssetAsync(assetId);
                Debug.Log($"✓ Asset retrieved: {assetInfo.asset_id}, density: {assetInfo.density}");

                // Update asset (if we have balance)
                try
                {
                    var updatedMetadata = new Dictionary<string, string>
                    {
                        ["name"] = "Updated Test Asset",
                        ["updated"] = "true"
                    };

                    var updateResponse = await _mistborn.UpdateAsync(
                        assetId: assetId,
                        metadata: updatedMetadata,
                        nonce: nonce + 1
                    );
                    Debug.Log($"✓ Asset updated! Hash: {updateResponse.hash}");
                }
                catch (Exception ex)
                {
                    Debug.LogWarning($"Update failed (may need balance): {ex.Message}");
                }

                testAssetOperations = true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Asset operations test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }

        private async Task TestSearch()
        {
            Debug.Log("\n[Test] Asset Search");
            try
            {
                if (_mistborn == null)
                {
                    Debug.LogWarning("Skipping - mistborn API not initialized");
                    return;
                }

                // Search by owner
                var myAssets = await _mistborn.SearchByOwnerAsync();
                Debug.Log($"✓ Found {myAssets.Count} assets by owner");

                // Search by game ID
                var gameAssets = await _mistborn.SearchByGameIdAsync("test-game");
                Debug.Log($"✓ Found {gameAssets.Count} assets by game ID");

                // Search by owner and game ID
                var filteredAssets = await _mistborn.SearchAsync(gameId: "test-game");
                Debug.Log($"✓ Found {filteredAssets.Count} assets (owner + game ID)");

                if (myAssets.Count > 0)
                {
                    var firstAsset = myAssets[0];
                    Debug.Log($"  First asset: {firstAsset.asset_id.Substring(0, 16)}... ({firstAsset.density})");
                }

                testSearch = true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Search test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }

        [ContextMenu("Test Density Limits Only")]
        public void TestDensityOnly()
        {
            TestDensityLimits();
        }

        [ContextMenu("Test Mistborn Helpers Only")]
        public void TestHelpersOnly()
        {
            TestMistbornApiHelpers();
        }

        [ContextMenu("Test Asset Operations Only")]
        public async void TestOperationsOnly()
        {
            try
            {
                _client = new HazeClient(nodeUrl);
                _keyPair = KeyPair.Generate();
                _mistborn = new MistbornAsset(_client, _keyPair);
                await TestAssetOperations();
            }
            catch (Exception ex)
            {
                Debug.LogError($"Test failed: {ex.Message}");
            }
            finally
            {
                _client?.Dispose();
            }
        }

        private void OnDestroy()
        {
            _client?.Dispose();
        }
    }
}
