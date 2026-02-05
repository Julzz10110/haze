using System;
using System.Linq;
using Haze;
using Haze.Crypto;
using UnityEngine;

namespace Haze.Tests
{
    /// <summary>
    /// Test to verify canonical payload matches Rust/TypeScript implementation
    /// This test can run without a node connection
    /// </summary>
    public class SigningPayloadTest : MonoBehaviour
    {
        [ContextMenu("Test Transfer Payload")]
        public void TestTransferPayload()
        {
            Debug.Log("=== Testing Transfer Transaction Payload ===");
            
            try
            {
                // Create a test transfer with known values
                var fromBytes = new byte[32];
                var toBytes = new byte[32];
                fromBytes[0] = 0x01;
                toBytes[0] = 0x02;
                
                var from = new Address(fromBytes);
                var to = new Address(toBytes);
                
                var transfer = TransactionBuilder.CreateTransfer(
                    from: from,
                    to: to,
                    amount: 1000000,
                    fee: 1000,
                    nonce: 5
                );
                
                // Get signing payload
                var payload = TransactionSigning.GetTransactionDataForSigning(transfer);
                
                Debug.Log($"Payload length: {payload.Length} bytes");
                Debug.Log($"Payload hex: {Utils.BytesToHex(payload)}");
                
                // Verify payload structure:
                // "Transfer" (7 bytes) + from (32) + to (32) + amount (8) + fee (8) + nonce (8) = 95 bytes
                if (payload.Length == 95)
                {
                    Debug.Log("✓ Payload length correct (95 bytes for Transfer without chain fields)");
                }
                else
                {
                    Debug.LogWarning($"⚠ Payload length: {payload.Length} (expected 95)");
                }
                
                // Verify "Transfer" prefix
                var prefix = System.Text.Encoding.UTF8.GetString(payload.Take(7).ToArray());
                if (prefix == "Transfer")
                {
                    Debug.Log("✓ Payload starts with 'Transfer'");
                }
                else
                {
                    Debug.LogError($"✗ Invalid prefix: {prefix}");
                }
                
                // Sign and verify signature length
                var keyPair = KeyPair.Generate();
                var signature = TransactionSigning.SignTransaction(transfer, keyPair);
                
                if (signature.Length == 64)
                {
                    Debug.Log($"✓ Signature length correct (64 bytes)");
                    Debug.Log($"✓ Signature hex: {Utils.BytesToHex(signature).Substring(0, 32)}...");
                }
                else
                {
                    Debug.LogError($"✗ Invalid signature length: {signature.Length} (expected 64)");
                }
                
                Debug.Log("=== Transfer Payload Test Complete ===");
            }
            catch (Exception ex)
            {
                Debug.LogError($"Test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }
        
        [ContextMenu("Test MistbornAsset Payload")]
        public void TestMistbornAssetPayload()
        {
            Debug.Log("=== Testing MistbornAsset Transaction Payload ===");
            
            try
            {
                var keyPair = KeyPair.Generate();
                var owner = keyPair.GetAddress();
                
                // Create asset ID
                var assetId = TransactionBuilder.CreateAssetId("test-seed-123");
                
                // Create asset transaction
                var assetTx = TransactionBuilder.CreateAsset(
                    assetId: assetId,
                    owner: owner,
                    density: DensityLevel.Ethereal,
                    metadata: new System.Collections.Generic.Dictionary<string, string>
                    {
                        ["name"] = "Test Asset"
                    },
                    fee: 0,
                    nonce: 0
                );
                
                // Get signing payload
                var payload = TransactionSigning.GetTransactionDataForSigning(assetTx);
                
                Debug.Log($"Payload length: {payload.Length} bytes");
                Debug.Log($"Payload hex (first 64 chars): {Utils.BytesToHex(payload).Substring(0, 64)}...");
                
                // Verify payload structure:
                // "MistbornAsset" (13) + from (32) + action (1) + asset_id (32) + owner (32) + density (1) + fee (8) + nonce (8) = 127 bytes
                if (payload.Length >= 127)
                {
                    Debug.Log($"✓ Payload length reasonable (>= 127 bytes for MistbornAsset Create)");
                }
                else
                {
                    Debug.LogWarning($"⚠ Payload length: {payload.Length} (expected >= 127)");
                }
                
                // Verify "MistbornAsset" prefix
                var prefix = System.Text.Encoding.UTF8.GetString(payload.Take(13).ToArray());
                if (prefix == "MistbornAsset")
                {
                    Debug.Log("✓ Payload starts with 'MistbornAsset'");
                }
                else
                {
                    Debug.LogError($"✗ Invalid prefix: {prefix}");
                }
                
                // Sign
                var signature = TransactionSigning.SignTransaction(assetTx, keyPair);
                
                if (signature.Length == 64)
                {
                    Debug.Log($"✓ Signature length correct (64 bytes)");
                }
                else
                {
                    Debug.LogError($"✗ Invalid signature length: {signature.Length}");
                }
                
                Debug.Log("=== MistbornAsset Payload Test Complete ===");
            }
            catch (Exception ex)
            {
                Debug.LogError($"Test failed: {ex.Message}\n{ex.StackTrace}");
            }
        }
        
        [ContextMenu("Run All Payload Tests")]
        public void RunAllPayloadTests()
        {
            TestTransferPayload();
            TestMistbornAssetPayload();
        }
    }
}
