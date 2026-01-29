# Transaction API contract

All endpoints that accept or return transactions use the same canonical transaction format. Byte fields (addresses, hashes, signatures) are sent as **hex strings** (64 hex chars for 32 bytes). Numeric fields (`amount`, `fee`, `nonce`, `gas_limit`) can be numbers or decimal strings.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/transactions` | Submit any signed transaction |
| `POST` | `/api/v1/assets` | Create asset (signed `MistbornAsset` Create) |
| `POST` | `/api/v1/assets/:asset_id/condense` | Condense asset (signed `MistbornAsset` Condense) |
| `POST` | `/api/v1/assets/:asset_id/evaporate` | Evaporate asset (signed `MistbornAsset` Evaporate) |
| `POST` | `/api/v1/assets/:asset_id/merge` | Merge assets (signed `MistbornAsset` Merge) |
| `POST` | `/api/v1/assets/:asset_id/split` | Split asset (signed `MistbornAsset` Split) |
| `POST` | `/api/v1/assets/:asset_id/permissions` | Set permissions (signed `SetAssetPermissions`) |
| `POST` | `/api/v1/assets/estimate-gas` | Estimate gas for a Mistborn asset transaction |

Request body for transaction endpoints: `{ "transaction": <Transaction> }`. Response: `{ "success": true, "data": { "hash": "<hex>", "status": "pending" } }`.

## Transaction variants and fields

Every user-signed transaction includes:

- **`from`** (32 bytes, hex) – signer and fee payer
- **`fee`** (u64) – fee in base units
- **`nonce`** (u64) – account nonce (replay protection)
- **`signature`** (bytes, hex) – Ed25519 signature over the canonical signing payload

### Transfer

```json
{
  "Transfer": {
    "from": "<hex 32 bytes>",
    "to": "<hex 32 bytes>",
    "amount": "1000000000000000000",
    "fee": "1000000000000000",
    "nonce": 0,
    "signature": "<hex>"
  }
}
```

### ContractCall

```json
{
  "ContractCall": {
    "from": "<hex 32 bytes>",
    "contract": "<hex 32 bytes>",
    "method": "transfer",
    "args": "<hex>",
    "gas_limit": 100000,
    "fee": "0",
    "nonce": 0,
    "signature": "<hex>"
  }
}
```

### MistbornAsset

```json
{
  "MistbornAsset": {
    "from": "<hex 32 bytes>",
    "action": "Create",
    "asset_id": "<hex 32 bytes>",
    "data": {
      "density": "Ethereal",
      "metadata": {},
      "attributes": [],
      "game_id": null,
      "owner": "<hex 32 bytes>"
    },
    "fee": 0,
    "nonce": 0,
    "signature": "<hex>"
  }
}
```

`action`: `Create`, `Update`, `Condense`, `Evaporate`, `Merge`, `Split`.  
For **Merge**, `data.metadata._other_asset_id` must be the other asset ID (hex).  
For **Split**, `data.metadata._components` must be a comma-separated list of component IDs.

### Stake

```json
{
  "Stake": {
    "from": "<hex 32 bytes>",
    "validator": "<hex 32 bytes>",
    "amount": "1000000000",
    "fee": 0,
    "nonce": 0,
    "signature": "<hex>"
  }
}
```

### SetAssetPermissions

Used by `POST /api/v1/assets/:asset_id/permissions` with a separate request shape; the node builds the transaction from `SetPermissionsRequest` (see API code).

## Signing

The client must sign the **canonical payload** (bytes), not the JSON. The payload is built as in the node’s `get_transaction_data_for_signing` (see `src/consensus.rs`). The TypeScript SDK’s `encodeTransaction` and `signTransaction` produce the same payload; use the SDK to build and sign transactions so the signature matches the node’s verification.

## Example: build and sign (TypeScript SDK)

See [Building and signing a transaction](../sdk/README.md#building-and-signing-a-transaction) in the SDK README.
