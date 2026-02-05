using System;
using System.Threading.Tasks;
using Haze;
using Haze.Crypto;
using UnityEngine;

namespace Haze.Tests
{
    /// <summary>
    /// Test script for HAZE Unity SDK
    /// Run this in Unity to verify SDK functionality
    /// </summary>
    public class HazeSDKTest : MonoBehaviour
    {
        [Header("Node Configuration")]
        [SerializeField] private string nodeUrl = "http://localhost:8080";
        
        [Header("Test Results")]
        [SerializeField] private bool testKeyGeneration = false;
        [SerializeField] private bool testNodeConnection = false;
        [SerializeField] private bool testGetBalance = false;
        [SerializeField] private bool testTransactionSigning = false;
        
        private HazeClient _client;
        private KeyPair _testKeyPair;
        private string _testAddress;

        [ContextMenu("Run All Tests")]
        public async void RunAllTests()
        {
            Debug.Log("=== HAZE SDK Tests ===");
            
            try
            {
                _client = new HazeClient(nodeUrl);
                
                // Test 1: Key Generation
                await TestKeyGeneration();
                
                // Test 2: Node Connection
                await TestNodeConnection();
                
                // Test 3: Get Balance
                await TestGetBalance();
                
                // Test 4: Transaction Signing
                await TestTransactionSigning();
                
                Debug.Log("=== All Tests Completed ===");
            }
            catch (Exception ex)
            {
                Debug.LogError($"Test failed with error: {ex.Message}\n{ex.StackTrace}");
            }
            finally
            {
                _client?.Dispose();
            }
        }

        private async Task TestKeyGeneration()
        {
            Debug.Log("\n[Test 1] Key Generation");
            try
            {
                // Generate new key pair
                _testKeyPair = KeyPair.Generate();
                _testAddress = _testKeyPair.GetAddressHex();
                
                Debug.Log($"✓ Generated address: {_testAddress}");
                Debug.Log($"✓ Private key (hex): {_testKeyPair.GetPrivateKeyHex()}");
                
                // Test restore from private key
                var restored = KeyPair.FromPrivateKeyHex(_testKeyPair.GetPrivateKeyHex());
                var restoredAddress = restored.GetAddressHex();
                
                if (restoredAddress == _testAddress)
                {
                    Debug.Log($"✓ Key restoration works correctly");
                    testKeyGeneration = true;
                }
                else
                {
                    Debug.LogError($"✗ Key restoration failed: addresses don't match");
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Key generation test failed: {ex.Message}");
            }
        }

        private async Task TestNodeConnection()
        {
            Debug.Log("\n[Test 2] Node Connection");
            try
            {
                // Health check
                var health = await _client.HealthCheckAsync();
                Debug.Log($"✓ Health check: {health}");
                
                // Blockchain info
                var blockchainInfo = await _client.GetBlockchainInfoAsync();
                Debug.Log($"✓ Blockchain height: {blockchainInfo.current_height}");
                Debug.Log($"✓ Total supply: {blockchainInfo.total_supply}");
                Debug.Log($"✓ Current wave: {blockchainInfo.current_wave}");
                
                testNodeConnection = true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Node connection test failed: {ex.Message}");
                Debug.LogError($"  Make sure the HAZE node is running at {nodeUrl}");
            }
        }

        private async Task TestGetBalance()
        {
            Debug.Log("\n[Test 3] Get Balance");
            try
            {
                if (_testKeyPair == null)
                {
                    Debug.LogWarning("Skipping balance test - key pair not generated");
                    return;
                }
                
                var balance = await _client.GetBalanceAsync(_testAddress);
                Debug.Log($"✓ Balance for {_testAddress}: {balance}");
                
                var account = await _client.GetAccountAsync(_testAddress);
                Debug.Log($"✓ Account nonce: {account.nonce}");
                Debug.Log($"✓ Account staked: {account.staked}");
                
                testGetBalance = true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Get balance test failed: {ex.Message}");
            }
        }

        private async Task TestTransactionSigning()
        {
            Debug.Log("\n[Test 4] Transaction Signing");
            try
            {
                if (_testKeyPair == null)
                {
                    Debug.LogWarning("Skipping signing test - key pair not generated");
                    return;
                }
                
                // Create a test transfer transaction (won't send it)
                var recipient = new Address(new byte[32]); // Zero address for testing
                var transfer = TransactionBuilder.CreateTransfer(
                    from: _testKeyPair.GetAddress(),
                    to: recipient,
                    amount: 1000000,
                    fee: 1000,
                    nonce: 0
                );
                
                // Sign the transaction
                var signedTransfer = TransactionBuilder.Sign(transfer, _testKeyPair);
                
                Debug.Log($"✓ Transfer transaction created");
                Debug.Log($"✓ Transaction signed: {signedTransfer.signature.Substring(0, 16)}...");
                
                // Verify signature is not empty
                if (!string.IsNullOrEmpty(signedTransfer.signature) && signedTransfer.signature.Length == 128)
                {
                    Debug.Log($"✓ Signature length correct (128 hex chars = 64 bytes)");
                    testTransactionSigning = true;
                }
                else
                {
                    Debug.LogError($"✗ Invalid signature format");
                }
                
                // Test MistbornAsset Create signing
                var assetId = TransactionBuilder.CreateAssetId("test-asset-seed");
                var assetTx = TransactionBuilder.CreateAsset(
                    assetId: assetId,
                    owner: _testKeyPair.GetAddress(),
                    density: DensityLevel.Ethereal,
                    metadata: new System.Collections.Generic.Dictionary<string, string>
                    {
                        ["name"] = "Test Asset"
                    }
                );
                
                var signedAssetTx = TransactionBuilder.Sign(assetTx, _testKeyPair);
                Debug.Log($"✓ MistbornAsset transaction created and signed");
                Debug.Log($"✓ Asset ID: {Utils.BytesToHex(assetId.Bytes)}");
                
            }
            catch (Exception ex)
            {
                Debug.LogError($"✗ Transaction signing test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }

        [ContextMenu("Test Key Generation Only")]
        public void TestKeyGenOnly()
        {
            TestKeyGeneration().ContinueWith(t =>
            {
                if (t.IsFaulted)
                    Debug.LogError($"Key generation test failed: {t.Exception?.GetBaseException().Message}");
            });
        }

        [ContextMenu("Test Node Connection Only")]
        public async void TestConnectionOnly()
        {
            try
            {
                _client = new HazeClient(nodeUrl);
                await TestNodeConnection();
            }
            catch (Exception ex)
            {
                Debug.LogError($"Connection test failed: {ex.Message}");
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
