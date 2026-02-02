# WASM Contracts

This document describes how to deploy and call WASM contracts on HAZE: transaction types, gas, limits, and developer workflow.

## Overview

- **DeployContract** — deploys WASM bytecode; contract address = `sha256(code)` (first 32 bytes). Code is stored in state and can be called by address.
- **ContractCall** — executes a deployed contract: `contract` (address), `method` (export name), `args` (bytes). Gas is metered and deducted from `from`.

## Transaction types

### DeployContract

| Field | Type | Description |
|-------|------|-------------|
| `from` | 32 bytes (hex) | Signer and fee payer |
| `code` | bytes (hex) | WASM bytecode |
| `fee` | u64 | Fee (same as other tx types) |
| `nonce` | u64 | Account nonce |
| `chain_id` | u64 (optional) | Chain ID |
| `valid_until_height` | u64 (optional) | Replay protection |
| `signature` | bytes (hex) | Ed25519 signature of canonical payload |

**Canonical payload for signing:** `"DeployContract"` + `from` + `len(code)` (u32 le) + `code` + `fee` (u64 le) + `nonce` (u64 le) + optional `chain_id`, `valid_until_height`.

**Limits:** Contract code size ≤ 2 MiB (enforced in consensus).

**Effect:** Contract is stored at address `sha256(code)`. Later ContractCall uses this address to load code.

### ContractCall

| Field | Type | Description |
|-------|------|-------------|
| `from` | 32 bytes (hex) | Signer and fee/gas payer |
| `contract` | 32 bytes (hex) | Contract address (from DeployContract) |
| `method` | string | Exported function name |
| `args` | bytes (hex) | Arguments (opaque to node) |
| `gas_limit` | u64 | Max gas for this call |
| `fee` | u64 | Transaction fee |
| `nonce` | u64 | Account nonce |
| `chain_id` | u64 (optional) | Chain ID |
| `valid_until_height` | u64 (optional) | Replay protection |
| `signature` | bytes (hex) | Ed25519 signature of canonical payload |

**Limits:** `gas_limit` must be > 0 and ≤ node config `vm.gas_limit` (default 10_000_000).

**Effect:** Node loads WASM code at `contract`, runs `method` with `args`, meters gas, deducts `fee` and `gas_used * gas_price` from `from`. Gas fee is processed (50% burned) like other fees.

## Gas metering

- **VM:** Compilation and instantiation consume gas (fixed costs); execution consumes gas (estimated or fuel-based in wasmtime).
- **Config:** `vm.gas_limit` (per-tx cap), `vm.gas_price` (fee = gas_used × gas_price).
- **Context:** `ExecutionContext` has `gas_limit` and `gas_used`; VM updates `gas_used` during execution. Caller is charged `gas_used * gas_price` after the call.

See [PERFORMANCE.md](PERFORMANCE.md) for production gas and limits.

## API

- **POST /api/v1/transactions** — submit a signed transaction. Body must include one of `Transfer`, `DeployContract`, `ContractCall`, `MistbornAsset`, `Stake`, `SetAssetPermissions` with required fields and `signature`.
- **DeployContract example (JSON):** `{ "DeployContract": { "from": "<hex>", "code": "<hex>", "fee": 0, "nonce": 0, "signature": "<hex>" } }`
- **ContractCall example (JSON):** `{ "ContractCall": { "from": "<hex>", "contract": "<hex>", "method": "execute", "args": "<hex>", "gas_limit": 10000, "fee": 0, "nonce": 0, "signature": "<hex>" } }`

Contract address for a deployed contract is `sha256(wasm_code)` (32 bytes); use the same when building ContractCall.

## SDK

The TypeScript SDK should align with the node: same canonical payload for signing (see node `get_transaction_data_for_signing` and [API_TRANSACTIONS.md](API_TRANSACTIONS.md)). Add helpers for DeployContract and ContractCall (build payload, sign, submit) as needed.

## Asset-related contracts (condense / evaporate)

Mistborn asset operations can be driven by WASM via `condense_via_wasm` and `evaporate_via_wasm` in the assets layer; those use `HazeVM::execute_contract` with a passed-in `ExecutionContext`. For on-chain ContractCall, the flow is: user submits ContractCall tx → consensus validates → state applies tx (loads code at `contract`, runs VM, deducts gas). Contract code for asset flows can be deployed with DeployContract and then invoked with ContractCall using the same contract address and method names expected by the asset layer (e.g. `condense`, `evaporate`).

## References

- [API_TRANSACTIONS.md](API_TRANSACTIONS.md) — transaction format and API
- [PERFORMANCE.md](PERFORMANCE.md) — gas and limits for production
- [MISTBORN_GUIDE.md](MISTBORN_GUIDE.md) — asset operations
