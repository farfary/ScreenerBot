// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! This machine's real `config.toml` upgrades to the current defaults and changes nothing else.
//!
//! The file is read, never written: every step parses an in-memory copy. The copy is first
//! put back into the shape a previous release saved it in (no `defaults_revision`, every
//! changed default at its old value), then parsed the way the first start of this release
//! parses it. The result must equal the file parsed as it is, apart from the defaults the
//! ledger moves, and must not migrate a second time.
//!
//! # Tier
//!
//! `#[ignore]`, run by `./test.sh live`. Self-skips when this machine has no real data
//! directory.

use crate::common;

use screenerbot::config::utils::parse_config_document;

/// The switches whose default moved to on, as a previous release saved them.
const PREVIOUS_RELEASE_ROUTES: [(&str, bool); 2] = [("direct", false), ("raptor", false)];

fn routes(config: &screenerbot::config::Config) -> [bool; 3] {
    let swaps = &config.chains.solana.swaps;
    [
        swaps.direct.enabled,
        swaps.jupiter.enabled,
        swaps.raptor.enabled,
    ]
}

/// `raw` as a previous release would have saved it.
fn as_previous_release_saved(raw: &str) -> String {
    let mut table: toml::Table = toml::from_str(raw).expect("real config is TOML");
    table.remove("defaults_revision");
    let swaps = table
        .get_mut("chains")
        .and_then(|chains| chains.get_mut("solana"))
        .and_then(|solana| solana.get_mut("swaps"))
        .and_then(toml::Value::as_table_mut)
        .expect("a saved config holds [chains.solana.swaps]");
    for (route, enabled) in PREVIOUS_RELEASE_ROUTES {
        let section = swaps
            .entry(route)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("a route section is a table");
        section.insert("enabled".to_owned(), toml::Value::Boolean(enabled));
    }
    toml::to_string(&table).expect("serialize")
}

fn without_route_switches(config: &screenerbot::config::Config) -> serde_json::Value {
    let mut config = config.clone();
    config.chains.solana.swaps.direct.enabled = true;
    config.chains.solana.swaps.raptor.enabled = true;
    serde_json::to_value(&config).expect("config serializes")
}

#[test]
#[ignore = "real data"]
fn the_real_config_upgrades_to_every_route_on_and_keeps_every_other_value() {
    let Some(raw) = common::real_config_toml() else {
        eprintln!("skipped: no real data directory");
        return;
    };
    let as_is = parse_config_document(&raw).expect("real config parses");

    let previous = as_previous_release_saved(&raw);
    let upgraded = parse_config_document(&previous).expect("previous-release shape parses");
    assert!(
        upgraded.migrated,
        "the first start of this release migrates"
    );
    assert_eq!(
        routes(&upgraded.config),
        [true, as_is.config.chains.solana.swaps.jupiter.enabled, true],
        "direct and Raptor turn on; Jupiter keeps its saved value"
    );
    assert_eq!(
        without_route_switches(&upgraded.config),
        without_route_switches(&as_is.config),
        "nothing but the moved defaults changes"
    );

    let written = toml::to_string(&upgraded.config).expect("serialize upgraded config");
    let again = parse_config_document(&written).expect("upgraded config parses");
    assert!(
        !again.migrated,
        "the upgrade is written once and never repeats"
    );
    assert_eq!(routes(&again.config), routes(&upgraded.config));
}
