// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! One-time migration of the legacy `[ai]` and `[agents]` TOML sections into
//! the canonical `[llm]`, `[llm_analysis]`, `[assistant]` and `[agent_control]`
//! sections.
//!
//! Runs on both the initial load and the hot-reload path. Every legacy value is
//! preserved. When a destination field is *explicitly present* in the new
//! section it wins; a legacy value only fills a destination the file left unset.
//! After migration the caller persists the canonical config with the legacy
//! tables removed, so the migration is one-time.
//!
//! The same module relocates the Solana settings that once lived in global
//! sections (`[rpc]`, `[swaps]`, `[sol_price]` and the Jupiter/Raptor monitors
//! under `[connectivity.endpoints]`) to `[chains.solana]`, and slippage to
//! `[trader.slippage]`. That relocation runs on the untyped document before the
//! typed parse, so keys the schema no longer knows move with their subtree and
//! are dropped by the first persist. A canonical leaf present in the document
//! always wins over its legacy counterpart.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::schemas::Config;
use super::{Error, Result};

/// Legacy `[ai]` section. Every field is optional so an absent key never
/// overwrites a destination with a stale default — only values the file
/// actually carried are carried forward.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyAi {
    // Master control -> llm. Provider credentials are migrated leaf-by-leaf
    // directly from the raw TOML (see `migrate_provider_leaves`) rather than
    // through a typed field here, so a partially-present canonical
    // `[llm.providers.*]` table cannot suppress unrelated legacy providers or
    // unrelated legacy leaves of the same provider.
    enabled: Option<bool>,
    default_provider: Option<String>,

    // Filtering / trading / blacklist / background / limits -> llm_analysis
    filtering_enabled: Option<bool>,
    filtering_min_confidence: Option<u8>,
    filtering_fallback_pass: Option<bool>,
    filtering_use_cache: Option<bool>,
    entry_analysis_enabled: Option<bool>,
    exit_analysis_enabled: Option<bool>,
    ai_trailing_stop_enabled: Option<bool>,
    trading_bypass_cache: Option<bool>,
    auto_blacklist_enabled: Option<bool>,
    auto_blacklist_min_confidence: Option<u8>,
    background_check_enabled: Option<bool>,
    background_check_interval_seconds: Option<u64>,
    background_batch_size: Option<u32>,
    max_evaluations_per_minute: Option<u32>,
    cache_ttl_seconds: Option<u64>,

    // Chat / automation -> assistant
    chat_enabled: Option<bool>,
    chat_max_session_messages: Option<u32>,
    chat_auto_summarize: Option<bool>,
    event_triggers_enabled: Option<bool>,
    scheduled_tasks_enabled: Option<bool>,
    scheduled_tasks_check_interval_seconds: Option<u64>,
    scheduled_tasks_max_concurrent: Option<u32>,
    scheduled_tasks_default_timeout_seconds: Option<u64>,

    // Tool permissions -> agent_control
    tool_permissions_analysis: Option<String>,
    tool_permissions_portfolio: Option<String>,
    tool_permissions_trading: Option<String>,
    tool_permissions_config: Option<String>,
    tool_permissions_system: Option<String>,
}

/// Legacy `[agents]` section.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyAgents {
    enabled: Option<bool>,
}

/// Was `[<section>] <field>` explicitly written in the file?
fn has_key(root: &toml::Table, section: &str, field: &str) -> bool {
    root.get(section)
        .and_then(toml::Value::as_table)
        .map(|t| t.contains_key(field))
        .unwrap_or(false)
}

/// Resolve a dotted key path (e.g. `["llm", "providers", "openai", "api_key"]`)
/// to the value the file actually carried, or `None` when any segment is absent
/// or not a table. Used to compare a legacy leaf against its canonical
/// counterpart one field at a time.
fn nested_leaf<'a>(root: &'a toml::Table, path: &[&str]) -> Option<&'a toml::Value> {
    let (first, rest) = path.split_first()?;
    let mut current = root.get(*first)?;
    for segment in rest {
        current = current.as_table()?.get(*segment)?;
    }
    Some(current)
}

/// Every provider whose credentials move from `[ai.providers.*]` to
/// `[llm.providers.*]`. The legacy and canonical key are identical, so one name
/// each. `base_url` is Ollama-only; the API-keyed providers carry `api_key`
/// instead. Both cases share `enabled`, `model` and `rate_limit_per_minute`.
const API_KEYED_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "groq",
    "deepseek",
    "gemini",
    "together",
    "openrouter",
    "mistral",
];

/// Migrate the credential leaves of one provider from `[ai.providers.<name>]`
/// into `config.llm.providers.<name>`, field by field. An explicitly present
/// canonical leaf always wins; a legacy leaf only fills a canonical leaf the
/// file left unset. Secrets are copied by value and never logged.
fn migrate_provider_leaves(root: &toml::Table, name: &str, config: &mut Config) -> Result<()> {
    // `bool` leaf: `enabled`.
    if let Some(legacy) = nested_leaf(root, &["ai", "providers", name, "enabled"]) {
        if nested_leaf(root, &["llm", "providers", name, "enabled"]).is_none() {
            let value = legacy.as_bool().ok_or_else(|| Error::ParseFailed {
                detail: format!("legacy [ai.providers.{name}].enabled must be a boolean"),
            })?;
            set_provider_bool(config, name, "enabled", value);
        }
    }

    // `string` leaves: `api_key` / `base_url` (mutually exclusive) and `model`.
    let string_leaves: &[&str] = if name == "ollama" {
        &["base_url", "model"]
    } else {
        &["api_key", "model"]
    };
    for leaf in string_leaves {
        if let Some(legacy) = nested_leaf(root, &["ai", "providers", name, leaf]) {
            if nested_leaf(root, &["llm", "providers", name, leaf]).is_none() {
                let value = legacy.as_str().ok_or_else(|| Error::ParseFailed {
                    detail: format!("legacy [ai.providers.{name}].{leaf} must be a string"),
                })?;
                set_provider_string(config, name, leaf, value.to_owned());
            }
        }
    }

    // `u32` leaf: `rate_limit_per_minute`.
    if let Some(legacy) = nested_leaf(root, &["ai", "providers", name, "rate_limit_per_minute"]) {
        if nested_leaf(root, &["llm", "providers", name, "rate_limit_per_minute"]).is_none() {
            let raw = legacy.as_integer().ok_or_else(|| Error::ParseFailed {
                detail: format!(
                    "legacy [ai.providers.{name}].rate_limit_per_minute must be an integer"
                ),
            })?;
            let value = u32::try_from(raw).map_err(|_| Error::ParseFailed {
                detail: format!(
                    "legacy [ai.providers.{name}].rate_limit_per_minute is out of range for u32"
                ),
            })?;
            set_provider_u32(config, name, "rate_limit_per_minute", value);
        }
    }

    Ok(())
}

/// Dispatch a `bool` provider leaf to the concrete field. A key not present in
/// the match is silently ignored, which cannot happen because callers only pass
/// leaves this migration owns.
fn set_provider_bool(config: &mut Config, name: &str, leaf: &str, value: bool) {
    let providers = &mut config.llm.providers;
    match (name, leaf) {
        ("openai", "enabled") => providers.openai.enabled = value,
        ("anthropic", "enabled") => providers.anthropic.enabled = value,
        ("groq", "enabled") => providers.groq.enabled = value,
        ("deepseek", "enabled") => providers.deepseek.enabled = value,
        ("gemini", "enabled") => providers.gemini.enabled = value,
        ("together", "enabled") => providers.together.enabled = value,
        ("openrouter", "enabled") => providers.openrouter.enabled = value,
        ("mistral", "enabled") => providers.mistral.enabled = value,
        ("ollama", "enabled") => providers.ollama.enabled = value,
        _ => {}
    }
}

/// Dispatch a `string` provider leaf (`api_key`, `base_url`, `model`) to the
/// concrete field.
fn set_provider_string(config: &mut Config, name: &str, leaf: &str, value: String) {
    let providers = &mut config.llm.providers;
    match (name, leaf) {
        ("openai", "api_key") => providers.openai.api_key = value,
        ("openai", "model") => providers.openai.model = value,
        ("anthropic", "api_key") => providers.anthropic.api_key = value,
        ("anthropic", "model") => providers.anthropic.model = value,
        ("groq", "api_key") => providers.groq.api_key = value,
        ("groq", "model") => providers.groq.model = value,
        ("deepseek", "api_key") => providers.deepseek.api_key = value,
        ("deepseek", "model") => providers.deepseek.model = value,
        ("gemini", "api_key") => providers.gemini.api_key = value,
        ("gemini", "model") => providers.gemini.model = value,
        ("together", "api_key") => providers.together.api_key = value,
        ("together", "model") => providers.together.model = value,
        ("openrouter", "api_key") => providers.openrouter.api_key = value,
        ("openrouter", "model") => providers.openrouter.model = value,
        ("mistral", "api_key") => providers.mistral.api_key = value,
        ("mistral", "model") => providers.mistral.model = value,
        ("ollama", "base_url") => providers.ollama.base_url = value,
        ("ollama", "model") => providers.ollama.model = value,
        _ => {}
    }
}

/// Dispatch the `rate_limit_per_minute` provider leaf to the concrete field.
fn set_provider_u32(config: &mut Config, name: &str, leaf: &str, value: u32) {
    let providers = &mut config.llm.providers;
    match (name, leaf) {
        ("openai", "rate_limit_per_minute") => providers.openai.rate_limit_per_minute = value,
        ("anthropic", "rate_limit_per_minute") => providers.anthropic.rate_limit_per_minute = value,
        ("groq", "rate_limit_per_minute") => providers.groq.rate_limit_per_minute = value,
        ("deepseek", "rate_limit_per_minute") => providers.deepseek.rate_limit_per_minute = value,
        ("gemini", "rate_limit_per_minute") => providers.gemini.rate_limit_per_minute = value,
        ("together", "rate_limit_per_minute") => providers.together.rate_limit_per_minute = value,
        ("openrouter", "rate_limit_per_minute") => {
            providers.openrouter.rate_limit_per_minute = value
        }
        ("mistral", "rate_limit_per_minute") => providers.mistral.rate_limit_per_minute = value,
        ("ollama", "rate_limit_per_minute") => providers.ollama.rate_limit_per_minute = value,
        _ => {}
    }
}

/// Apply the legacy `[ai]` / `[agents]` sections found in `raw` onto `config`.
///
/// `config` has already been parsed from the file, so any explicitly present
/// canonical field is in place; this only backfills the ones the file omitted.
/// Returns `true` when a legacy section was present and consumed (the caller
/// must then persist the canonical config to make the migration permanent).
pub(super) fn migrate_legacy_sections(raw: &str, config: &mut Config) -> Result<bool> {
    let root: toml::Table = toml::from_str(raw).map_err(|e| Error::ParseFailed {
        detail: format!("could not re-parse config for legacy migration: {e}"),
    })?;

    let legacy_ai_value = root.get("ai").cloned();
    let legacy_agents_value = root.get("agents").cloned();
    if legacy_ai_value.is_none() && legacy_agents_value.is_none() {
        return Ok(false);
    }

    if let Some(value) = legacy_ai_value {
        let legacy: LegacyAi = value.try_into().map_err(|e| Error::ParseFailed {
            detail: format!("legacy [ai] section is malformed: {e}"),
        })?;

        // --- llm ---
        if let Some(v) = legacy.enabled {
            if !has_key(&root, "llm", "enabled") {
                config.llm.enabled = v;
            }
        }
        if let Some(v) = legacy.default_provider {
            if !has_key(&root, "llm", "default_provider") {
                config.llm.default_provider = v;
            }
        }

        // Provider credentials: migrate every leaf of every provider on its own.
        // A canonical `[llm.providers.<name>].<leaf>` that the file wrote
        // explicitly wins; every other leaf — of this provider or any other —
        // is still filled from `[ai.providers.*]`.
        for name in API_KEYED_PROVIDERS
            .iter()
            .copied()
            .chain(std::iter::once("ollama"))
        {
            migrate_provider_leaves(&root, name, config)?;
        }

        // --- llm_analysis ---
        macro_rules! fill_analysis {
            ($opt:expr, $field:ident) => {
                if let Some(v) = $opt {
                    if !has_key(&root, "llm_analysis", stringify!($field)) {
                        config.llm_analysis.$field = v;
                    }
                }
            };
        }
        fill_analysis!(legacy.filtering_enabled, filtering_enabled);
        fill_analysis!(legacy.filtering_min_confidence, min_confidence);
        fill_analysis!(legacy.filtering_fallback_pass, fallback_pass);
        fill_analysis!(legacy.filtering_use_cache, use_cache);
        fill_analysis!(legacy.entry_analysis_enabled, entry_analysis_enabled);
        fill_analysis!(legacy.exit_analysis_enabled, exit_analysis_enabled);
        fill_analysis!(legacy.ai_trailing_stop_enabled, trailing_stop_enabled);
        fill_analysis!(legacy.trading_bypass_cache, trading_bypass_cache);
        fill_analysis!(legacy.auto_blacklist_enabled, auto_blacklist_enabled);
        fill_analysis!(
            legacy.auto_blacklist_min_confidence,
            auto_blacklist_min_confidence
        );
        fill_analysis!(legacy.background_check_enabled, background_check_enabled);
        fill_analysis!(
            legacy.background_check_interval_seconds,
            background_check_interval_seconds
        );
        fill_analysis!(legacy.background_batch_size, background_batch_size);
        fill_analysis!(
            legacy.max_evaluations_per_minute,
            max_evaluations_per_minute
        );
        fill_analysis!(legacy.cache_ttl_seconds, cache_ttl_seconds);

        // --- assistant ---
        macro_rules! fill_assistant {
            ($opt:expr, $field:ident) => {
                if let Some(v) = $opt {
                    if !has_key(&root, "assistant", stringify!($field)) {
                        config.assistant.$field = v;
                    }
                }
            };
        }
        fill_assistant!(legacy.chat_enabled, enabled);
        fill_assistant!(legacy.chat_max_session_messages, max_session_messages);
        fill_assistant!(legacy.chat_auto_summarize, auto_summarize);
        fill_assistant!(legacy.event_triggers_enabled, event_triggers_enabled);
        fill_assistant!(legacy.scheduled_tasks_enabled, scheduled_tasks_enabled);
        fill_assistant!(
            legacy.scheduled_tasks_check_interval_seconds,
            check_interval_seconds
        );
        fill_assistant!(legacy.scheduled_tasks_max_concurrent, max_concurrent);
        fill_assistant!(
            legacy.scheduled_tasks_default_timeout_seconds,
            default_timeout_seconds
        );

        // --- agent_control (tool permissions from [ai]) ---
        macro_rules! fill_agent {
            ($opt:expr, $field:ident) => {
                if let Some(v) = $opt {
                    if !has_key(&root, "agent_control", stringify!($field)) {
                        config.agent_control.$field = v;
                    }
                }
            };
        }
        fill_agent!(legacy.tool_permissions_analysis, analysis);
        fill_agent!(legacy.tool_permissions_portfolio, portfolio);
        fill_agent!(legacy.tool_permissions_trading, trading);
        fill_agent!(legacy.tool_permissions_config, config);
        fill_agent!(legacy.tool_permissions_system, system);
    }

    if let Some(value) = legacy_agents_value {
        let legacy: LegacyAgents = value.try_into().map_err(|e| Error::ParseFailed {
            detail: format!("legacy [agents] section is malformed: {e}"),
        })?;
        if let Some(v) = legacy.enabled {
            if !has_key(&root, "agent_control", "enabled") {
                config.agent_control.enabled = v;
            }
        }
    }

    Ok(true)
}

/// One relocation of a legacy subtree. `to` names the destination table,
/// created when absent; `None` drops the subtree.
struct Relocation {
    from: &'static [&'static str],
    to: Option<&'static [&'static str]>,
}

/// The `[chains.<id>]` table key that receives the legacy global sections.
const LEGACY_CHAIN_KEY: &str = crate::chains::PRE_CHAINS_LAYOUT_CHAIN.as_str();

/// Order matters: slippage leaves `swaps` before the rest of `swaps` moves.
const CHAIN_RELOCATIONS: &[Relocation] = &[
    Relocation {
        from: &["swaps", "slippage"],
        to: Some(&["trader", "slippage"]),
    },
    Relocation {
        from: &["swaps"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "swaps"]),
    },
    Relocation {
        from: &["rpc"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "rpc"]),
    },
    Relocation {
        from: &["connectivity", "endpoints", "jupiter"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "connectivity", "jupiter"]),
    },
    Relocation {
        from: &["connectivity", "endpoints", "raptor"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "connectivity", "raptor"]),
    },
    Relocation {
        from: &["sol_price"],
        to: None,
    },
];

/// The node at `path`, or `None` when a segment is absent or a parent on the
/// way is not a table.
fn node_at<'a>(doc: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(doc, |node, segment| node.as_object()?.get(*segment))
}

/// `path` written the way the config file names a table, e.g. `[chains.solana]`.
fn table_name(path: &[&str]) -> String {
    format!("[{}]", path.join("."))
}

/// True when the document still carries a section that moved under
/// `[chains.<id>]` or `[trader.slippage]`.
pub(crate) fn has_legacy_chain_sections(doc: &Value) -> bool {
    CHAIN_RELOCATIONS
        .iter()
        .any(|relocation| node_at(doc, relocation.from).is_some())
}

/// Move every legacy subtree in `doc` to its canonical path, leaf by leaf.
/// A leaf already present at the destination wins; a legacy leaf fills only
/// what the destination lacks. Returns `true` when anything moved or was
/// dropped. Fails without modifying `doc` when a legacy subtree or a
/// destination table has the wrong shape.
pub(crate) fn relocate_legacy_chain_sections(doc: &mut Value) -> Result<bool> {
    let mut working = doc.clone();
    let mut changed = false;
    for relocation in CHAIN_RELOCATIONS {
        let Some(legacy) = take_node(&mut working, relocation.from) else {
            continue;
        };
        let source = table_name(relocation.from);
        let Value::Object(legacy) = legacy else {
            return Err(Error::ParseFailed {
                detail: format!("legacy {source} must be a table"),
            });
        };
        if let Some(to) = relocation.to {
            let destination = table_at(&mut working, to, &source)?;
            merge_missing_leaves(destination, legacy);
        }
        changed = true;
    }
    if changed {
        *doc = working;
    }
    Ok(changed)
}

/// Remove and return the node at `path`; `None` when it is absent.
fn take_node(doc: &mut Value, path: &[&str]) -> Option<Value> {
    let (last, parents) = path.split_last()?;
    let mut node = doc;
    for segment in parents {
        node = node.as_object_mut()?.get_mut(*segment)?;
    }
    node.as_object_mut()?.remove(*last)
}

/// The table at `path`, creating every missing table on the way. A node on the
/// way that exists but is not a table refuses the relocation of `source`.
fn table_at<'a>(
    doc: &'a mut Value,
    path: &[&str],
    source: &str,
) -> Result<&'a mut Map<String, Value>> {
    let refused = |depth: usize| Error::ParseFailed {
        detail: if depth == 0 {
            format!("the config document must be a table to receive legacy {source}")
        } else {
            format!(
                "{} must be a table to receive legacy {source}",
                table_name(&path[..depth])
            )
        },
    };
    let mut node = doc;
    for (depth, segment) in path.iter().enumerate() {
        node = node
            .as_object_mut()
            .ok_or_else(|| refused(depth))?
            .entry((*segment).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    node.as_object_mut().ok_or_else(|| refused(path.len()))
}

/// Fill `destination` from `legacy`: tables merge key by key, recursively; a
/// key the destination already holds keeps the destination's value; any
/// other key takes the legacy value. Arrays are leaves.
fn merge_missing_leaves(destination: &mut Map<String, Value>, legacy: Map<String, Value>) {
    for (key, legacy_value) in legacy {
        match destination.get_mut(&key) {
            Some(Value::Object(existing)) => {
                if let Value::Object(legacy_table) = legacy_value {
                    merge_missing_leaves(existing, legacy_table);
                }
            }
            Some(_) => {}
            None => {
                destination.insert(key, legacy_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate(raw: &str) -> (Config, bool) {
        let mut config: Config = toml::from_str(raw).expect("base parse");
        let changed = migrate_legacy_sections(raw, &mut config).expect("migration");
        (config, changed)
    }

    fn json<T: serde::Serialize>(v: &T) -> serde_json::Value {
        serde_json::to_value(v).unwrap()
    }

    #[test]
    fn absent_legacy_sections_are_noop() {
        let (config, changed) = migrate("[trader]\nmax_open_positions = 5\n");
        assert!(!changed);
        let defaults = Config::default();
        assert_eq!(json(&config.llm), json(&defaults.llm));
        assert_eq!(json(&config.agent_control), json(&defaults.agent_control));
    }

    #[test]
    fn field_mappings_and_secrets_are_preserved() {
        let raw = r#"
[ai]
enabled = true
default_provider = "anthropic"
filtering_enabled = true
filtering_min_confidence = 55
filtering_fallback_pass = true
filtering_use_cache = false
entry_analysis_enabled = true
exit_analysis_enabled = true
ai_trailing_stop_enabled = true
trading_bypass_cache = false
auto_blacklist_enabled = true
auto_blacklist_min_confidence = 77
background_check_enabled = true
background_check_interval_seconds = 900
background_batch_size = 9
max_evaluations_per_minute = 42
cache_ttl_seconds = 1200
chat_enabled = true
chat_max_session_messages = 250
chat_auto_summarize = false
event_triggers_enabled = true
scheduled_tasks_enabled = true
scheduled_tasks_check_interval_seconds = 45
scheduled_tasks_max_concurrent = 3
scheduled_tasks_default_timeout_seconds = 300
tool_permissions_analysis = "deny"
tool_permissions_portfolio = "ask_user"
tool_permissions_trading = "deny"
tool_permissions_config = "allow"
tool_permissions_system = "deny"

[ai.providers.openai]
enabled = true
api_key = "sk-legacy-secret"
model = "gpt-4o"
rate_limit_per_minute = 33

[ai.providers.ollama]
enabled = true
model = "llama3.1"
base_url = "http://127.0.0.1:9999"

[agents]
enabled = false
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);

        assert!(config.llm.enabled);
        assert_eq!(config.llm.default_provider, "anthropic");
        assert!(config.llm.providers.openai.enabled);
        assert_eq!(config.llm.providers.openai.api_key, "sk-legacy-secret");
        assert_eq!(config.llm.providers.openai.model, "gpt-4o");
        assert_eq!(config.llm.providers.openai.rate_limit_per_minute, 33);
        assert!(config.llm.providers.ollama.enabled);
        assert_eq!(
            config.llm.providers.ollama.base_url,
            "http://127.0.0.1:9999"
        );

        assert!(config.llm_analysis.filtering_enabled);
        assert_eq!(config.llm_analysis.min_confidence, 55);
        assert!(config.llm_analysis.fallback_pass);
        assert!(!config.llm_analysis.use_cache);
        assert!(config.llm_analysis.entry_analysis_enabled);
        assert!(config.llm_analysis.exit_analysis_enabled);
        assert!(config.llm_analysis.trailing_stop_enabled);
        assert!(!config.llm_analysis.trading_bypass_cache);
        assert!(config.llm_analysis.auto_blacklist_enabled);
        assert_eq!(config.llm_analysis.auto_blacklist_min_confidence, 77);
        assert!(config.llm_analysis.background_check_enabled);
        assert_eq!(config.llm_analysis.background_check_interval_seconds, 900);
        assert_eq!(config.llm_analysis.background_batch_size, 9);
        assert_eq!(config.llm_analysis.max_evaluations_per_minute, 42);
        assert_eq!(config.llm_analysis.cache_ttl_seconds, 1200);

        assert!(config.assistant.enabled);
        assert_eq!(config.assistant.max_session_messages, 250);
        assert!(!config.assistant.auto_summarize);
        assert!(config.assistant.event_triggers_enabled);
        assert!(config.assistant.scheduled_tasks_enabled);
        assert_eq!(config.assistant.check_interval_seconds, 45);
        assert_eq!(config.assistant.max_concurrent, 3);
        assert_eq!(config.assistant.default_timeout_seconds, 300);

        assert!(!config.agent_control.enabled);
        assert_eq!(config.agent_control.analysis, "deny");
        assert_eq!(config.agent_control.portfolio, "ask_user");
        assert_eq!(config.agent_control.trading, "deny");
        assert_eq!(config.agent_control.config, "allow");
        assert_eq!(config.agent_control.system, "deny");
    }

    #[test]
    fn explicit_new_destination_wins_field_by_field() {
        let raw = r#"
[ai]
enabled = true
default_provider = "groq"
cache_ttl_seconds = 999

[llm]
default_provider = "openai"

[llm_analysis]
cache_ttl_seconds = 120
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);
        // legacy fills the destination the file left unset
        assert!(config.llm.enabled);
        // explicit new destination wins over legacy
        assert_eq!(config.llm.default_provider, "openai");
        assert_eq!(config.llm_analysis.cache_ttl_seconds, 120);
    }

    #[test]
    fn migration_is_idempotent() {
        let raw = r#"
[ai]
enabled = true
default_provider = "mistral"
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);

        // Re-serialize the canonical config (legacy tables gone) and re-run.
        let round_tripped = toml::to_string_pretty(&config).unwrap();
        assert!(!round_tripped.contains("\n[ai]"));
        assert!(!round_tripped.contains("[agents]"));
        let (config2, changed2) = migrate(&round_tripped);
        assert!(!changed2);
        assert_eq!(json(&config2.llm), json(&config.llm));
    }

    #[test]
    fn malformed_legacy_value_is_a_typed_error() {
        let raw = "[ai]\nenabled = \"not-a-bool\"\n";
        let mut config: Config = toml::from_str(raw).expect("base parse ignores unknown [ai]");
        let err = migrate_legacy_sections(raw, &mut config).unwrap_err();
        assert!(matches!(err, Error::ParseFailed { .. }));
    }

    /// A brand-new `[llm.providers.openai]` entry must not stop the legacy
    /// `[ai.providers.anthropic]` / `groq` credentials from migrating.
    #[test]
    fn new_provider_does_not_suppress_other_legacy_providers() {
        let raw = r#"
[ai.providers.anthropic]
enabled = true
api_key = "sk-ant-legacy"
model = "claude-3-5-sonnet"
rate_limit_per_minute = 25

[ai.providers.groq]
enabled = true
api_key = "gsk-legacy"

[llm.providers.openai]
enabled = true
api_key = "sk-openai-new"
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);

        // The explicitly-written new provider is untouched by the migration.
        assert!(config.llm.providers.openai.enabled);
        assert_eq!(config.llm.providers.openai.api_key, "sk-openai-new");

        // Every legacy provider still lands, field for field.
        assert!(config.llm.providers.anthropic.enabled);
        assert_eq!(config.llm.providers.anthropic.api_key, "sk-ant-legacy");
        assert_eq!(config.llm.providers.anthropic.model, "claude-3-5-sonnet");
        assert_eq!(config.llm.providers.anthropic.rate_limit_per_minute, 25);
        assert!(config.llm.providers.groq.enabled);
        assert_eq!(config.llm.providers.groq.api_key, "gsk-legacy");
    }

    /// Within one provider, an explicit canonical leaf wins and every other
    /// leaf of the same provider is still backfilled from legacy.
    #[test]
    fn same_provider_leaf_precedence() {
        let raw = r#"
[ai.providers.openai]
enabled = true
api_key = "sk-legacy"
model = "gpt-4o"
rate_limit_per_minute = 33

[llm.providers.openai]
model = "gpt-4o-mini"
"#;
        let (config, _) = migrate(raw);
        // Explicit canonical leaf wins.
        assert_eq!(config.llm.providers.openai.model, "gpt-4o-mini");
        // Untouched leaves fill from legacy, including the secret.
        assert!(config.llm.providers.openai.enabled);
        assert_eq!(config.llm.providers.openai.api_key, "sk-legacy");
        assert_eq!(config.llm.providers.openai.rate_limit_per_minute, 33);
    }

    /// A canonical leaf for one provider does not touch a different provider.
    #[test]
    fn unrelated_provider_is_preserved() {
        let raw = r#"
[ai.providers.mistral]
enabled = true
api_key = "sk-mistral-legacy"

[llm.providers.gemini]
api_key = "sk-gemini-new"
"#;
        let (config, _) = migrate(raw);
        assert_eq!(config.llm.providers.gemini.api_key, "sk-gemini-new");
        assert!(config.llm.providers.mistral.enabled);
        assert_eq!(config.llm.providers.mistral.api_key, "sk-mistral-legacy");
        // A provider named in neither table keeps its schema default.
        assert!(!config.llm.providers.deepseek.enabled);
        assert_eq!(config.llm.providers.deepseek.api_key, "");
    }

    /// Ollama migrates `base_url` (not `api_key`), and an explicit canonical
    /// Ollama leaf wins while the rest fill from legacy.
    #[test]
    fn ollama_leaf_precedence() {
        let raw = r#"
[ai.providers.ollama]
enabled = true
model = "llama3.1"
base_url = "http://127.0.0.1:9999"
rate_limit_per_minute = 200

[llm.providers.ollama]
base_url = "http://10.0.0.5:11434"
"#;
        let (config, _) = migrate(raw);
        assert_eq!(
            config.llm.providers.ollama.base_url,
            "http://10.0.0.5:11434"
        );
        assert!(config.llm.providers.ollama.enabled);
        assert_eq!(config.llm.providers.ollama.model, "llama3.1");
        assert_eq!(config.llm.providers.ollama.rate_limit_per_minute, 200);
    }

    /// Re-running the migration on the already-canonical output changes nothing
    /// and reports no legacy section.
    #[test]
    fn provider_migration_is_idempotent() {
        let raw = r#"
[ai.providers.openai]
enabled = true
api_key = "sk-legacy"
model = "gpt-4o"
rate_limit_per_minute = 33

[ai.providers.ollama]
base_url = "http://127.0.0.1:9999"
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);

        let round_tripped = toml::to_string_pretty(&config).unwrap();
        let (config2, changed2) = migrate(&round_tripped);
        assert!(!changed2);
        assert_eq!(json(&config2.llm), json(&config.llm));
    }

    /// A legacy provider leaf of the wrong TOML type is a typed error, not a
    /// silent default.
    #[test]
    fn malformed_provider_leaf_is_a_typed_error() {
        for raw in [
            "[ai.providers.openai]\nenabled = \"yes\"\n",
            "[ai.providers.openai]\napi_key = 12345\n",
            "[ai.providers.openai]\nrate_limit_per_minute = \"fast\"\n",
            "[ai.providers.openai]\nrate_limit_per_minute = -5\n",
        ] {
            let mut config: Config = toml::from_str(raw).expect("base parse");
            let err = migrate_legacy_sections(raw, &mut config).unwrap_err();
            assert!(
                matches!(err, Error::ParseFailed { .. }),
                "expected ParseFailed for {raw:?}"
            );
        }
    }

    /// The persisted canonical config carries no `[ai]` / `[agents]` tables and
    /// no secret is echoed into an error or a log line by the migration itself.
    #[test]
    fn canonical_serialization_drops_legacy_tables() {
        let raw = r#"
[ai]
enabled = true

[ai.providers.anthropic]
api_key = "sk-ant-secret"
"#;
        let (config, changed) = migrate(raw);
        assert!(changed);
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(!serialized.contains("[ai]"));
        assert!(!serialized.contains("[ai.providers"));
        assert!(!serialized.contains("[agents]"));
        // The secret survives in the canonical location.
        assert!(serialized.contains("sk-ant-secret"));
        assert_eq!(config.llm.providers.anthropic.api_key, "sk-ant-secret");
    }

    // ------------------------------------------------------------------
    // Relocation of the legacy chain sections
    // ------------------------------------------------------------------

    const V0_2_13_CONFIG: &str = include_str!("../../tests/fixtures/v0.2.13-config.toml");

    fn parse(raw: &str) -> super::super::utils::ParsedConfig {
        super::super::utils::parse_config_document(raw).expect("parse")
    }

    fn table(config: &Config) -> toml::Table {
        toml::from_str(&toml::to_string(config).expect("serialize")).expect("reparse")
    }

    #[test]
    fn v0_2_13_layout_relocates_every_moved_value() {
        let parsed = parse(V0_2_13_CONFIG);
        assert!(parsed.migrated);
        let config = &parsed.config;

        let rpc = &config.chains.solana.rpc;
        assert_eq!(
            rpc.urls,
            vec![
                "https://rpc-one.fixture.invalid/".to_owned(),
                "https://rpc-two.fixture.invalid/".to_owned()
            ]
        );
        assert_eq!(rpc.selection_strategy, "round_robin");
        assert_eq!(rpc.default_rate_limit, 12);
        assert_eq!(rpc.helius_rate_limit, 61);
        assert_eq!(rpc.quicknode_rate_limit, 27);
        assert_eq!(rpc.triton_rate_limit, 111);
        assert_eq!(rpc.public_rate_limit, 3);
        assert!(!rpc.circuit_breaker_enabled);
        assert_eq!(rpc.circuit_breaker_failure_threshold, 7);
        assert_eq!(rpc.circuit_breaker_success_threshold, 4);
        assert_eq!(rpc.circuit_breaker_open_duration_secs, 45);
        assert_eq!(rpc.circuit_breaker_half_open_requests, 2);
        assert_eq!(rpc.request_timeout_secs, 25);
        assert_eq!(rpc.connection_timeout_secs, 12);
        assert_eq!(rpc.max_retries, 5);
        assert_eq!(rpc.retry_base_delay_ms, 150);
        assert_eq!(rpc.retry_max_delay_ms, 4000);
        assert_eq!(rpc.pool_connections_per_host, 6);
        assert_eq!(rpc.pool_idle_timeout_secs, 60);
        assert!(!rpc.stats_enabled);

        let swaps = &config.chains.solana.swaps;
        assert!(!swaps.jupiter.enabled);
        assert!(!swaps.jupiter.dynamic_compute_unit_limit);
        assert_eq!(swaps.jupiter.priority_fee_micro_lamports, 75_000);
        assert_eq!(swaps.jupiter.api_key, "fixture-jupiter-key");
        assert!(swaps.direct.enabled);
        assert_eq!(swaps.direct.priority_fee_micro_lamports, 80_000);
        assert_eq!(swaps.direct.confirmation_timeout_secs, 90);
        assert_eq!(swaps.direct.max_price_impact_pct, 4.5);
        assert!(swaps.raptor.enabled);
        assert_eq!(swaps.raptor.priority_fee_micro_lamports, 65_000);
        assert_eq!(swaps.raptor.max_hops, 2);
        assert!(!swaps.cost_guard.enabled);
        assert_eq!(swaps.cost_guard.max_extra_cost_pct, 2.5);
        assert_eq!(swaps.cost_guard.always_allow_below_lamports, 250_000);
        assert!(!swaps.cost_guard.retry_excluding_venue);

        let slippage = &config.trader.slippage;
        assert_eq!(slippage.quote_default_pct, 2.5);
        assert_eq!(slippage.exit_profit_shortfall_pct, 4.0);
        assert_eq!(slippage.exit_loss_shortfall_pct, 7.5);
        assert_eq!(slippage.exit_retry_steps_pct, vec![5.0, 12.0, 30.0]);

        let monitors = &config.chains.solana.connectivity;
        assert!(!monitors.jupiter.enabled);
        assert_eq!(monitors.jupiter.timeout_secs, 7);
        assert!(!monitors.raptor.enabled);
        assert_eq!(monitors.raptor.timeout_secs, 8);
        let endpoints = &config.connectivity.endpoints;
        assert!(!endpoints.rpc.enabled);
        assert_eq!(endpoints.rpc.timeout_secs, 6);
        assert_eq!(endpoints.rugcheck.timeout_secs, 12);

        assert!(config.pools.enable_raydium_discovery);

        // Trading amounts keep their sections.
        assert_eq!(config.trader.max_open_positions, 4);
        assert_eq!(config.trader.trade_size_sol, 0.02);
        assert_eq!(config.trader.entry_sizes, vec![0.01, 0.03]);
        assert_eq!(config.trader.loss_limit_sol, 0.4);
        assert_eq!(config.positions.profit_extra_needed_sol, 0.0005);
        assert_eq!(config.wallet.min_balance_sol, 0.03);
        assert_eq!(config.telegram.trade_alert_min_sol, 0.25);
        assert_eq!(config.telegram.significant_pnl_threshold, 0.8);

        assert!(config.chains.solana.enabled);
        assert!(!config.chains.show_preview);
        assert!(super::super::utils::validate_config(config).is_ok());
    }

    #[test]
    fn v0_2_13_layout_serializes_without_legacy_sections_and_is_idempotent() {
        let first = parse(V0_2_13_CONFIG);
        let serialized = toml::to_string(&first.config).expect("serialize");
        let written: toml::Table = toml::from_str(&serialized).expect("reparse");

        for section in ["rpc", "swaps", "sol_price"] {
            assert!(!written.contains_key(section), "[{section}] persisted");
        }
        let endpoints = written["connectivity"]["endpoints"]
            .as_table()
            .expect("endpoints table");
        assert!(!endpoints.contains_key("jupiter"));
        assert!(!endpoints.contains_key("raptor"));
        let rpc = written["chains"]["solana"]["rpc"]
            .as_table()
            .expect("rpc table");
        for dead in [
            "rate_limit_burst_factor",
            "stats_retention_days",
            "stats_minute_buckets",
            "debug_rpc",
        ] {
            assert!(!rpc.contains_key(dead), "rpc.{dead} persisted");
        }
        let jupiter = written["chains"]["solana"]["swaps"]["jupiter"]
            .as_table()
            .expect("jupiter table");
        assert!(!jupiter.contains_key("default_swap_mode"));
        let pools = written["pools"].as_table().expect("pools table");
        assert!(!pools.contains_key("account_batch_size"));

        let second = parse(&serialized);
        assert!(!second.migrated);
        assert_eq!(json(&second.config), json(&first.config));
    }

    #[test]
    fn canonical_leaf_wins_and_legacy_fills_the_rest() {
        let parsed = parse(
            r#"
[rpc]
max_retries = 7
urls = ["https://legacy.fixture.invalid/"]

[chains.solana.rpc]
max_retries = 9
"#,
        );
        assert!(parsed.migrated);
        assert_eq!(parsed.config.chains.solana.rpc.max_retries, 9);
        assert_eq!(
            parsed.config.chains.solana.rpc.urls,
            vec!["https://legacy.fixture.invalid/".to_owned()]
        );
    }

    #[test]
    fn partial_legacy_section_moves_only_its_leaves() {
        let parsed = parse("[swaps.jupiter]\napi_key = \"k\"\n");
        assert!(parsed.migrated);
        let swaps = &parsed.config.chains.solana.swaps;
        assert_eq!(swaps.jupiter.api_key, "k");
        let mut expected = super::super::schemas::SwapsConfig::default();
        expected.jupiter.api_key = "k".to_owned();
        assert_eq!(json(swaps), json(&expected));
    }

    #[test]
    fn misshapen_sections_are_refused_without_touching_the_document() {
        for raw in [
            "rpc = 5\n",
            "[chains]\nsolana = 5\n\n[rpc]\nmax_retries = 4\n",
        ] {
            let table: toml::Table = toml::from_str(raw).expect("toml");
            let original = serde_json::to_value(&table).expect("json");
            let mut document = original.clone();
            let err = relocate_legacy_chain_sections(&mut document).unwrap_err();
            assert!(
                matches!(err, Error::ParseFailed { .. }),
                "expected ParseFailed for {raw:?}"
            );
            assert_eq!(document, original, "document modified for {raw:?}");
            assert!(matches!(
                super::super::utils::parse_config_document(raw),
                Err(Error::ParseFailed { .. })
            ));
        }
    }

    #[test]
    fn unknown_legacy_key_loads_and_is_dropped() {
        let parsed = parse("[rpc]\nmax_retries = 4\nretired_setting = true\n");
        assert!(parsed.migrated);
        assert_eq!(parsed.config.chains.solana.rpc.max_retries, 4);
        let written = table(&parsed.config);
        let rpc = written["chains"]["solana"]["rpc"]
            .as_table()
            .expect("rpc table");
        assert!(!rpc.contains_key("retired_setting"));
    }

    #[test]
    fn canonical_document_parses_as_plain_toml() {
        let raw = r#"
[trader]
max_open_positions = 3

[trader.slippage]
quote_default_pct = 1.5

[chains.solana.rpc]
urls = ["https://canonical.fixture.invalid/"]
"#;
        let parsed = parse(raw);
        assert!(!parsed.migrated);
        let mut expected: Config = toml::from_str(raw).expect("typed parse");
        expected.gui.dashboard.navigation.tabs =
            crate::config::schemas::ensure_all_tabs_present(expected.gui.dashboard.navigation.tabs);
        assert_eq!(json(&parsed.config), json(&expected));
    }

    #[test]
    fn import_shape_relocates_to_canonical_sections() {
        let mut document = serde_json::json!({
            "rpc": { "max_retries": 6 },
            "swaps": {
                "slippage": { "quote_default_pct": 3.0 },
                "jupiter": { "enabled": false }
            },
            "sol_price": {}
        });
        assert!(relocate_legacy_chain_sections(&mut document).expect("relocate"));
        assert_eq!(
            document,
            serde_json::json!({
                "chains": {
                    "solana": {
                        "rpc": { "max_retries": 6 },
                        "swaps": { "jupiter": { "enabled": false } }
                    }
                },
                "trader": { "slippage": { "quote_default_pct": 3.0 } }
            })
        );
    }
}
