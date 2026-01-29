# Mistborn NFT Full Implementation Plan

Some items in this plan are already implemented (indexes by owner/game_id/density, full-text search, versions, API). For current status see [TESTING_REPORT.md](TESTING_REPORT.md).

## Current State

### Already implemented

1. **Base structure:**
   - `MistbornAsset` with density levels (Ethereal, Light, Dense, Core)
   - `AssetData` with metadata, attributes, game_id, owner
   - `BlobStorage` for large files (Core density)
   - Change history (`AssetHistoryEntry`)

2. **Core operations:**
   - `create()` - create asset
   - `condense()` - increase density
   - `evaporate()` - decrease density
   - `merge()` - merge assets
   - `split()` - split into components
   - `update()` - update metadata

3. **Integration:**
   - WASM contracts (`condense_via_wasm`, `evaporate_via_wasm`)
   - REST API endpoints
   - TypeScript SDK
   - Transaction handling in `StateManager`

### Issues and gaps

1. **Blob references not persisted in state:**
   - `blob_refs` in `MistbornAsset` are not synced with `AssetState` in `StateManager`
   - Blob references are lost when loading asset from state

2. **History not persisted:**
   - `history` in `MistbornAsset` is not stored in `AssetState`
   - History is lost on node restart

3. **Incomplete validation:**
   - No data size checks on operations
   - No access control checks (beyond owner)
   - No metadata format validation

4. **Attributes underused:**
   - `attributes` not fully handled in merge/split operations
   - No dedicated attribute operations

5. **Search and filtering:**
   - No indexing by game_id, owner, density
   - No full-text search on metadata

6. **Economics:**
   - No operation costs (gas fees)
   - No operation limits

7. **Versioning:**
   - No asset version support
   - No rollback capability

---

## Implementation Plan

### Stage 1: Critical fixes

#### 1.1. BlobStorage integration with StateManager

**Tasks:**
- Add `blob_refs: HashMap<String, Hash>` to `AssetState`
- Update `StateManager::apply_block()` to persist blob_refs
- Update `StateManager::get_asset()` to load blob_refs
- Add blob storage helpers in `StateManager`

**Files to change:**
- `src/state.rs` - add blob_refs to AssetState
- `src/assets.rs` - sync blob_refs between MistbornAsset and AssetState

**API changes:**
- `GET /api/v1/assets/:asset_id` - return blob_refs
- `GET /api/v1/assets/:asset_id/blob/:blob_key` - get blob data

#### 1.2. Persist change history

**Tasks:**
- Add `history: Vec<AssetHistoryEntry>` to `AssetState`
- Persist history on each operation
- Cap history size (e.g. last 100 entries)
- Add method to get asset history

**Files to change:**
- `src/state.rs` - add history to AssetState
- `src/assets.rs` - update all operations to write history

**API changes:**
- `GET /api/v1/assets/:asset_id/history` - get change history
- `GET /api/v1/assets/:asset_id/history/:limit` - get last N entries

#### 1.3. Stronger operation validation

**Tasks:**
- Add data size checks before operations
- Metadata format validation (optional JSON schema)
- Access checks (owner, game_id permissions)
- Density transition validation (allowed A → B)

**Files to change:**
- `src/assets.rs` - add validation helpers
- `src/state.rs` - add validation in apply_block

**New error types:**
- `AssetSizeExceeded`
- `InvalidMetadataFormat`
- `InvalidDensityTransition`
- `AccessDenied`

---

### Stage 2: Extended functionality

#### 2.1. Attribute operations

**Tasks:**
- Add attribute management operations:
  - `add_attribute(name, value, rarity)`
  - `update_attribute(name, value)`
  - `remove_attribute(name)`
  - `get_attribute(name)`
- On merge, combine attributes with conflict resolution
- On split, distribute attributes across components
- Support derived attributes

**Files to change:**
- `src/assets.rs` - add attribute methods
- `src/types.rs` - refine Attribute structure

**API changes:**
- `POST /api/v1/assets/:asset_id/attributes` - add/update attribute
- `DELETE /api/v1/assets/:asset_id/attributes/:name` - remove attribute
- `GET /api/v1/assets/:asset_id/attributes` - get all attributes

#### 2.2. Asset search and filtering

**Tasks:**
- Add indexes for fast lookup:
  - By owner (HashMap<Address, Vec<Hash>>)
  - By game_id (HashMap<String, Vec<Hash>>)
  - By density (HashMap<DensityLevel, Vec<Hash>>)
- Full-text search on metadata
- Filter by attributes (e.g. rarity > 0.9)
- Sort results (by created_at, updated_at, rarity)

**Files to change:**
- `src/state.rs` - add indexes
- `src/api.rs` - add search endpoint

**API changes:**
- `GET /api/v1/assets/search?owner=...&game_id=...&density=...&q=...`
- `GET /api/v1/assets/by-owner/:address`
- `GET /api/v1/assets/by-game/:game_id`

#### 2.3. Asset versioning

**Tasks:**
- Add asset versioning (snapshots)
- Create snapshot on significant changes (condense, merge)
- Ability to fetch asset by version
- Cap number of versions (e.g. last 10)

**Files to change:**
- `src/state.rs` - add version storage
- `src/assets.rs` - add snapshot methods

**API changes:**
- `GET /api/v1/assets/:asset_id/versions` - list versions
- `GET /api/v1/assets/:asset_id/versions/:version` - get version
- `POST /api/v1/assets/:asset_id/snapshot` - create snapshot manually

---

### Stage 3: Economics and optimization

#### 3.1. Operation costs

**Tasks:**
- Define gas costs per operation:
  - Create: base gas
  - Condense: depends on new density level
  - Evaporate: minimal gas (archival)
  - Merge: sum of asset sizes
  - Split: number of components
- Integrate with Tokenomics for gas deduction
- Support custom gas price

**Files to change:**
- `src/assets.rs` - add gas cost calculation
- `src/state.rs` - deduct gas on operations
- `src/config.rs` - add gas cost settings

**API changes:**
- `POST /api/v1/assets/estimate-gas` - estimate operation cost

#### 3.2. Limits and quotas

**Tasks:**
- Limit assets per account
- Limit metadata size
- Limit number of blob files
- Quotas per node type (core/edge/light)

**Files to change:**
- `src/config.rs` - add limit settings
- `src/state.rs` - enforce limits before operations

#### 3.3. Performance optimization

**Tasks:**
- Cache frequently requested assets
- Lazy-load blob data
- Batch operations (batch create/update)
- Index optimization

**Files to change:**
- `src/state.rs` - add caching
- `src/assets.rs` - optimize operations

---

### Stage 4: Additional features

#### 4.1. Access control and permissions

**Tasks:**
- Permission model for assets:
  - Owner (full access)
  - Game contract (restricted by game_id)
  - Public read (read-only)
- Permission delegation
- Time-limited permissions

**Files to change:**
- `src/types.rs` - add Permission enum
- `src/assets.rs` - permission checks

**API changes:**
- `POST /api/v1/assets/:asset_id/permissions` - set permissions
- `GET /api/v1/assets/:asset_id/permissions` - get permissions

#### 4.2. Events and notifications

**Tasks:**
- Extend WebSocket events:
  - AssetAttributeUpdated
  - AssetVersionCreated
  - AssetPermissionChanged
- Subscribe with filters
- Event history per asset

**Files to change:**
- `src/ws_events.rs` - add new events
- `src/state.rs` - emit events

#### 4.3. Export and import

**Tasks:**
- Export asset to JSON
- Import asset from JSON
- Export blob data
- Validation on import

**Files to change:**
- `src/assets.rs` - export/import methods
- `src/api.rs` - export/import endpoints

**API changes:**
- `GET /api/v1/assets/:asset_id/export` - export asset
- `POST /api/v1/assets/import` - import asset

---

### Stage 5: Testing and documentation

#### 5.1. Unit tests

**Tasks:**
- Tests for all operations (create, condense, evaporate, merge, split)
- Blob storage tests
- Validation tests
- Index and search tests

**Files:**
- `src/assets.rs` - extend existing tests
- `src/state.rs` - add state operation tests

#### 5.2. Integration tests

**Tasks:**
- E2E tests via API
- Multi-node asset sync tests
- Load tests (many assets)

**Files:**
- `tests/integration/` - new integration tests

#### 5.3. Documentation

**Tasks:**
- API docs (OpenAPI/Swagger)
- Usage examples for all operations
- Best practices guide
- Data format description

**Files:**
- `docs/API.md` - API reference
- `docs/MISTBORN_GUIDE.md` - user guide
- `examples/` - code examples

---

## Implementation priorities

### High priority (Stage 1)
1. BlobStorage integration with StateManager
2. Persist change history
3. Stronger operation validation

### Medium priority (Stage 2)
4. Attribute operations
5. Asset search and filtering
6. Asset versioning

### Low priority (Stages 3–5)
7. Economics and optimization
8. Additional features
9. Testing and documentation

## Dependencies

- `serde` - serialization
- `sled` - database
- `dashmap` - concurrent HashMap
- `hex` - hex encoding
- `chrono` - timestamps
- `wasmtime` - WASM VM (already in use)

---

## Risks and mitigation

1. **Performance with many assets:**
   - Mitigation: indexes, caching, pagination

2. **Blob storage size:**
   - Mitigation: compression, deduplication, archiving old blobs

3. **Sync across nodes:**
   - Mitigation: integrity checks, blob storage replication

4. **Security:**
   - Mitigation: validate all inputs, enforce access control
