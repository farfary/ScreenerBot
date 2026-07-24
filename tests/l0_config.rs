//! L0 (pure): config serialize -> toml file -> parse round-trip. No network/wallet.
//!
//! Config is macro-driven (`config_struct!`, 20 sections), so a serde regression is
//! otherwise silent — the app would start with defaults and quietly drop persisted
//! settings. This asserts a non-default value survives a full save/load cycle.
//!
//! Kept in its own file (its own test binary → own process) because it initialises the
//! global CONFIG via `load_config_from_path`, which uses `OnceLock::set` and would
//! clash with any other config-initialising test sharing the process.

mod common;

use screenerbot::config::schemas::Config;
use screenerbot::config::utils::{load_config_from_path, save_config_to_file, with_config};

#[test]
fn config_survives_toml_round_trip() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    let path = dir.path().join("config.toml");
    let path_str = path.to_str().expect("utf-8 temp path");

    let mut cfg = Config::default();
    cfg.trader.max_open_positions = 7;
    cfg.swaps.slippage.quote_default_pct = 2.5;

    save_config_to_file(&cfg, path_str, false).expect("save config to file");
    assert!(path.exists(), "config file should be written");

    load_config_from_path(path_str).expect("load config into global");

    with_config(|c| {
        assert_eq!(
            c.trader.max_open_positions, 7,
            "int field survived round-trip"
        );
        assert_eq!(
            c.swaps.slippage.quote_default_pct, 2.5,
            "float field survived round-trip"
        );
    });
}
