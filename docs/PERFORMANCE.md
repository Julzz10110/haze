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

## Load tests

Load tests cover transfers, MistbornAsset creation, and mixed workloads. Use them to measure throughput (TPS), success rate, and latency under load.

### Running Rust load tests

Integration load tests live in `tests/integration/load_test.rs`. Run all load tests:

```bash
cargo test test_load_
```

Or run specific tests: `test_load_many_transfers`, `test_load_create_many_assets`, `test_load_mixed_transfers_and_assets`, `test_load_batch_operations`, `test_load_search_performance`, `test_stress_many_assets_per_account`. Use filter: `cargo test test_load_` or `cargo test test_stress_`.

Tests run in-process (no separate node); they create a fresh state and consensus, submit transactions, and process blocks until the pool is empty. Output includes TPS (transactions or assets per second) and block count. Assertions verify balances, asset counts, and quota consistency.

### Running SDK load test

The SDK example sends transactions to a running node (or several, round-robin). Build and run:

```bash
cd sdk && npm run build
node dist/examples/load-test.js
```

**Environment variables:**

| Variable | Default | Description |
|----------|---------|--------------|
| `HAZE_LOAD_NODE_URLS` | `http://localhost:8080` | Comma-separated node URLs (round-robin). |
| `HAZE_LOAD_TX_COUNT` | `100` | Total transactions to send. |
| `HAZE_LOAD_TX_PER_SEC` | `10` | Target rate (tx/sec). |
| `HAZE_LOAD_CONCURRENT` | `1` | Number of concurrent senders (capped at 50). |
| `HAZE_LOAD_MODE` | `transfer` | `transfer`, `asset`, or `mixed`. |
| `HAZE_LOAD_MIX_RATIO` | `50` | In `mixed` mode: percent of transfers (0–100); rest are asset creates. |

For `asset` and `mixed` modes the sender account must have balance (gas/fees); use a faucet or pre-fund the account.

The script prints duration, sent/success/failed, success rate, latency percentiles (P50, P95, P99), and actual vs target rate.

### Stress scenario: Many assets per account

**Goal:** Verify that one account can create a large number of assets without state corruption and within config limits.

**Steps:**

1. Ensure default config allows the target count (e.g. `max_assets_per_account` in `asset_limits.quotas` for your `network.node_type`).
2. Run the Rust stress test:  
   `cargo test test_stress_many_assets_per_account`  
   This creates 500 assets in-process and asserts asset count and quota.
3. Alternatively, run the SDK against a live node (with a funded account):  
   `HAZE_LOAD_MODE=asset HAZE_LOAD_TX_COUNT=500 node dist/examples/load-test.js`

**Expected result:** All transactions succeed; `assets_count` quota equals the number of creates; no panics or state inconsistency.

**Observed limits (reference):** Depend on hardware and config. With default config, the in-process Rust test typically completes 500 asset creates in a few seconds. Max TPS is limited by block assembly and application (see [Observed limits (reference)](#observed-limits-reference) and [Tuning](#tuning)).

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

**Load and throughput:**

4. **Block interval** — Lower interval (e.g. 1–2 s in `main.rs`) increases max TPS; higher interval reduces block volume and storage growth.
5. **Transactions per block** — `consensus.max_transactions_per_block` caps how many tx are included per block; increase for higher throughput per block.
6. **Tx pool** — Pool is in-memory; ensure the node has enough RAM for the expected pool size under load.
7. **VM and asset limits** — `vm.gas_limit`, `vm.gas_price`, and `asset_limits.quotas` (e.g. `max_assets_per_account`, `max_metadata_size`) define per-tx and per-account caps; increase for stress tests or relax for dev, decrease for stricter production limits.
8. **Storage (sled)** — Default DB path and sled settings; on high write load, monitor disk and consider faster storage or tuning sled options if exposed.

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

### Observed limits (reference)

These numbers are indicative; run load tests in your target environment for accurate values.

- **Max assets per account:** From config `asset_limits.quotas.<node_type>.max_assets_per_account` (e.g. 10_000 for core by default). The stress test `test_stress_many_assets_per_account` creates 500 assets; ensure your quota ≥ 500 if you run it.
- **Throughput:** In-process Rust load tests (single thread, no network) can reach hundreds of tx/s for transfers or asset creates, depending on block size and `max_transactions_per_block`. Realistic TPS with a live node and network is lower; use the SDK load test against your node to measure.
- **Block interval:** Default 5 s in `main.rs` limits how often blocks are created; reduce it for higher throughput in development (at the cost of more blocks and storage).

### Recommendations

- Set `vm.gas_limit` and `vm.gas_price` according to desired fee level and DoS resistance.
- Tune `asset_limits.quotas` per node role (core nodes can allow higher limits than light/mobile).
- Use `estimate-gas` in clients to show users expected fees before signing.

## References

- [OBSERVABILITY.md](OBSERVABILITY.md) — Metrics endpoint and logging
- [MISTBORN_GUIDE.md](MISTBORN_GUIDE.md) — Asset operations and API
