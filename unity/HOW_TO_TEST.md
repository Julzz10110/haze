# How to Test HAZE Unity SDK

## Prerequisites

1. **Unity Project Setup**
   - Unity 2020.3 LTS or later
   - HAZE SDK installed (see [INSTALLATION.md](INSTALLATION.md))
   - Chaos.NaCl.dll in `Assets/Plugins/`
   - Newtonsoft.Json package installed

2. **HAZE Node** (optional for basic tests)
   - For full testing: start node with `cargo run`
   - Default URL: `http://localhost:8080`

## Quick Test (5 minutes)

### Step 1: Add Test Script

1. Open Unity Editor
2. Create empty GameObject (right-click Hierarchy → Create Empty)
3. Rename to "HAZE Test"
4. Add Component → Search "QuickTest" → Add

### Step 2: Configure

1. Select the GameObject
2. In Inspector, find QuickTest component
3. Set `testWithoutNode = true` (for testing without node)
4. Set `nodeUrl = "http://localhost:8080"` (if testing with node)

### Step 3: Run Test

1. Press **Play** button
2. Open **Console** window (Window → General → Console)
3. Look for test output:
   - `✓` = Test passed
   - `✗` = Test failed

### Expected Output

```
=== HAZE SDK Quick Test ===

[Test] Key Generation
✓ Generated address: abc123def456...
✓ Private key: 789abc012def...
✓ Key restoration works

[Skipped] Node connection test (testWithoutNode = true)

[Test] Transaction Signing
✓ Transfer signed: 3456789abc...
✓ MistbornAsset signed: def0123456...
✓ Asset ID: 789abcdef012...

=== Quick Test Complete ===
```

## Full Test Suite (With Node)

### Step 1: Start Node

```bash
cd /path/to/haze
cargo run
```

Verify node is running:
```bash
curl http://localhost:8080/health
# Should return: "OK"
```

### Step 2: Run Full Tests

1. Create GameObject
2. Add `HazeSDKTest` component
3. Set `Node Url` = `http://localhost:8080`
4. Right-click component → **"Run All Tests"**

### Expected Results

All 4 tests should pass:
- ✅ Key Generation
- ✅ Node Connection  
- ✅ Get Balance
- ✅ Transaction Signing

## Individual Test Methods

### Test Key Generation Only

1. Add `HazeSDKTest` component
2. Right-click → **"Test Key Generation Only"**

### Test Node Connection Only

1. Add `HazeSDKTest` component  
2. Right-click → **"Test Node Connection Only"**

### Test Payload Format

1. Add `SigningPayloadTest` component
2. Right-click → **"Run All Payload Tests"**

## Troubleshooting

### ❌ "Chaos.NaCl not found"

**Problem:** Missing Ed25519 library

**Solution:**
1. Download `Chaos.NaCl.dll` from [GitHub](https://github.com/CodesInChaos/Chaos.NaCl/releases)
2. Place in `Assets/Plugins/` folder
3. Ensure DLL is compatible with your Unity target platform

### ❌ "Node connection failed"

**Problem:** Cannot connect to HAZE node

**Solutions:**
- Verify node is running: `curl http://localhost:8080/health`
- Check node URL in test script
- Check firewall/network settings
- For WebGL builds: ensure CORS is enabled on node

### ❌ "Transaction signing failed"

**Problem:** Signature generation error

**Solutions:**
- Verify KeyPair is generated correctly
- Check transaction fields are valid (non-negative amounts, valid addresses)
- Ensure signature is 64 bytes (128 hex characters)

### ❌ "JSON serialization error"

**Problem:** Transaction format incorrect

**Solutions:**
- Verify Newtonsoft.Json is installed (Package Manager)
- Check transaction structure matches API format
- Ensure Address/Hash are serialized as hex strings (64 chars)

## What Gets Tested

### ✅ Key Generation
- Generate new Ed25519 key pair
- Restore from private key hex
- Address format (32 bytes → 64 hex chars)

### ✅ Transaction Signing  
- Transfer transaction payload
- MistbornAsset transaction payload
- Signature format (64 bytes → 128 hex chars)
- Canonical payload matches Rust/TypeScript

### ✅ Node Connection (requires node)
- Health check endpoint
- Blockchain info endpoint
- Account/balance queries

### ✅ Transaction Submission (requires node + funded account)
- Send transfer transaction
- Create Mistborn asset
- Get transaction status

## Next Steps

After tests pass:

1. **Test Real Transactions**
   - Fund a test account (via faucet or transfer)
   - Send transfer to another address
   - Create Mistborn asset
   - Verify transactions appear in pool

2. **Test Mistborn Operations**
   - Create, Update, Condense, Evaporate
   - Merge, Split
   - Search by owner/game_id

3. **Test Economy**
   - Get liquidity pools
   - Create pool
   - Calculate swap quotes

4. **Create Sample Scenes**
   - Mistborn asset creation UI
   - Economy demo scene
   - Full integration example

## Files Reference

- `Tests/QuickTest.cs` - Simple test (runs on Start)
- `Tests/HazeSDKTest.cs` - Full test suite (context menu)
- `Tests/SigningPayloadTest.cs` - Payload verification
- `TESTING.md` - Detailed testing guide
- `TEST_QUICK_START.md` - Quick start guide
- `TEST_SUMMARY.md` - Test coverage summary

## Support

If tests fail:
1. Check Console for error messages
2. Verify prerequisites (Chaos.NaCl, Newtonsoft.Json)
3. Check node is running (if testing connection)
4. Review [TESTING.md](TESTING.md) for detailed troubleshooting
