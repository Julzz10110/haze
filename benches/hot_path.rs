//! Benchmarks for consensus/state hot path (Phase 2.4).
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use haze::config::Config;
use haze::state::StateManager;
use haze::consensus::ConsensusEngine;
use haze::crypto::KeyPair;
use haze::types::{Block, BlockHeader, Address};
use std::sync::Arc;

fn config_with_temp_db() -> (tempfile::TempDir, Config) {
    let temp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.storage.db_path = temp.path().join("db");
    config.storage.blob_storage_path = temp.path().join("blobs");
    (temp, config)
}

fn empty_block_height_1(validator: Address) -> Block {
    let mut header = BlockHeader {
        hash: [0u8; 32],
        parent_hash: [0u8; 32],
        height: 1,
        timestamp: 0,
        validator,
        merkle_root: [0u8; 32],
        state_root: [0u8; 32],
        wave_number: 0,
        committee_id: 0,
    };
    header.hash = header.compute_hash();
    Block {
        header,
        transactions: vec![],
        dag_references: vec![],
    }
}

fn bench_compute_state_root(c: &mut Criterion) {
    let (_temp, config) = config_with_temp_db();
    let state = StateManager::new(&config).unwrap();
    c.bench_function("compute_state_root_empty", |b| {
        b.iter(|| black_box(state.compute_state_root()))
    });
}

fn bench_apply_block(c: &mut Criterion) {
    let validator = KeyPair::generate().address();
    let block = empty_block_height_1(validator);
    c.bench_function("apply_block_empty", |b| {
        b.iter_with_setup(
            || {
                let (temp, config) = config_with_temp_db();
                let state = StateManager::new(&config).unwrap();
                (temp, state)
            },
            |(_temp, state)| {
                state.apply_block(&block).unwrap();
                black_box(())
            },
        )
    });
}

fn bench_process_block(c: &mut Criterion) {
    let validator = KeyPair::generate().address();
    c.bench_function("process_block_empty", |b| {
        b.iter_with_setup(
            || {
                let (temp, config) = config_with_temp_db();
                let state = Arc::new(StateManager::new(&config).unwrap());
                let consensus = ConsensusEngine::new(config, state.clone()).unwrap();
                let block = consensus.create_block(validator).unwrap();
                (temp, consensus, block)
            },
            |(_temp, consensus, block)| {
                consensus.process_block(&block).unwrap();
                black_box(())
            },
        )
    });
}

criterion_group!(benches, bench_compute_state_root, bench_apply_block, bench_process_block);
criterion_main!(benches);
