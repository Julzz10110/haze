# Quick Test Guide

## Prerequisites

1. **Unity Project**
   - Unity 2020.3 LTS or later
   - HAZE SDK installed (see [INSTALLATION.md](INSTALLATION.md))
   - Chaos.NaCl.dll in `Assets/Plugins/`

2. **HAZE Node (Optional)**
   - For full testing, start a local node: `cargo run`
   - Default URL: `http://localhost:8080`

## Quick Test (No Node Required)

### Step 1: Add Test Script

1. Create empty GameObject in scene
2. Add `QuickTest` component (from `Tests/QuickTest.cs`)
3. Set `testWithoutNode = true` in inspector

### Step 2: Run Test

1. Press Play in Unity
2. Check Console for output:
   - `✓` = Test passed
   - `✗` = Test failed

**Expected Output:**
```
=== HAZE SDK Quick Test ===
[Test] Key Generation
✓ Generated address: abc123...
✓ Private key: def456...
✓ Key restoration works
[Skipped] Node connection test
[Test] Transaction Signing
✓ Transfer signed: 789abc...
✓ MistbornAsset signed: def012...
✓ Asset ID: 345678...
=== Quick Test Complete ===
```

## Full Test (With Node)

### Step 1: Start Node

```bash
cd /path/to/haze
cargo run
```

Verify node is running:
```bash
curl http://localhost:8080/health
```

### Step 2: Run Full Test

1. Add `HazeSDKTest` component to GameObject
2. Set `Node Url` to `http://localhost:8080`
3. Right-click component → "Run All Tests"

**Or use QuickTest:**
1. Set `testWithoutNode = false`
2. Press Play

## Test Individual Components

### Test Key Generation Only

```csharp
var keyPair = KeyPair.Generate();
Debug.Log($"Address: {keyPair.GetAddressHex()}");
```

### Test Signing Payload

1. Add `SigningPayloadTest` component
2. Right-click → "Test Transfer Payload"
3. Right-click → "Test MistbornAsset Payload"

### Test Node Connection Only

1. Add `HazeSDKTest` component
2. Right-click → "Test Node Connection Only"

## Troubleshooting

### "Chaos.NaCl not found"
- Place `Chaos.NaCl.dll` in `Assets/Plugins/`
- Check `.asmdef` references

### "Node connection failed"
- Verify node is running: `curl http://localhost:8080/health`
- Check firewall settings
- For WebGL: ensure CORS enabled

### "Transaction signing failed"
- Check KeyPair is generated correctly
- Verify transaction fields are valid
- Ensure signature is 64 bytes (128 hex chars)

## Next Steps

After tests pass:
- Test sending real transactions (requires funded account)
- Test Mistborn operations
- Create sample scenes
