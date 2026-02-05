# Testing Guide

## Prerequisites

1. **HAZE Node Running**
   - Start a local HAZE node: `cargo run` (or use existing node)
   - Default URL: `http://localhost:8080`
   - Verify node is running: `curl http://localhost:8080/health`

2. **Unity Project Setup**
   - Unity 2020.3 LTS or later
   - HAZE SDK installed (see [INSTALLATION.md](INSTALLATION.md))
   - Chaos.NaCl.dll in `Assets/Plugins/`
   - Newtonsoft.Json package installed

## Running Tests

### Option 1: Using Test Script (Recommended)

1. **Add Test Script to Scene**
   - Create empty GameObject in scene
   - Add `HazeSDKTest` component (from `Tests/HazeSDKTest.cs`)
   - Set `Node Url` in inspector (default: `http://localhost:8080`)

2. **Run Tests**
   - **Method A:** Right-click component → "Run All Tests"
   - **Method B:** Enter Play mode → tests run automatically (if configured)
   - **Method C:** Use individual test methods via context menu

3. **Check Results**
   - Open Console window (Window → General → Console)
   - Look for test output:
     - `✓` = Test passed
     - `✗` = Test failed
   - Inspector shows boolean flags for each test

### Option 2: Manual Testing

Create your own test script:

```csharp
using Haze;
using Haze.Crypto;
using UnityEngine;

public class MyTest : MonoBehaviour
{
    async void Start()
    {
        var client = new HazeClient("http://localhost:8080");
        var keyPair = KeyPair.Generate();
        Debug.Log($"Address: {keyPair.GetAddressHex()}");
        
        var balance = await client.GetBalanceAsync(keyPair.GetAddressHex());
        Debug.Log($"Balance: {balance}");
    }
}
```

## Test Coverage

### Test 1: Key Generation ✓
- Generate new key pair
- Restore from private key hex
- Verify addresses match

**Expected:** Address generated, restoration works, addresses match

### Test 2: Node Connection ✓
- Health check
- Get blockchain info
- Verify node responds

**Expected:** Health check returns "OK", blockchain info retrieved

### Test 3: Get Balance ✓
- Get balance for generated address
- Get account info (nonce, staked)

**Expected:** Balance retrieved (may be 0 for new address), account info retrieved

### Test 4: Transaction Signing ✓
- Create Transfer transaction
- Sign transaction
- Verify signature format
- Create and sign MistbornAsset transaction

**Expected:** Transactions created, signatures generated (128 hex chars), no errors

## Troubleshooting

### "Chaos.NaCl not found"
- Ensure `Chaos.NaCl.dll` is in `Assets/Plugins/`
- Check DLL is compatible with target platform
- Verify `.asmdef` references Chaos.NaCl

### "Node connection failed"
- Verify node is running: `curl http://localhost:8080/health`
- Check node URL in test script
- Check firewall/network settings
- For WebGL: ensure CORS is enabled on node

### "Transaction signing failed"
- Verify KeyPair is generated correctly
- Check that transaction fields are valid
- Ensure signature is 64 bytes (128 hex chars)

### "JSON serialization error"
- Verify Newtonsoft.Json is installed
- Check transaction structure matches API format
- Ensure Address/Hash are serialized as hex strings

## Integration Test with Real Node

To test against a real node with transactions:

1. **Fund Test Account**
   - Use faucet or transfer tokens to test address
   - Get address from test output

2. **Send Test Transfer**
   ```csharp
   var transfer = TransactionBuilder.CreateTransfer(...);
   var signed = TransactionBuilder.Sign(transfer, keyPair);
   var response = await client.SendTransactionAsync(signed);
   Debug.Log($"Tx hash: {response.hash}");
   ```

3. **Create Test Asset**
   ```csharp
   var assetId = TransactionBuilder.CreateAssetId("test-seed");
   var assetTx = TransactionBuilder.CreateAsset(...);
   var signed = TransactionBuilder.Sign(assetTx, keyPair);
   var response = await client.CreateAssetAsync(signed);
   ```

4. **Verify Transaction**
   ```csharp
   var tx = await client.GetTransactionAsync(response.hash);
   Debug.Log($"Status: {tx.status}");
   ```

## Next Steps

After basic tests pass:
- Test Mistborn operations (Create, Update, Condense, etc.)
- Test asset search
- Test economy pools
- Create sample scenes
