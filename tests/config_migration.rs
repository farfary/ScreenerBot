// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: a config file in the v0.2.13 layout (Solana settings in global
//! `[rpc]`/`[swaps]`/`[sol_price]` sections) loads with its values relocated,
//! stays untouched on disk until startup completes, and is then written in the
//! canonical `[chains.solana]` layout. A hot reload of a legacy file applies and
//! rewrites it at once.
//!
//! Its own file (own test binary → own process) because `load_config_from_path`
//! initialises the global CONFIG (`OnceLock::set`). The steps share that global,
//! so they run in one test, in order.

use screenerbot::config::utils::{
    load_config_from_path, parse_config_document, persist_pending_config_migration,
    reload_config_from_path, with_config,
};

const V0_2_13_CONFIG: &str = include_str!("fixtures/v0.2.13-config.toml");

fn top_level_tables(raw: &str) -> toml::Table {
    toml::from_str(raw).expect("config file is TOML")
}

#[test]
fn legacy_layout_is_migrated_in_memory_then_persisted_after_startup() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    let path = dir.path().join("config.toml");
    let path_str = path.to_str().expect("utf-8 temp path").to_owned();
    std::fs::write(&path, V0_2_13_CONFIG).expect("write fixture");

    // Load: values relocated in memory, file bytes unchanged.
    load_config_from_path(&path_str).expect("legacy layout loads");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        V0_2_13_CONFIG,
        "the load itself must not rewrite the file"
    );
    with_config(|cfg| {
        assert_eq!(cfg.chains.solana.rpc.max_retries, 5);
        assert_eq!(
            cfg.chains.solana.rpc.urls,
            vec![
                "https://rpc-one.fixture.invalid/".to_owned(),
                "https://rpc-two.fixture.invalid/".to_owned()
            ]
        );
        assert_eq!(
            cfg.chains.solana.swaps.jupiter.api_key,
            "fixture-jupiter-key"
        );
        assert_eq!(cfg.chains.solana.swaps.raptor.max_hops, 2);
        assert_eq!(cfg.trader.slippage.quote_default_pct, 2.5);
        assert_eq!(cfg.chains.solana.connectivity.jupiter.timeout_secs, 7);
        assert_eq!(cfg.trader.trade_size_sol, 0.02);
    });

    // Startup completed: the canonical layout reaches disk.
    persist_pending_config_migration();
    let written = std::fs::read_to_string(&path).expect("read migrated config");
    assert!(written.contains("[chains.solana.rpc]"));
    let tables = top_level_tables(&written);
    for section in ["rpc", "swaps", "sol_price"] {
        assert!(!tables.contains_key(section), "[{section}] still on disk");
    }
    let reparsed = parse_config_document(&written).expect("migrated file parses");
    assert!(!reparsed.migrated, "a persisted migration must be final");
    assert_eq!(reparsed.config.chains.solana.rpc.max_retries, 5);
    assert_eq!(reparsed.config.trader.slippage.exit_loss_shortfall_pct, 7.5);

    // A second persist has nothing pending and leaves the file alone.
    persist_pending_config_migration();
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        written
    );

    // Hot reload of another legacy file: applied and rewritten immediately.
    let reload_path = dir.path().join("reload.toml");
    let reload_str = reload_path.to_str().expect("utf-8 temp path").to_owned();
    std::fs::write(
        &reload_path,
        "[rpc]\nurls = [\"https://reload.fixture.invalid/\"]\nmax_retries = 8\n\n\
         [swaps.slippage]\nquote_default_pct = 3.5\n",
    )
    .expect("write reload fixture");
    reload_config_from_path(&reload_str).expect("legacy layout reloads");
    with_config(|cfg| {
        assert_eq!(cfg.chains.solana.rpc.max_retries, 8);
        assert_eq!(
            cfg.chains.solana.rpc.urls,
            vec!["https://reload.fixture.invalid/".to_owned()]
        );
        assert_eq!(cfg.trader.slippage.quote_default_pct, 3.5);
    });
    let rewritten = std::fs::read_to_string(&reload_path).expect("read reloaded config");
    let tables = top_level_tables(&rewritten);
    assert!(!tables.contains_key("rpc"));
    assert!(!tables.contains_key("swaps"));
    assert!(!parse_config_document(&rewritten).expect("parses").migrated);
}
