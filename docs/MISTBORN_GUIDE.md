# Mistborn Assets — Quick Guide

Mistborn is HAZE’s dynamic NFT system with density levels (Ethereal → Light → Dense → Core) and operations: create, update, condense, evaporate, merge, split.

## Purpose

- **Assets** are tied to an **owner** (`owner`) and optionally to a game (`game_id`).
- **Density** defines the data size limit: Ethereal (5 KB), Light (50 KB), Dense (5 MB), Core (50 MB+).
- All asset operations are signed `MistbornAsset` transactions from `from` (same as `data.owner` for Create).

## Common Scenarios

### 1. Create asset (Create)

- Build a `MistbornAsset` transaction with `action: Create`, a unique `asset_id`, and `data` (density, metadata, attributes, owner, game_id).
- Sign with the owner’s key (`from` = owner).
- Submit: `POST /api/v1/assets` with body `{ "transaction": <signed tx> }` or `POST /api/v1/transactions` with the same transaction.

**SDK (TypeScript):**

```ts
import { MistbornAsset, KeyPair, HazeClient } from '@haze/sdk';

const keyPair = await KeyPair.generate();
const owner = keyPair.getAddress();
const assetId = MistbornAsset.createAssetId('my-unique-seed');
const tx = MistbornAsset.createCreateTransaction(
  assetId,
  owner,
  DensityLevel.Ethereal,
  { name: 'My NFT' },
  [],
  'game-1'
);
const signed = await MistbornAsset.sign(tx, keyPair);
const client = new HazeClient({ baseUrl: 'http://localhost:8080' });
await client.sendTransaction(signed);
```

### 2. Update metadata (Update)

- Use a `MistbornAsset` transaction with `action: Update`, same `asset_id`, updated `data.metadata` / `data.attributes`.
- Sign with the owner.

### 3. Increase density (Condense)

- Transaction with `action: Condense`, `asset_id`, and `data.density` set to the target level (e.g. Light or Dense).
- Sign with the owner. Submit: `POST /api/v1/assets/:asset_id/condense`.

### 4. Decrease density (Evaporate)

- Transaction with `action: Evaporate`, `data.density` set to the new level.
- Submit: `POST /api/v1/assets/:asset_id/evaporate`.

### 5. Merge two assets (Merge)

- In `data.metadata`, include `_other_asset_id` (hex of the second asset).
- Transaction with `action: Merge`, `asset_id` is the first asset. Submit: `POST /api/v1/assets/:asset_id/merge`.

### 6. Split asset (Split)

- In `data.metadata`, include `_components` — comma-separated list of component IDs.
- Transaction with `action: Split`. Submit: `POST /api/v1/assets/:asset_id/split`.

## Signing and format

- In all cases the **owner** (`from`) signs; signature is Ed25519 over the canonical payload (without the `signature` field).
- Payload format must match the node (Rust) and SDK: see `getTransactionDataForSigning` in the SDK and [API_TRANSACTIONS.md](API_TRANSACTIONS.md).
- Optionally set `chain_id` and `valid_until_height` for replay protection and chain binding.

## References

- [API_TRANSACTIONS.md](API_TRANSACTIONS.md) — API contract and transaction formats
- [OpenAPI](openapi.yaml) — REST API description
- [MISTBORN_NFT_PLAN.md](MISTBORN_NFT_PLAN.md) — Mistborn development plan
- [PERFORMANCE.md](PERFORMANCE.md) — Gas and limits, load tests, tuning; recommended limits and operator knobs for production
