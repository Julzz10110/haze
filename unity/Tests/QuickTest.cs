using System;
using System.Threading.Tasks;
using Haze;
using Haze.Crypto;
using UnityEngine;

namespace Haze.Tests
{
    /// <summary>
    /// Quick test script - add to GameObject and press Play
    /// Tests basic functionality without requiring node connection
    /// </summary>
    public class QuickTest : MonoBehaviour
    {
        [Header("Configuration")]
        [SerializeField] private string nodeUrl = "http://localhost:8080";
        [SerializeField] private bool testWithoutNode = true;

        async void Start()
        {
            Debug.Log("=== HAZE SDK Quick Test ===");
            
            // Test 1: Key Generation (no node needed)
            TestKeyGeneration();
            
            if (!testWithoutNode)
            {
                // Test 2: Node Connection (requires running node)
                await TestNodeConnection();
            }
            else
            {
                Debug.Log("\n[Skipped] Node connection test (testWithoutNode = true)");
                Debug.Log("To test node connection, set testWithoutNode = false and ensure node is running");
            }
            
            // Test 3: Transaction Signing (no node needed)
            TestTransactionSigning();
            
            Debug.Log("\n=== Quick Test Complete ===");
        }

        private void TestKeyGeneration()
        {
            Debug.Log("\n[Test] Key Generation");
            try
            {
                var keyPair = KeyPair.Generate();
                var address = keyPair.GetAddressHex();
                var privateKey = keyPair.GetPrivateKeyHex();
                
                Debug.Log($"✓ Generated address: {address}");
                Debug.Log($"✓ Private key: {privateKey.Substring(0, 16)}...");
                
                // Test restoration
                var restored = KeyPair.FromPrivateKeyHex(privateKey);
                if (restored.GetAddressHex() == address)
                {
                    Debug.Log("✓ Key restoration works");
                }
                else
                {
                    Debug.LogError("✗ Key restoration failed");
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Key generation failed: {ex.Message}");
            }
        }

        private async Task TestNodeConnection()
        {
            Debug.Log("\n[Test] Node Connection");
            try
            {
                var client = new HazeClient(nodeUrl);
                
                var health = await client.HealthCheckAsync();
                Debug.Log($"✓ Health: {health}");
                
                var info = await client.GetBlockchainInfoAsync();
                Debug.Log($"✓ Height: {info.current_height}");
                
                client.Dispose();
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Node connection failed: {ex.Message}");
                Debug.LogError($"  Make sure node is running at {nodeUrl}");
            }
        }

        private void TestTransactionSigning()
        {
            Debug.Log("\n[Test] Transaction Signing");
            try
            {
                var keyPair = KeyPair.Generate();
                
                // Test Transfer
                var to = new Address(new byte[32]);
                var transfer = TransactionBuilder.CreateTransfer(
                    from: keyPair.GetAddress(),
                    to: to,
                    amount: 1000000,
                    fee: 1000,
                    nonce: 0
                );
                
                var signed = TransactionBuilder.Sign(transfer, keyPair);
                Debug.Log($"✓ Transfer signed: {signed.signature.Substring(0, 16)}...");
                
                // Test MistbornAsset
                var assetId = TransactionBuilder.CreateAssetId("test-seed");
                var assetTx = TransactionBuilder.CreateAsset(
                    assetId: assetId,
                    owner: keyPair.GetAddress(),
                    density: DensityLevel.Ethereal,
                    metadata: new System.Collections.Generic.Dictionary<string, string>
                    {
                        ["name"] = "Test"
                    }
                );
                
                var signedAsset = TransactionBuilder.Sign(assetTx, keyPair);
                Debug.Log($"✓ MistbornAsset signed: {signedAsset.signature.Substring(0, 16)}...");
                Debug.Log($"✓ Asset ID: {Utils.BytesToHex(assetId.Bytes).Substring(0, 16)}...");
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Signing failed: {ex.Message}\n{ex.StackTrace}");
            }
        }
    }
}
