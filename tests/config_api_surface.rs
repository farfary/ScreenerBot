// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: the config API surface must cover every section the dashboard renders.
//!
//! The config page is data-driven: it renders one section per entry of
//! `GET /api/config/metadata` but reads the values from `GET /api/config`, and
//! saves through `PATCH /api/config/<section>`. Those three lists are hand-wired
//! in three different files, so a new config section can be added to
//! `Config`/`collect_config_metadata()` while `FullConfigResponse` and the router
//! never learn about it. That failure is silent: the section renders with blank
//! inputs, booleans fall back to unchecked, and saving 404s. It shipped exactly
//! that way for seven sections (pools, maintenance, wallet, strategies,
//! holder_watch, performance, webserver).
//!
//! These tests fail the moment those lists drift apart again.
//!
//! Its own file (own test binary → own process) because it initialises the
//! global CONFIG via `load_config_from_path` (`OnceLock::set`).

mod common;

use axum::Json;
use screenerbot::config::metadata::collect_config_metadata;
use screenerbot::config::schemas::{Config, GuiConfig};
use screenerbot::config::updates::update_config_section;
use screenerbot::config::utils::{load_config_from_path, save_config_to_file, with_config};
use screenerbot::webserver::routes::config::getters::{get_full_config, patch_any_config};
use screenerbot::webserver::routes::config::import_export::{
    export_config, import_config, import_config_preview,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Once;

/// Router source, read at compile time: the route table is a static list, so
/// asserting against its text needs no AppState and no running server.
const ROUTER_SRC: &str = include_str!("../src/webserver/routes/config/mod.rs");

/// Top-level `Config` fields that deliberately have no config-page section.
/// Everything else MUST be reachable from the page — add a section to
/// `collect_config_metadata()` rather than extending this list.
const SECTIONS_WITHOUT_METADATA: &[&str] = &[
    // Secrets, never exposed or edited as config fields.
    "wallet_encrypted",
    "wallet_nonce",
    // Internal tuning with no user-facing controls.
    "connectivity",
    // Desktop shell settings, owned by the GUI's own preferences UI.
    "gui",
];

static INIT: Once = Once::new();

/// Serializes the tests that write the global config with the tests that
/// compare two reads of it.
static CONFIG_WRITES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// One global config per test binary — `load_config_from_path` uses a `OnceLock`.
/// Seeds the webserver auth secrets so the sanitisation test is not vacuous.
fn init_config() {
    INIT.call_once(init_config_once);
}

fn init_config_once() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    // Persisting updates writes to `<data dir>/data/config.toml`.
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    let path = dir.path().join("config.toml");
    let path_str = path.to_str().expect("utf-8 temp path").to_owned();
    save_config_to_file(&Config::default(), &path_str, false).expect("write default config");
    load_config_from_path(&path_str).expect("load config into global");
    update_config_section(
        |cfg| {
            cfg.webserver.auth_password_hash = "hash-must-not-leak".to_owned();
            cfg.webserver.auth_password_salt = "salt-must-not-leak".to_owned();
            cfg.webserver.auth_totp_secret = "totp-must-not-leak".to_owned();
        },
        false,
    )
    .expect("seed webserver auth secrets");
    // Keep the tempdir alive for the whole process — the global config holds its path.
    std::mem::forget(dir);
}

async fn full_config_json() -> Value {
    let response = get_full_config().await;
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read /api/config body");
    serde_json::from_slice(&bytes).expect("/api/config returns JSON")
}

fn metadata_sections() -> BTreeSet<String> {
    collect_config_metadata()
        .keys()
        .map(|k| k.to_string())
        .collect()
}

fn config_sections() -> BTreeSet<String> {
    let value = serde_json::to_value(Config::default()).expect("serialize default config");
    value
        .as_object()
        .expect("config serializes to an object")
        .keys()
        .cloned()
        .collect()
}

/// `.route("/config/<name>", ...)` entries that mention the given method.
fn routed_sections(method: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for block in ROUTER_SRC.split(".route(").skip(1) {
        let Some(start) = block.find("\"/config/") else {
            continue;
        };
        let rest = &block[start + "\"/config/".len()..];
        let Some(end) = rest.find('"') else { continue };
        let path = &rest[..end];
        // Only the section endpoints — not /config/metadata, /config/reload, …
        if path.contains('/') {
            continue;
        }
        // The handler list of one .route() call ends at the next call.
        let handlers = block.split(".route(").next().unwrap_or(block);
        if handlers.contains(&format!("{method}(")) {
            found.insert(path.to_string());
        }
    }
    found
}

#[tokio::test]
async fn full_config_carries_every_metadata_section() {
    init_config();
    let payload = full_config_json().await;
    let served: BTreeSet<String> = payload
        .as_object()
        .expect("/api/config returns an object")
        .keys()
        .cloned()
        .collect();

    let missing: Vec<String> = metadata_sections()
        .into_iter()
        .filter(|section| !served.contains(section))
        .collect();

    assert!(
        missing.is_empty(),
        "sections rendered by the config page but absent from GET /api/config: {missing:?}\n\
         Add them to FullConfigResponse (src/webserver/routes/config/types.rs) and to \
         get_full_config (getters.rs) — without them the page renders blank inputs and \
         cannot revert the section."
    );
}

#[tokio::test]
async fn full_config_values_match_the_live_config() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    let payload = full_config_json().await;
    // Compare against the LIVE config, not Config::default(): the live one has
    // been through a TOML round-trip, and f32 fields widen on the way back.
    // Serialize -> bytes -> parse, the exact path axum's Json takes: to_value()
    // widens f32 (1.2 becomes 1.2000000476837158) and would false-alarm.
    let expected: Value = with_config(|cfg| {
        let bytes = serde_json::to_vec(cfg).expect("serialize the live config");
        serde_json::from_slice(&bytes).expect("live config parses back")
    });

    // Every served section must equal the config it claims to mirror, except the
    // webserver section whose auth secrets are deliberately blanked.
    for section in metadata_sections() {
        if section == "webserver" {
            continue;
        }
        let Some(expected_section) = expected.get(&section) else {
            continue;
        };
        assert_eq!(
            payload.get(&section),
            Some(expected_section),
            "GET /api/config serves a different value than the live config for section \
             '{section}' — the handler is mapping the wrong field"
        );
    }
}

#[tokio::test]
async fn webserver_auth_secrets_never_leave_the_process() {
    init_config();
    let payload = full_config_json().await;
    let webserver = payload
        .get("webserver")
        .expect("webserver section is served");

    for secret in [
        "auth_password_hash",
        "auth_password_salt",
        "auth_totp_secret",
    ] {
        assert_eq!(
            webserver.get(secret).and_then(Value::as_str),
            Some(""),
            "GET /api/config must blank webserver.{secret} — the dashboard has no use for \
             an auth secret and PATCH merges only the keys the client sends"
        );
    }
}

#[test]
fn every_metadata_section_has_get_and_patch_routes() {
    let sections = metadata_sections();
    let gets = routed_sections("get");
    let patches = routed_sections("patch");

    let missing_get: Vec<&String> = sections.iter().filter(|s| !gets.contains(*s)).collect();
    let missing_patch: Vec<&String> = sections.iter().filter(|s| !patches.contains(*s)).collect();

    assert!(
        missing_get.is_empty(),
        "config sections with no GET /api/config/<section> route: {missing_get:?}"
    );
    assert!(
        missing_patch.is_empty(),
        "config sections with no PATCH /api/config/<section> route: {missing_patch:?} — \
         editing such a section 404s on save"
    );
}

#[test]
fn every_config_section_is_reachable_from_the_config_page() {
    let metadata = metadata_sections();
    let orphans: Vec<String> = config_sections()
        .into_iter()
        .filter(|section| {
            !metadata.contains(section) && !SECTIONS_WITHOUT_METADATA.contains(&section.as_str())
        })
        .collect();

    assert!(
        orphans.is_empty(),
        "config sections that exist in Config but are invisible on the config page: \
         {orphans:?} — register them in collect_config_metadata(), or list them in \
         SECTIONS_WITHOUT_METADATA with the reason they are not user-editable"
    );
}

#[test]
fn patch_handler_knows_every_metadata_section_type() {
    let getters_src = include_str!("../src/webserver/routes/config/getters.rs");
    // patch_any_config dispatches on the type name; a section whose type is
    // missing from that match returns "Failed to serialize current config".
    let mut unknown = Vec::new();
    for section in metadata_sections() {
        let type_name = format!(
            "\"{}Config\"",
            section
                .split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<String>()
        );
        if getters_src.matches(&type_name).count() < 2 {
            unknown.push(section);
        }
    }

    assert!(
        unknown.is_empty(),
        "sections missing a read and/or write arm in patch_any_config: {unknown:?} — \
         saving them fails at runtime with no compile error"
    );
}

/// A config that enables no chain cannot boot — the process chain would not
/// exist — so the loader refuses it before the global config is replaced.
/// (A failed load never touches the global `CONFIG`, so this is order-safe
/// beside the tests above.)
#[test]
fn a_config_that_enables_no_chain_is_refused_at_load() {
    init_config();
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("no-chain.toml");
    std::fs::write(&path, "[chains.solana]\nenabled = false\n").expect("write fixture");

    let error = load_config_from_path(path.to_str().expect("utf-8 path"))
        .expect_err("a zero-enabled config must be refused at load");

    assert!(
        error.to_string().contains("at least one supported chain"),
        "unexpected refusal message: {error}"
    );
}

/// A PATCH body naming one nested field must leave that field's siblings, and
/// the sibling sub-sections, at their stored values.
#[tokio::test]
async fn patch_of_one_nested_field_keeps_its_siblings() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    update_config_section(
        |cfg| {
            cfg.gui.dashboard.interface.show_hints = false;
            cfg.gui.dashboard.interface.table_page_size = 50;
            cfg.gui.dashboard.lockscreen.password_hash = "stored-hash".to_owned();
            cfg.gui.dashboard.lockscreen.password_salt = "stored-salt".to_owned();
            cfg.gui.dashboard.startup.default_page = "positions".to_owned();
        },
        false,
    )
    .expect("seed gui settings");

    let response = patch_any_config::<GuiConfig>(Json(
        serde_json::json!({"dashboard": {"interface": {"sounds_enabled": false}}}),
    ))
    .await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    assert!(
        status.is_success(),
        "patch rejected: {status} {}",
        String::from_utf8_lossy(&body)
    );

    with_config(|cfg| {
        let dashboard = &cfg.gui.dashboard;
        assert!(!dashboard.interface.sounds_enabled);
        assert!(!dashboard.interface.show_hints);
        assert_eq!(dashboard.interface.table_page_size, 50);
        assert_eq!(dashboard.lockscreen.password_hash, "stored-hash");
        assert_eq!(dashboard.lockscreen.password_salt, "stored-salt");
        assert_eq!(dashboard.startup.default_page, "positions");
    });
}

/// The config layout written by v0.2.13, as a TOML document.
const V0_2_13_CONFIG: &str = include_str!("fixtures/v0.2.13-config.toml");

/// The sections a v0.2.13 export carried, taken from the v0.2.13 config file.
/// That release exported `rpc`, `swaps` and `sol_price` at the top level and
/// never wrote a `chains` section.
fn v0_2_13_export(sections: &[&str]) -> Value {
    let table: toml::Table = toml::from_str(V0_2_13_CONFIG).expect("fixture is valid TOML");
    let document = serde_json::to_value(table).expect("fixture converts to JSON");
    let mut export = serde_json::Map::new();
    for section in sections {
        let value = document
            .get(*section)
            .unwrap_or_else(|| panic!("fixture carries [{section}]"));
        export.insert((*section).to_owned(), value.clone());
    }
    export.insert(
        "timestamp".to_owned(),
        Value::String("2026-01-01T00:00:00+00:00".to_owned()),
    );
    Value::Object(export)
}

/// The status and JSON body of a handler response.
async fn response_json(response: axum::response::Response) -> (axum::http::StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
        panic!(
            "response body is JSON: {status} {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, body)
}

async fn post_import(body: Value) -> Value {
    let request = serde_json::from_value(body).expect("valid import request");
    let (status, body) = response_json(import_config(Json(request)).await).await;
    assert!(status.is_success(), "import rejected: {status} {body}");
    body
}

/// The imported value the preview reports for `field` of `section`.
fn previewed_change<'a>(preview: &'a Value, section: &str, field: &str) -> Option<&'a Value> {
    preview["sections"]
        .as_array()
        .expect("preview lists sections")
        .iter()
        .find(|entry| entry["name"] == section)?["changes"]
        .as_array()?
        .iter()
        .find(|change| change["field"] == field)
        .map(|change| &change["imported"])
}

#[tokio::test]
async fn import_preview_of_a_v0_2_13_export_reports_the_relocated_sections() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    // The preview lists only values that differ from the live config.
    update_config_section(
        |cfg| {
            cfg.chains.solana.rpc.urls = vec!["https://stored-rpc.fixture.invalid/".to_owned()];
            cfg.chains.solana.swaps.raptor.max_hops = 4;
            cfg.trader.slippage.quote_default_pct = 1.0;
        },
        false,
    )
    .expect("seed values the export changes");
    let request = serde_json::from_value(serde_json::json!({
        "config": v0_2_13_export(&["rpc", "swaps", "sol_price"]),
    }))
    .expect("valid preview request");
    let (status, preview) = response_json(import_config_preview(Json(request)).await).await;
    assert!(status.is_success(), "preview rejected: {status} {preview}");
    assert_eq!(preview["valid"], true, "preview: {preview}");

    let present: BTreeSet<&str> = preview["sections"]
        .as_array()
        .expect("preview lists sections")
        .iter()
        .filter(|entry| entry["present"] == true)
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(present, BTreeSet::from(["chains", "trader"]));

    assert_eq!(
        previewed_change(&preview, "chains", "solana.rpc.urls"),
        Some(&serde_json::json!([
            "https://rpc-one.fixture.invalid/",
            "https://rpc-two.fixture.invalid/"
        ]))
    );
    assert_eq!(
        previewed_change(&preview, "chains", "solana.swaps.raptor.max_hops"),
        Some(&serde_json::json!(2))
    );
    assert_eq!(
        previewed_change(&preview, "trader", "slippage.quote_default_pct"),
        Some(&serde_json::json!(2.5))
    );
}

#[tokio::test]
async fn merge_import_of_a_v0_2_13_export_lands_every_moved_value_at_its_new_path() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    post_import(serde_json::json!({
        "config": v0_2_13_export(&["rpc", "swaps", "sol_price", "trader"]),
        "merge": true,
        "save_to_disk": false,
    }))
    .await;

    with_config(|cfg| {
        let solana = &cfg.chains.solana;
        assert_eq!(
            solana.rpc.urls,
            [
                "https://rpc-one.fixture.invalid/",
                "https://rpc-two.fixture.invalid/"
            ]
        );
        assert_eq!(solana.rpc.helius_rate_limit, 61);
        assert!(!solana.rpc.circuit_breaker_enabled);
        assert_eq!(solana.swaps.jupiter.api_key, "fixture-jupiter-key");
        assert_eq!(solana.swaps.jupiter.priority_fee_micro_lamports, 75000);
        assert_eq!(solana.swaps.direct.max_price_impact_pct, 4.5);
        assert_eq!(solana.swaps.raptor.max_hops, 2);
        assert_eq!(solana.swaps.cost_guard.always_allow_below_lamports, 250000);
        assert_eq!(cfg.trader.slippage.quote_default_pct, 2.5);
        assert_eq!(cfg.trader.slippage.exit_retry_steps_pct, [5.0, 12.0, 30.0]);
        assert_eq!(cfg.trader.trade_size_sol, 0.02);
        assert_eq!(cfg.trader.max_open_positions, 4);
    });
}

#[tokio::test]
async fn sanitized_chains_export_round_trips_without_losing_the_jupiter_key() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    update_config_section(
        |cfg| {
            cfg.chains.solana.swaps.jupiter.api_key = "stored-jupiter-key".to_owned();
            cfg.chains.solana.swaps.raptor.max_hops = 3;
        },
        false,
    )
    .expect("seed chain settings");

    let request = serde_json::from_value(serde_json::json!({
        "sections": ["chains"],
        "sanitize_secrets": true,
    }))
    .expect("valid export request");
    let (status, export) = response_json(export_config(Json(request)).await).await;
    assert!(status.is_success(), "export rejected: {status} {export}");
    let mut exported = export["config"].clone();
    let jupiter = &exported["chains"]["solana"]["swaps"]["jupiter"];
    assert!(
        jupiter.is_object() && jupiter.get("api_key").is_none(),
        "a sanitized export must omit the Jupiter key: {jupiter}"
    );

    exported["chains"]["solana"]["swaps"]["raptor"]["max_hops"] = serde_json::json!(1);
    post_import(serde_json::json!({
        "config": exported,
        "merge": true,
        "save_to_disk": false,
    }))
    .await;

    with_config(|cfg| {
        assert_eq!(
            cfg.chains.solana.swaps.jupiter.api_key,
            "stored-jupiter-key"
        );
        assert_eq!(cfg.chains.solana.swaps.raptor.max_hops, 1);
    });
}

/// A legacy export holding only `swaps` builds `trader` and `chains` sections
/// that carry just the moved fields; importing them with merge unchecked must
/// not reset the rest of either section.
#[tokio::test]
async fn replace_import_of_a_partial_legacy_export_keeps_the_sections_it_lacks() {
    init_config();
    let _writes = CONFIG_WRITES.lock().await;
    update_config_section(
        |cfg| {
            cfg.trader.trade_size_sol = 0.042;
            cfg.trader.stop_loss_enabled = true;
            cfg.trader.max_open_positions = 7;
            cfg.chains.solana.rpc.urls = vec!["https://stored-rpc.fixture.invalid/".to_owned()];
            cfg.chains.solana.rpc.helius_rate_limit = 33;
        },
        false,
    )
    .expect("seed trader and rpc settings");

    let body = post_import(serde_json::json!({
        "config": v0_2_13_export(&["swaps"]),
        "merge": false,
        "save_to_disk": false,
    }))
    .await;
    let imported: BTreeSet<&str> = body["imported_sections"]
        .as_array()
        .expect("import lists its sections")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(imported, BTreeSet::from(["chains", "trader"]));

    with_config(|cfg| {
        assert_eq!(cfg.trader.trade_size_sol, 0.042);
        assert!(cfg.trader.stop_loss_enabled);
        assert_eq!(cfg.trader.max_open_positions, 7);
        assert_eq!(
            cfg.chains.solana.rpc.urls,
            ["https://stored-rpc.fixture.invalid/"]
        );
        assert_eq!(cfg.chains.solana.rpc.helius_rate_limit, 33);

        assert_eq!(cfg.trader.slippage.quote_default_pct, 2.5);
        assert_eq!(cfg.trader.slippage.exit_loss_shortfall_pct, 7.5);
        assert_eq!(cfg.chains.solana.swaps.direct.max_price_impact_pct, 4.5);
        assert_eq!(cfg.chains.solana.swaps.raptor.max_hops, 2);
        assert!(!cfg.chains.solana.swaps.cost_guard.enabled);
    });
}
