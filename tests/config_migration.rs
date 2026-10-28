// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: a config file in the v0.2.13 layout (Solana settings in global
//! `[rpc]`/`[swaps]`/`[sol_price]` sections) loads with its values relocated,
//! stays untouched on disk until startup completes, and is then written in the
//! canonical `[chains.solana]` layout. A hot reload of a legacy file applies and
//! rewrites it at once. Keys set while that migration is pending change only themselves
//! in the file, which keeps the loaded layout until startup completes.
//! The exit-rule leaves of `[positions]` (trailing stop, partial stop-loss share and the
//! retired partial-exit switch) follow the same load, persist and reload path into `[trader]`;
//! the switch the loaded file held stays readable for the store migrations. The ROI target of
//! `[trader]` becomes the one step of the take-profit ladder on the same path, and its
//! uniform DCA rule (one drop, one size and a count) becomes an equal DCA ladder. A file
//! written before trailing bands existed loads with none.
//!
//! It runs in a process of its own (nextest isolates every test) because `load_config_from_path`
//! initialises the global CONFIG (`OnceLock::set`). The steps share that global,
//! so they run in one test, in order.

use screenerbot::config::utils::{
    load_config_from_path, parse_config_document, persist_pending_config_migration,
    reload_config_from_path, validate_config, with_config,
};
use screenerbot::config::DcaMeasureFrom;

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
        assert_eq!(
            cfg.chains.solana.access, "own_rpc",
            "a config written before the access setting reaches the chain through its own RPC"
        );
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
        assert!(!cfg.trader.take_profit_enabled);
        assert_eq!(cfg.trader.take_profit_levels_pct, vec![35.0]);
        assert_eq!(cfg.trader.take_profit_sell_pct, vec![100.0]);
        assert!(cfg.trader.dca_enabled);
        assert_eq!(cfg.trader.dca_levels_pct, vec![-15.0; 3]);
        assert_eq!(cfg.trader.dca_size_multipliers, vec![0.4; 3]);
        assert_eq!(cfg.trader.dca_measure_from, DcaMeasureFrom::Average);
        assert_eq!(cfg.trader.dca_cooldown_minutes, 45);
        assert_eq!(cfg.trader.dca_min_hold_minutes, 0);
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
    assert_eq!(reparsed.config.trader.take_profit_levels_pct, vec![35.0]);
    let trader = tables["trader"].as_table().expect("trader table");
    assert!(!trader.contains_key("roi_target_percent"));
    assert!(!trader.contains_key("roi_exit_enabled"));
    for retired in ["dca_threshold_pct", "dca_max_count", "dca_size_percentage"] {
        assert!(
            !trader.contains_key(retired),
            "[trader] {retired} still on disk"
        );
    }
    assert_eq!(reparsed.config.trader.dca_levels_pct, vec![-15.0; 3]);
    assert_eq!(reparsed.config.trader.dca_size_multipliers, vec![0.4; 3]);

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
         [swaps.slippage]\nquote_default_pct = 3.5\n\n\
         [trader]\nroi_target_percent = 60.0\n",
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
        assert_eq!(cfg.trader.take_profit_levels_pct, vec![60.0]);
        assert_eq!(cfg.trader.take_profit_sell_pct, vec![100.0]);
    });
    let rewritten = std::fs::read_to_string(&reload_path).expect("read reloaded config");
    let tables = top_level_tables(&rewritten);
    assert!(!tables.contains_key("rpc"));
    assert!(!tables.contains_key("swaps"));
    assert!(!parse_config_document(&rewritten).expect("parses").migrated);
}

#[test]
fn keys_set_while_a_migration_is_pending_leave_the_loaded_layout_on_disk() {
    crate::common::run_isolated(
        module_path!(),
        "keys_set_while_a_migration_is_pending_leave_the_loaded_layout_on_disk",
        || async {
            let dir = tempfile::tempdir().expect("temp dir");
            std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            let path = screenerbot::paths::get_config_path();
            std::fs::write(&path, V0_2_13_CONFIG).expect("write fixture");
            load_config_from_path(path.to_str().expect("utf-8 temp path"))
                .expect("legacy layout loads");

            screenerbot::config::set_config_keys(&[
                (
                    "wallet_encrypted",
                    toml::Value::String("ciphertext".to_owned()),
                ),
                ("wallet_nonce", toml::Value::String("nonce".to_owned())),
                ("trader.enabled", toml::Value::Boolean(false)),
                ("copy_trading.enabled", toml::Value::Boolean(false)),
            ])
            .expect("set the keys");

            let written = std::fs::read_to_string(&path).expect("read config");
            let tables = top_level_tables(&written);
            for section in ["rpc", "swaps", "sol_price"] {
                assert!(
                    tables.contains_key(section),
                    "[{section}] was migrated early"
                );
            }
            assert!(
                !tables.contains_key("chains"),
                "the migrated layout reached disk"
            );
            assert_eq!(tables["wallet_encrypted"].as_str(), Some("ciphertext"));
            assert_eq!(tables["wallet_nonce"].as_str(), Some("nonce"));
            assert_eq!(tables["trader"]["enabled"].as_bool(), Some(false));
            assert_eq!(tables["trader"]["max_open_positions"].as_integer(), Some(4));
            assert_eq!(tables["copy_trading"]["enabled"].as_bool(), Some(false));
            with_config(|cfg| {
                assert_eq!(cfg.wallet_encrypted, "ciphertext");
                assert_eq!(cfg.chains.solana.rpc.max_retries, 5);
            });

            // The migration still waits for startup, then writes the keys with it.
            persist_pending_config_migration();
            let migrated = std::fs::read_to_string(&path).expect("read migrated config");
            let tables = top_level_tables(&migrated);
            assert!(!tables.contains_key("rpc"), "[rpc] still on disk");
            assert_eq!(tables["wallet_encrypted"].as_str(), Some("ciphertext"));
        },
    );
}

#[test]
fn exit_rule_leaves_move_into_trader_on_load_and_on_hot_reload() {
    crate::common::run_isolated(
        module_path!(),
        "exit_rule_leaves_move_into_trader_on_load_and_on_hot_reload",
        || async {
            let dir = tempfile::tempdir().expect("temp dir");
            std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
            let path = dir.path().join("config.toml");
            let path_str = path.to_str().expect("utf-8 temp path").to_owned();
            let legacy = "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\n\
                          [positions]\npartial_exit_enabled = false\n\
                          partial_exit_default_pct = 30.0\npartial_exit_min_pct = 10.0\n\
                          trailing_stop_enabled = true\ntrailing_stop_activation_pct = 0.5\n\
                          trailing_stop_distance_pct = 6.0\n";
            std::fs::write(&path, legacy).expect("write fixture");

            // Load: values in [trader] in memory, file bytes unchanged.
            load_config_from_path(&path_str).expect("legacy exit leaves load");
            assert_eq!(
                screenerbot::config::legacy_partial_exit_switch(),
                Some(false),
                "the load hands the retired switch to the store migrations"
            );
            assert_eq!(
                std::fs::read_to_string(&path).expect("read config"),
                legacy,
                "the load itself must not rewrite the file"
            );
            with_config(|cfg| {
                assert!(!cfg.trader.stop_loss_allow_partial);
                assert_eq!(cfg.trader.stop_loss_partial_pct, 30.0);
                assert!(cfg.trader.trailing_stop_enabled);
                assert_eq!(cfg.trader.trailing_stop_activation_pct, 0.5);
                assert_eq!(cfg.trader.trailing_stop_distance_pct, 6.0);
            });

            // Startup completed: the canonical layout reaches disk, once.
            persist_pending_config_migration();
            let written = std::fs::read_to_string(&path).expect("read migrated config");
            let tables = top_level_tables(&written);
            let positions = tables["positions"].as_table().expect("[positions]");
            for leaf in [
                "partial_exit_enabled",
                "partial_exit_default_pct",
                "partial_exit_min_pct",
                "trailing_stop_enabled",
                "trailing_stop_activation_pct",
                "trailing_stop_distance_pct",
            ] {
                assert!(
                    !positions.contains_key(leaf),
                    "[positions] {leaf} still on disk"
                );
            }
            assert_eq!(
                tables["trader"]["stop_loss_allow_partial"].as_bool(),
                Some(false)
            );
            assert_eq!(
                tables["trader"]["stop_loss_partial_pct"].as_float(),
                Some(30.0)
            );
            assert!(!parse_config_document(&written).expect("parses").migrated);

            // Hot reload of another legacy file: applied and rewritten at once.
            let reload_path = dir.path().join("reload.toml");
            let reload_str = reload_path.to_str().expect("utf-8 temp path").to_owned();
            std::fs::write(
                &reload_path,
                "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\n\
                 [positions]\npartial_exit_enabled = true\npartial_exit_default_pct = 40.0\n\
                 trailing_stop_distance_pct = 9.0\n",
            )
            .expect("write reload fixture");
            reload_config_from_path(&reload_str).expect("legacy exit leaves reload");
            assert_eq!(
                screenerbot::config::legacy_partial_exit_switch(),
                Some(false),
                "a hot reload keeps the switch the process loaded with"
            );
            with_config(|cfg| {
                assert!(cfg.trader.stop_loss_allow_partial);
                assert_eq!(cfg.trader.stop_loss_partial_pct, 40.0);
                assert_eq!(cfg.trader.trailing_stop_distance_pct, 9.0);
            });
            let rewritten = std::fs::read_to_string(&reload_path).expect("read reloaded config");
            let positions = top_level_tables(&rewritten)["positions"]
                .as_table()
                .expect("[positions]")
                .clone();
            assert!(!positions.contains_key("partial_exit_enabled"));
            assert!(!positions.contains_key("trailing_stop_distance_pct"));
            assert!(!parse_config_document(&rewritten).expect("parses").migrated);
        },
    );
}

/// A legacy ROI target outside the bounds of a take-profit step migrates clamped into them,
/// so the migrated ladder passes the validation every later config save runs.
#[test]
fn a_roi_target_outside_the_step_bounds_migrates_into_them() {
    for (legacy, migrated) in [("1500", 1000.0), ("1500.0", 1000.0), ("0.5", 1.0)] {
        let parsed = parse_config_document(&format!("[trader]\nroi_target_percent = {legacy}\n"))
            .expect("the legacy file parses");
        assert!(parsed.migrated);
        assert_eq!(
            parsed.config.trader.take_profit_levels_pct,
            vec![migrated],
            "roi_target_percent = {legacy}"
        );
        assert_eq!(parsed.config.trader.take_profit_sell_pct, vec![100.0]);
        validate_config(&parsed.config).unwrap_or_else(|error| {
            panic!("roi_target_percent = {legacy} migrated invalid: {error}")
        });
    }
}

/// The ladder a `[trader]` section in the uniform DCA layout migrates into, as
/// (drops, multiples, switch), after checking it passes the validation every later save runs.
fn migrated_dca(trader: &str) -> (Vec<f64>, Vec<f64>, bool) {
    let parsed = parse_config_document(&format!("[trader]\n{trader}"))
        .unwrap_or_else(|error| panic!("{trader:?} must parse: {error}"));
    assert!(parsed.migrated, "{trader:?} holds a legacy layout");
    validate_config(&parsed.config)
        .unwrap_or_else(|error| panic!("{trader:?} migrated invalid: {error}"));
    let trader = parsed.config.trader;
    assert_eq!(trader.dca_measure_from, DcaMeasureFrom::Average);
    (
        trader.dca_levels_pct,
        trader.dca_size_multipliers,
        trader.dca_enabled,
    )
}

/// Each counted add of the uniform rule becomes one equal step, clamped into the step
/// bounds; a key the file lacked takes the value the uniform rule defaulted to. A clamp
/// that adds later or buys less than the stored value keeps DCA on; one that would add
/// sooner or buy more turns it off, so no install spends more than it did.
#[test]
fn a_uniform_dca_rule_migrates_into_an_equal_ladder() {
    for (trader, drops, multiples, enabled) in [
        (
            "dca_threshold_pct = -12.0\ndca_max_count = 4\ndca_size_percentage = 75.0\n",
            vec![-12.0; 4],
            vec![0.75; 4],
            true,
        ),
        ("dca_max_count = 3\n", vec![-10.0; 3], vec![0.5; 3], true),
        (
            "dca_threshold_pct = -20.0\n",
            vec![-20.0; 2],
            vec![0.5; 2],
            true,
        ),
        (
            "dca_size_percentage = 30.0\n",
            vec![-10.0; 2],
            vec![0.3; 2],
            true,
        ),
        // A positive threshold fired below that profit; the loss of its size adds later.
        (
            "dca_threshold_pct = 15.0\ndca_max_count = 1\n",
            vec![-15.0],
            vec![0.5],
            true,
        ),
        (
            "dca_threshold_pct = 97.0\ndca_max_count = 1\n",
            vec![-95.0],
            vec![0.5],
            true,
        ),
        // A threshold shallower than the shallowest step adds later at that step.
        (
            "dca_threshold_pct = -0.5\n",
            vec![-1.0; 2],
            vec![0.5; 2],
            true,
        ),
        (
            "dca_threshold_pct = 0.0\n",
            vec![-1.0; 2],
            vec![0.5; 2],
            true,
        ),
        // A size above the largest multiple buys less at that multiple.
        (
            "dca_threshold_pct = -12.0\ndca_max_count = 9\ndca_size_percentage = 600.0\n",
            vec![-12.0; 5],
            vec![5.0; 5],
            true,
        ),
        // A threshold deeper than the deepest step, -100 never firing at all, would add
        // sooner at that step.
        (
            "dca_threshold_pct = -99.0\n",
            vec![-95.0; 2],
            vec![0.5; 2],
            false,
        ),
        (
            "dca_threshold_pct = -100.0\n",
            vec![-95.0; 2],
            vec![0.5; 2],
            false,
        ),
        // A size below the smallest multiple would buy more at that multiple.
        (
            "dca_size_percentage = 5.0\n",
            vec![-10.0; 2],
            vec![0.1; 2],
            false,
        ),
        (
            "dca_size_percentage = 0.0\n",
            vec![-10.0; 2],
            vec![0.1; 2],
            false,
        ),
    ] {
        let (migrated_drops, migrated_multiples, migrated_enabled) =
            migrated_dca(&format!("dca_enabled = true\n{trader}"));
        assert_eq!(migrated_drops, drops, "{trader:?}");
        assert_eq!(migrated_multiples, multiples, "{trader:?}");
        assert_eq!(migrated_enabled, enabled, "{trader:?}");
    }
}

/// A count of zero made no add at all: the migrated ladder holds one step and its switch
/// is off, so it still makes none and still validates.
#[test]
fn a_dca_count_of_zero_migrates_into_a_switched_off_ladder() {
    let (drops, multiples, enabled) = migrated_dca(
        "dca_enabled = true\ndca_threshold_pct = -10.0\ndca_max_count = 0\ndca_size_percentage = 50.0\n",
    );
    assert_eq!(drops, vec![-10.0]);
    assert_eq!(multiples, vec![0.5]);
    assert!(!enabled);
}

/// A count that is not a whole number of adds cannot become a ladder, and the file is
/// refused instead of guessed at.
#[test]
fn a_dca_count_that_is_not_a_whole_number_is_refused() {
    for count in ["-1", "2.5", "\"two\""] {
        let raw = format!("[trader]\ndca_threshold_pct = -10.0\ndca_max_count = {count}\n");
        assert!(
            parse_config_document(&raw).is_err(),
            "dca_max_count = {count} must be refused"
        );
    }
}

/// A ladder already stored wins over the retired keys beside it, and the retired keys are
/// dropped either way.
#[test]
fn a_stored_dca_ladder_wins_over_the_retired_keys() {
    let parsed = parse_config_document(
        "[trader]\ndca_levels_pct = [-30.0]\ndca_size_multipliers = [1.0]\n\
         dca_threshold_pct = -10.0\ndca_max_count = 3\ndca_size_percentage = 50.0\n",
    )
    .expect("the mixed file parses");
    assert!(parsed.migrated);
    assert_eq!(parsed.config.trader.dca_levels_pct, vec![-30.0]);
    assert_eq!(parsed.config.trader.dca_size_multipliers, vec![1.0]);
}

/// A hand-edited file holding one ladder list beside the other list's retired key gets a
/// migrated list of the stored list's length, so the two stay equal and validate.
#[test]
fn a_retired_dca_key_beside_a_stored_ladder_list_takes_its_length() {
    let (drops, multiples, enabled) = migrated_dca(
        "dca_enabled = true\ndca_levels_pct = [-10.0, -20.0, -30.0]\ndca_size_percentage = 50.0\n",
    );
    assert_eq!(drops, vec![-10.0, -20.0, -30.0]);
    assert_eq!(multiples, vec![0.5; 3]);
    assert!(enabled);

    let (drops, multiples, enabled) = migrated_dca(
        "dca_enabled = true\ndca_size_multipliers = [0.5, 1.0, 2.0]\ndca_threshold_pct = -15.0\n\
         dca_max_count = 1\n",
    );
    assert_eq!(drops, vec![-15.0; 3]);
    assert_eq!(multiples, vec![0.5, 1.0, 2.0]);
    assert!(enabled);
}

/// The migration is final: the migrated config written back out parses without one.
#[test]
fn a_migrated_dca_ladder_migrates_no_further() {
    let parsed = parse_config_document(
        "[trader]\ndca_enabled = true\ndca_threshold_pct = -12.0\ndca_max_count = 3\n",
    )
    .expect("the legacy file parses");
    let written = toml::to_string(&parsed.config).expect("the config serialises");
    let reparsed = parse_config_document(&written).expect("the migrated file parses");
    assert!(!reparsed.migrated);
    assert_eq!(reparsed.config.trader.dca_levels_pct, vec![-12.0; 3]);
    assert_eq!(reparsed.config.trader.dca_size_multipliers, vec![0.5; 3]);
    assert_eq!(
        reparsed.config.trader.dca_measure_from,
        DcaMeasureFrom::Average
    );
}

/// A stored DCA basis in another letter case loads under its canonical name, and one that
/// names no basis loads as the average entry the trader has always measured from.
#[test]
fn a_stored_dca_basis_loads_under_its_canonical_name() {
    for (stored, basis) in [
        ("First_Entry", DcaMeasureFrom::FirstEntry),
        ("AVERAGE", DcaMeasureFrom::Average),
        ("fortnightly", DcaMeasureFrom::Average),
    ] {
        let parsed = parse_config_document(&format!(
            "[trader]\ndca_measure_from = \"{stored}\"\ndca_levels_pct = [-10.0, -20.0]\n\
             dca_size_multipliers = [0.5, 0.5]\n"
        ))
        .expect("the stored basis loads");
        assert!(parsed.migrated, "{stored}");
        assert_eq!(parsed.config.trader.dca_measure_from, basis, "{stored}");
    }
}

/// A file written before trailing bands existed loads with none, so its trailing stop
/// keeps the one distance it had; a file that stores bands loads them.
#[test]
fn a_file_without_trailing_bands_trails_at_its_one_distance() {
    let parsed = parse_config_document(
        "[trader]\ntrailing_stop_enabled = true\ntrailing_stop_activation_pct = 20.0\n\
         trailing_stop_distance_pct = 8.0\n",
    )
    .expect("a trader section without bands loads");
    assert!(parsed.config.trader.trailing_stop_bands().is_empty());
    assert!(validate_config(&parsed.config).is_ok());

    let stored = parse_config_document(
        "[trader]\ntrailing_stop_band_profit_pct = [50.0]\n\
         trailing_stop_band_distance_pct = [5.0]\n",
    )
    .expect("stored bands load");
    assert_eq!(stored.config.trader.trailing_stop_bands().len(), 1);
}
