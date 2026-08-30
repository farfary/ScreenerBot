//! What a paired agent may do to configuration, and what it may never touch.
//!
//! The contract these tests pin down:
//! - An agent can read and set ANY setting the app has — including the RPC
//!   endpoint list — through dotted paths, with no hand-maintained allowlist.
//! - Wallet private-key material (`wallet_encrypted`, `wallet_nonce`) is never
//!   returned and never writable, at any permission level.
//! - A batch of changes is atomic and schema-validated: a rejected entry leaves
//!   the live configuration and `config.toml` exactly as they were.
//!
//! Its own file (own test binary → own process) because it initialises the
//! global CONFIG via `load_config_from_path` (`OnceLock::set`) and then mutates
//! it; the writes also land on a real `config.toml` in a temp data directory.

use std::sync::Once;

use screenerbot::agent_control::config_access;
use screenerbot::agent_control::{create_tool_registry, Error};
use screenerbot::config::schemas::Config;
use screenerbot::config::updates::update_config_section;
use screenerbot::config::utils::{load_config_from_path, save_config_to_file, with_config};
use serde_json::{json, Value};

static INIT: Once = Once::new();

/// A wallet secret that must never appear in anything an agent can read.
const WALLET_CIPHERTEXT: &str = "encrypted-wallet-must-not-leak";
const WALLET_NONCE: &str = "nonce-must-not-leak";

fn init_config() {
    INIT.call_once(|| {
        let dir = tempfile::tempdir().expect("temp dir");
        std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
        // `get_config_path()` resolves to <data dir>/data/config.toml.
        let data = dir.path().join("data");
        std::fs::create_dir_all(&data).expect("data dir");
        let path = data.join("config.toml");
        let path_str = path.to_str().expect("utf-8 temp path").to_owned();
        save_config_to_file(&Config::default(), &path_str, false).expect("write default config");
        load_config_from_path(&path_str).expect("load config into global");
        update_config_section(
            |cfg| {
                cfg.wallet_encrypted = WALLET_CIPHERTEXT.to_owned();
                cfg.wallet_nonce = WALLET_NONCE.to_owned();
            },
            false,
        )
        .expect("seed wallet key material");
        std::mem::forget(dir);
    });
}

fn config_toml_on_disk() -> String {
    std::fs::read_to_string(screenerbot::paths::get_config_path()).expect("config.toml on disk")
}

#[test]
fn a_full_read_never_carries_wallet_key_material() {
    init_config();
    let all = config_access::read(None).expect("read whole config");
    let rendered = serde_json::to_string(&all).expect("serialize");

    assert!(
        !rendered.contains(WALLET_CIPHERTEXT) && !rendered.contains(WALLET_NONCE),
        "wallet key material leaked into an agent-visible config read"
    );
    assert_eq!(all["wallet_encrypted"], json!(config_access::REDACTED));
    assert_eq!(all["wallet_nonce"], json!(config_access::REDACTED));

    // Addressing the secret directly is no way around it either.
    assert_eq!(
        config_access::read(Some("wallet_encrypted")).unwrap(),
        json!(config_access::REDACTED)
    );

    // The live config still holds the real value: reads are redacted, not
    // destructive.
    assert_eq!(
        with_config(|cfg| cfg.wallet_encrypted.clone()),
        WALLET_CIPHERTEXT
    );
}

#[test]
fn any_setting_is_reachable_by_path_including_nested_and_indexed_ones() {
    init_config();
    assert!(config_access::read(Some("rpc")).unwrap()["urls"].is_array());
    assert!(config_access::read(Some("rpc.urls")).unwrap().is_array());
    assert!(config_access::read(Some("rpc.urls.0")).unwrap().is_string());
    assert!(config_access::read(Some("trader.trade_size_sol"))
        .unwrap()
        .is_number());
    // A deeply nested leaf, not just top-level sections.
    assert!(config_access::read(Some("llm.providers.openai.model"))
        .unwrap()
        .is_string());

    // An unknown path names what IS available instead of failing blankly.
    let err = config_access::read(Some("rpc.not_a_field")).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("unknown config path"), "{message}");
    assert!(message.contains("urls"), "{message}");
}

#[test]
fn an_agent_can_set_the_rpc_endpoints_and_the_change_is_persisted() {
    init_config();
    let endpoints = json!([
        "https://rpc.example.test/one",
        "https://rpc.example.test/two"
    ]);

    let applied = config_access::set_one("rpc.urls", endpoints.clone()).expect("set rpc.urls");
    assert_eq!(applied.path, "rpc.urls");
    assert_eq!(applied.value, endpoints);

    assert_eq!(
        with_config(|cfg| cfg.rpc.urls.clone()),
        vec![
            "https://rpc.example.test/one".to_owned(),
            "https://rpc.example.test/two".to_owned()
        ]
    );
    // Persisted, so the endpoints survive the restart they take effect on.
    assert!(config_toml_on_disk().contains("https://rpc.example.test/one"));
}

#[test]
fn a_batch_of_changes_is_applied_atomically() {
    init_config();
    let before = with_config(|cfg| (cfg.trader.max_open_positions, cfg.rpc.max_retries));

    // One good path, one unknown path: nothing may land.
    let err = config_access::apply(&[
        ("trader.max_open_positions".to_owned(), json!(7)),
        ("rpc.no_such_setting".to_owned(), json!(1)),
    ])
    .unwrap_err();
    assert!(matches!(err, Error::InvalidParameters { .. }));
    assert_eq!(
        with_config(|cfg| (cfg.trader.max_open_positions, cfg.rpc.max_retries)),
        before,
        "a rejected batch must not apply its earlier entries"
    );

    // All-good batch lands as one change.
    config_access::apply(&[
        ("trader.max_open_positions".to_owned(), json!(7)),
        ("rpc.max_retries".to_owned(), json!(9)),
    ])
    .expect("valid batch");
    assert_eq!(
        with_config(|cfg| (cfg.trader.max_open_positions, cfg.rpc.max_retries)),
        (7, 9)
    );
}

#[test]
fn wallet_key_material_can_never_be_written() {
    init_config();
    for path in [
        "wallet_encrypted",
        "wallet_nonce",
        // A child of a secret, and the configuration root, which would clobber
        // both secrets wholesale.
        "wallet_encrypted.inner",
        "",
        "   ",
    ] {
        let err = config_access::set_one(path, json!("attacker-supplied")).unwrap_err();
        assert!(
            matches!(err, Error::SecretPath { .. }),
            "writing {path:?} must be refused as key material, got {err}"
        );
    }

    // A batch cannot smuggle one in beside a legitimate change either.
    let err = config_access::apply(&[
        ("trader.trade_size_sol".to_owned(), json!(0.5)),
        ("wallet_nonce".to_owned(), json!("smuggled")),
    ])
    .unwrap_err();
    assert!(matches!(err, Error::SecretPath { .. }));

    assert_eq!(
        with_config(|cfg| (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone())),
        (WALLET_CIPHERTEXT.to_owned(), WALLET_NONCE.to_owned())
    );
    let on_disk = config_toml_on_disk();
    assert!(!on_disk.contains("attacker-supplied") && !on_disk.contains("smuggled"));
}

#[test]
fn a_wrong_type_or_invalid_value_is_refused_before_anything_changes() {
    init_config();
    config_access::set_one("trader.trade_size_sol", json!(0.05)).expect("baseline");

    // Wrong type for the field.
    let err = config_access::set_one("trader.trade_size_sol", json!("plenty")).unwrap_err();
    assert!(
        err.to_string()
            .contains("rejected by the configuration schema"),
        "{err}"
    );

    // Right type, but rejected by `validate_config`.
    let err = config_access::set_one("trader.max_open_positions", json!(0)).unwrap_err();
    assert!(err.to_string().contains("max_open_positions"), "{err}");

    assert_eq!(with_config(|cfg| cfg.trader.trade_size_sol), 0.05);
    assert!(with_config(|cfg| cfg.trader.max_open_positions) > 0);
}

#[test]
fn the_schema_describes_what_an_agent_may_set() {
    init_config();
    let rpc = config_access::schema(Some("rpc")).expect("rpc metadata");
    assert!(rpc["urls"].is_object(), "rpc.urls must be described");
    assert!(rpc["urls"]["type"].is_string());

    let all = config_access::schema(None).expect("full metadata");
    assert!(all.as_object().expect("object").len() > 5);
    // Key material is not a settable field, so it is not in the schema at all.
    assert!(all.get("wallet_encrypted").is_none());
}

/// The same guarantees through the actual tool surface an MCP client calls.
#[tokio::test]
async fn the_config_tools_expose_reads_and_writes_without_leaking_the_wallet() {
    init_config();
    let registry = create_tool_registry();

    let get = registry
        .get("get_config")
        .expect("get_config is registered");
    let result = get.execute(json!({})).await;
    assert!(result.success, "{:?}", result.error);
    let rendered = serde_json::to_string(&result.data).unwrap();
    assert!(!rendered.contains(WALLET_CIPHERTEXT) && !rendered.contains(WALLET_NONCE));

    let update = registry
        .get("update_config")
        .expect("update_config is registered");
    let result = update
        .execute(json!({ "path": "rpc.request_timeout_secs", "value": 42 }))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert_eq!(with_config(|cfg| cfg.rpc.request_timeout_secs), 42);

    // The tool refuses key material with a message that names the reason.
    let result = update
        .execute(json!({ "path": "wallet_encrypted", "value": "nope" }))
        .await;
    assert!(!result.success);
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("wallet"),
        "{:?}",
        result.error
    );

    // Half-specified arguments are rejected rather than silently ignored.
    let result = update.execute(json!({ "path": "rpc.max_retries" })).await;
    assert!(!result.success);

    // The batch form goes through the same tool.
    let result = update
        .execute(json!({ "updates": { "rpc.max_retries": 4, "rpc.debug_rpc": true } }))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert_eq!(
        with_config(|cfg| (cfg.rpc.max_retries, cfg.rpc.debug_rpc)),
        (4, true)
    );

    let describe = registry
        .get("describe_config")
        .expect("describe_config is registered");
    let result = describe.execute(json!({ "section": "rpc" })).await;
    assert!(result.success, "{:?}", result.error);
    let schema: Value = result.data.expect("schema payload");
    assert!(schema["schema"]["urls"].is_object());
}
