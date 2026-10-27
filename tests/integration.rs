// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The integration suite's one test binary: every `tests/*.rs` file and test directory is a
//! module of it, so a library change relinks one binary instead of one per file.
//!
//! The app keeps process-wide state (the config `OnceLock`, store pools, wallet caches), so the
//! whole suite runs under `cargo nextest run`, which gives every test its own process. Plain
//! `cargo test --test integration` is safe only filtered to one module, the isolation each file
//! had as a binary of its own.

mod agent_control_bridge;
mod agent_control_config_tools;
mod agent_control_store;
mod apis;
mod architecture_boundaries;
mod authority_blocking;
mod chart_load_perf;
mod common;
mod config;
mod config_api_surface;
mod config_migration;
mod conversions;
mod copy_analytics;
mod copy_exits;
mod copy_live;
mod copy_pipeline;
mod copy_sizing;
mod dashboard_launch_readiness;
mod dashboard_startup_perf;
mod direct_swaps_mainnet;
mod direct_swaps_offline;
mod events_runtime_enable;
mod filtering_decision_snapshot;
mod filtering_perf;
mod filtering_pipeline;
mod filtering_realdata;
mod filtering_sources;
mod filtering_surface_perf;
mod money_path_swaps;
mod ohlcv;
mod positions_apply;
mod positions_holdings;
mod positions_ledger;
mod positions_ledger_sync;
mod positions_lifecycle;
mod positions_pnl;
mod service_health_reporting;
mod service_startup_readiness;
mod solana_pools;
mod solana_rpc;
mod solana_rpc_subscriptions;
mod solana_token_balance;
mod solana_transactions_decode;
mod storage_chain_columns_migration;
mod storage_chain_isolation;
mod storage_copy_columns_migration;
mod storage_eviction_realdata;
mod storage_market_columns_migration;
mod storage_positions_raw_migration;
mod storage_recovery;
mod storage_report_api;
mod storage_startup;
mod storage_subject_deltas_migration;
mod storage_tokens_migration;
mod storage_transaction_columns_migration;
mod storage_transactions_migration;
mod storage_unavailable_startup;
mod storage_wallet_amount_migration;
mod storage_wallet_columns_migration;
mod strategies_conditions;
mod strategies_engine;
mod strategies_routes;
mod subject_deltas_persistence;
mod swap_amount_execution;
mod swap_resilience;
mod swap_routers_mainnet;
mod swaps_chain_routing;
mod token_blacklist_scope;
mod trader_dca;
mod trader_exit_policy;
mod trader_exits;
mod trader_loss_limit;
mod trader_manual_routes;
mod trading;
mod trading_data_readiness;
mod transactions_subject;
mod wallet_ata_cleanup;
mod wallet_main_keypair;
mod wallet_monitor_locking;
mod wallet_readiness;
mod wallet_replacement;
mod wallet_snapshot_subject_scope;
mod wallet_watch;
mod webserver_boot_perf;

/// Every test file and test directory is declared above; a file missing from the list would
/// never compile or run.
#[test]
fn every_test_file_is_a_module_of_the_suite() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let root = std::fs::read_to_string(tests.join("integration.rs")).expect("read integration.rs");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&tests).expect("list tests") {
        let path = entry.expect("test entry").path();
        let name = if path.is_dir() && path.join("mod.rs").exists() {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        } else {
            None
        };
        if let Some(name) = name.filter(|name| name != "integration") {
            if !root.lines().any(|line| line == format!("mod {name};")) {
                missing.push(name);
            }
        }
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "test modules not declared in tests/integration.rs: {missing:?}"
    );
}
