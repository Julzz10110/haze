# Quick Test - Mistborn API

## Test Without Node (30 seconds)

1. **Add Test Script**
   - Create GameObject
   - Add `MistbornApiTest` component
   - Set `testWithoutNode = true`

2. **Run Test**
   - Right-click component → **"Run All Mistborn Tests"**
   - Or press Play (if auto-run configured)

3. **Check Results**
   - Console should show:
     - ✓ Density limits tests
     - ✓ Mistborn API helpers tests
     - [Skipped] Node-dependent tests

## Test With Node (2 minutes)

1. **Start Node**
   ```bash
   cargo run
   ```

2. **Configure Test**
   - Set `testWithoutNode = false`
   - Set `nodeUrl = "http://localhost:8080"`

3. **Run Test**
   - Right-click → **"Run All Mistborn Tests"**

4. **Expected Results**
   - ✓ Density limits
   - ✓ API helpers
   - ✓ Asset operations (if account has balance)
   - ✓ Search operations

## Test Simple Example

1. **Add Example Script**
   - Add `MistbornSimpleExample` component
   - Set `nodeUrl` (if using node)

2. **Press Play**
   - Example runs automatically
   - Creates asset, searches, updates

## What to Look For

### ✅ Success Indicators
- No errors in Console
- All tests show ✓
- Asset IDs are 64-character hex strings
- Signatures are 128-character hex strings
- Density limits match expected values

### ❌ Common Issues
- "Chaos.NaCl not found" → Install DLL
- "Node connection failed" → Check node is running
- "Transaction failed" → Check balance and nonce
- "Asset not found" → Wait after creation before searching

## Next Steps

- Try creating assets with different densities
- Test Condense/Evaporate operations
- Test Merge/Split operations
- Build custom UI using the API
