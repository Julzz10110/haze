# Testing Summary

## What Has Been Created

### Test Scripts

1. **QuickTest.cs** - Simple test that runs on Start()
   - Tests key generation (no node needed)
   - Tests transaction signing (no node needed)
   - Optional node connection test
   - **Usage:** Add to GameObject, press Play

2. **HazeSDKTest.cs** - Comprehensive test suite
   - All 4 test categories
   - Context menu methods for individual tests
   - **Usage:** Right-click component → "Run All Tests"

3. **SigningPayloadTest.cs** - Payload verification tests
   - Tests canonical payload format
   - Verifies payload structure matches Rust/TypeScript
   - **Usage:** Right-click component → test methods

### Test Coverage

✅ **Key Generation**
- Generate new key pair
- Restore from private key
- Address format (32 bytes → 64 hex chars)

✅ **Transaction Signing**
- Transfer transaction payload
- MistbornAsset transaction payload
- Signature format (64 bytes → 128 hex chars)

✅ **Node Connection** (requires running node)
- Health check
- Blockchain info
- Account/balance queries

✅ **Transaction Submission** (requires running node + funded account)
- Send transfer
- Create asset
- Get transaction status

## How to Test

### Option 1: Quick Test (No Node)

1. Open Unity project
2. Create empty GameObject
3. Add `QuickTest` component
4. Set `testWithoutNode = true`
5. Press Play
6. Check Console for ✓/✗ results

### Option 2: Full Test (With Node)

1. Start HAZE node: `cargo run`
2. Verify: `curl http://localhost:8080/health`
3. In Unity: Add `HazeSDKTest` component
4. Set `Node Url` = `http://localhost:8080`
5. Right-click → "Run All Tests"
6. Check Console and Inspector flags

### Option 3: Payload Verification

1. Add `SigningPayloadTest` component
2. Right-click → "Run All Payload Tests"
3. Verify payload structure matches expectations

## Expected Results

### Quick Test (No Node)
```
✓ Key generation works
✓ Key restoration works  
✓ Transfer signing works
✓ MistbornAsset signing works
```

### Full Test (With Node)
```
✓ Key generation
✓ Node connection (health + blockchain info)
✓ Get balance (may be 0 for new address)
✓ Transaction signing
```

### Payload Test
```
✓ Transfer payload length = 95 bytes (without chain fields)
✓ Payload starts with "Transfer"
✓ MistbornAsset payload length >= 127 bytes
✓ Payload starts with "MistbornAsset"
✓ Signatures are 64 bytes (128 hex chars)
```

## Common Issues

### Issue: "Chaos.NaCl not found"
**Solution:** Place `Chaos.NaCl.dll` in `Assets/Plugins/`

### Issue: "Node connection failed"
**Solution:** 
- Verify node is running: `curl http://localhost:8080/health`
- Check node URL in test script
- For WebGL: ensure CORS enabled

### Issue: "Transaction signing failed"
**Solution:**
- Verify KeyPair is generated
- Check transaction fields are valid
- Ensure signature is 64 bytes

### Issue: "JSON serialization error"
**Solution:**
- Verify Newtonsoft.Json installed
- Check transaction structure
- Ensure Address/Hash serialized as hex

## Next Steps After Testing

Once basic tests pass:

1. **Test Real Transactions** (requires funded account)
   - Send transfer to another address
   - Create Mistborn asset
   - Verify transaction appears in pool

2. **Test Mistborn Operations**
   - Create, Update, Condense, Evaporate
   - Merge, Split
   - Search by owner/game_id

3. **Test Economy**
   - Get pools
   - Create pool
   - Calculate swap quotes

4. **Create Sample Scenes**
   - Mistborn asset creation scene
   - Economy demo scene
   - Full integration example

## Files Created

- `Tests/QuickTest.cs` - Quick test script
- `Tests/HazeSDKTest.cs` - Full test suite
- `Tests/SigningPayloadTest.cs` - Payload verification
- `TESTING.md` - Detailed testing guide
- `TEST_QUICK_START.md` - Quick start guide
- `TEST_SUMMARY.md` - This file
