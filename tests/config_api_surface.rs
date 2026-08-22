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

use screenerbot::config::metadata::collect_config_metadata;
use screenerbot::config::schemas::Config;
use screenerbot::config::updates::update_config_section;
use screenerbot::config::utils::{load_config_from_path, save_config_to_file, with_config};
use screenerbot::webserver::routes::config::getters::get_full_config;
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

/// One global config per test binary — `load_config_from_path` uses a `OnceLock`.
/// Seeds the webserver auth secrets so the sanitisation test is not vacuous.
fn init_config() {
    INIT.call_once(init_config_once);
}

fn init_config_once() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
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
