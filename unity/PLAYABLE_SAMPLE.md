# Playable sample

One playable sample demonstrates **address + balance + list (and optionally create) assets** without editing code after setup.

## Option A: Mistborn sample scene (full UI)

Shows: address, balance, list of assets by owner, create NFT button, asset detail.

### Prerequisites

- Unity project with HAZE SDK installed
- Chaos.NaCl and Newtonsoft.Json installed
- HAZE node running (e.g. `cargo run`), or use a remote API URL

### Steps

1. **Create a new scene** with UI:
   - Add Canvas (UI → Canvas)
   - Add Text for address (e.g. "Address: ...")
   - Add Text for balance (e.g. "Balance: 0")
   - Add InputField for asset name, Dropdown for density, InputField for game ID
   - Add Button "Create NFT", Button "Refresh"
   - Add ScrollView for assets list; add a child Text or prefab for each list item
   - Add Text for asset detail (selected asset info)

2. **Add the sample script:**
   - Create empty GameObject
   - Add component `MistbornSampleScene` (from `Samples~/Mistborn/MistbornSampleScene.cs`)

3. **Assign references** in the Inspector:
   - Node Url: `http://localhost:8080` (or your node)
   - Address Text, Balance Text
   - Asset Name Input, Density Dropdown, Game Id Input
   - Create Asset Button, Refresh Assets Button
   - Assets List (Scroll Rect), Asset Item Prefab (if used), Asset Detail Text

4. **Press Play.** You should see:
   - Generated address (shortened) and balance (from node)
   - List of assets owned by that address (empty if new key)
   - Use "Create NFT" to create one asset, then "Refresh" to see it in the list
   - Click an asset in the list to see details

Detailed UI wiring is in [Samples~/Mistborn/README.md](Samples~/Mistborn/README.md).

## Option B: Code-only (no UI)

- **BasicUsageExample** — attach to GameObject; press Play. Shows address, balance, blockchain info in Console. Set `nodeUrl` in Inspector.
- **MistbornSimpleExample** — attach to GameObject; press Play. Creates an asset, searches by owner, gets asset, updates metadata. Logs to Console. Set `nodeUrl`.

No scene UI needed; all output in Unity Console.
