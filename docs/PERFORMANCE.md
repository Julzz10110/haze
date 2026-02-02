# HAZE Performance & Benchmarks

This document describes target metrics, how to run benchmarks, and how to interpret results.

## Target metrics

| Metric | Target | Notes |
|--------|--------|--------|
| **Block propagation** | &lt; 50 ms | Time from block creation on one node to receipt on another node (P2P). Measured via multi-node setup and logs or external tooling. |
| **Time to finalization** | &lt; 200 ms | Time from block creation to the block’s wave being finalized (`last_finalized_height` updated). Depends on `golden_wave_threshold` (default 500 ms) and wave having ≥2 blocks. |

These are goals for tuning; current defaults (e.g. 5 s block interval, 500 ms golden wave) may not hit them without tuning (see [Tuning](#tuning)).

## Benchmarks (hot path)

Basic benchmarks for consensus/state hot path are in `benches/hot_path.rs` and use [Criterion](https://github.com/bheisler/criterion.rs).

### Run benchmarks

```bash
cargo bench
```

To run a specific benchmark:

```bash
cargo bench -- compute_state_root
cargo bench -- apply_block
cargo bench -- process_block
```

### Benchmarked operations

| Benchmark | What it measures |
|-----------|------------------|
| **compute_state_root_empty** | Cost of `StateManager::compute_state_root()` with empty state (no accounts/assets). |
| **apply_block_empty** | Cost of `StateManager::apply_block()` for one empty block (height 1). Includes block rewards, storing block, updating height. Fresh state per iteration (temp DB). |
| **process_block_empty** | Cost of `ConsensusEngine::process_block()` for one block: DAG insert, wave update, `apply_block`, optional wave finalization check. Fresh consensus+state per iteration. |

Results are printed in the terminal; Criterion also writes HTML reports under `target/criterion/` if the `html_reports` feature is enabled.

## Existing metrics (API)

The node exposes basic metrics via HTTP; see [OBSERVABILITY.md](OBSERVABILITY.md).

- **GET /api/v1/metrics/basic** — `current_height`, `last_finalized_height`, `last_finalized_wave`, `tx_pool_size`, `connected_peers`, `block_time_avg_ms` (average block time from last 10 blocks).
- **GET /api/v1/sync/status** — Same heights + `connected_peers`, `syncing`.

Block propagation delay and time-to-finalization are not yet exposed as API fields; they can be derived from logs (block created vs block received vs wave finalized) or from multi-node tests.

## Tuning

If observed propagation or finalization time exceeds targets:

1. **Propagation**
   - Increase network buffer sizes if available (libp2p / request-response config).
   - Reduce block size or batch sync size if large blocks dominate latency.
   - Ensure nodes are on a low-latency network (e.g. same region).

2. **Finalization**
   - Lower `consensus.golden_wave_threshold` (ms) so waves finalize sooner (at the cost of less time for multiple blocks in the same wave).
   - Ensure enough blocks per wave (min 2) so finalization can trigger.

3. **Block creation interval**
   - Current MVP uses a fixed 5 s interval in `main.rs`. Tuning block production rate may affect both propagation and finalization metrics.

## Gas and limits (production)

For production use, operators should be aware of the following limits and gas settings.

### VM and transaction gas

- **`vm.gas_limit`** — Maximum gas per transaction (default: 10_000_000). ContractCall and MistbornAsset operations consume gas; exceeding this limit causes the transaction to fail. ContractCall’s `gas_limit` must not exceed this value (enforced in consensus).
- **`vm.gas_price`** — Gas price in base units (default: 1). Fee for asset operations is `gas_cost * gas_price`; 50% of gas fees are burned (see tokenomics).
- **`POST /api/v1/assets/estimate-gas`** — Use this endpoint to estimate gas cost and fee before submitting an asset transaction.

### Asset gas (AssetGasConfig)

Gas costs for Mistborn operations are configured in `config.asset_gas`: create (base + per KB metadata), update, condense (base + density multiplier + per KB), evaporate, merge (base + per KB combined size), split (base + per component + per KB). See `haze_config.json` after first run for full structure.

### Asset limits (AssetLimits, NodeQuotas)

- **Per account:** `max_assets_per_account` (by node type: core/edge/light/mobile).
- **Per asset:** `max_metadata_size` (bytes), `max_blob_files_per_asset`.
- **Per account blob storage:** `max_blob_storage_per_account` (bytes, estimated from blob count).

Limits are enforced in state before create/update/condense; exceeding them returns `InvalidTransaction` or `AssetSizeExceeded`. Node type is taken from `config.network.node_type` and selects the quota (e.g. `asset_limits.quotas.light`).

### Recommendations

- Set `vm.gas_limit` and `vm.gas_price` according to desired fee level and DoS resistance.
- Tune `asset_limits.quotas` per node role (core nodes can allow higher limits than light/mobile).
- Use `estimate-gas` in clients to show users expected fees before signing.

## References

- [OBSERVABILITY.md](OBSERVABILITY.md) — Metrics endpoint and logging
- [MISTBORN_GUIDE.md](MISTBORN_GUIDE.md) — Asset operations and API
