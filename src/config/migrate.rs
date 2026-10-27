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
//!
//! The exit-rule leaves of `[positions]` (trailing stop and the partial
//! stop-loss share) move into `[trader]` the same way, and the auto-trader
//! partial-exit switch is retired while the stop-loss behaviour the document
//! ran with is kept. The single ROI target of `[trader]` becomes the one step of
//! the take-profit ladder, selling the whole position as it did. Every move is
//! one row of [`RELOCATIONS`], applied by [`relocate_legacy_layout`] to the TOML
//! file and to a JSON import alike.
//!
//! The same pass rewrites the stored time-override unit to its canonical name: an alias
//! or another letter case becomes the lowercase unit name, and a name that matches no
//! unit becomes hours, the unit the exit monitor has always read it as. The typed
//! schema refuses such a name on every write, so only a stored document reaches this
//! tolerance.
//!
//! A file that omits a key ran with that key's default at the time. When a
//! default changes, [`ImpliedDefaults`] writes the earlier default into a
//! document that omits the key, so an upgrade keeps the behaviour the file
//! had; only a config created after the change starts from the new default.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::schemas::{Config, TimeUnit};
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

/// The values a config document implies by omitting a key whose default has
/// changed since files were first written without it.
pub(super) struct ImpliedDefaults {
    /// `trader.stop_loss_enabled` defaulted to `false` before it defaulted to
    /// `true`; a file without the key ran with the stop loss off.
    stop_loss_enabled: Option<bool>,
}

impl ImpliedDefaults {
    /// Read from the document as written, before any typed parse fills defaults.
    pub(super) fn of(root: &toml::Table) -> Self {
        Self {
            stop_loss_enabled: (!has_key(root, "trader", "stop_loss_enabled")).then_some(false),
        }
    }

    /// Write the implied values into `config`. Returns `true` when any was
    /// written (the caller must then persist `config` to make it permanent).
    pub(super) fn apply(self, config: &mut Config) -> bool {
        let Some(enabled) = self.stop_loss_enabled else {
            return false;
        };
        config.trader.stop_loss_enabled = enabled;
        true
    }
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

/// What a relocation row moves.
#[derive(Clone, Copy)]
enum RelocationNode {
    /// A subtree, merged leaf by leaf into the destination table.
    Table,
    /// One value, stored under the last segment of `to`, which may rename it.
    Leaf,
    /// One value that became the single item of a list: stored at `to` as a
    /// one-item list, and `companion` (a sibling list read in step with it) is
    /// written as `[item]` when the list was written and the document lacks it.
    /// A number outside `bounds`, which the list's items must keep, is clamped
    /// into them: the legacy key accepted values the list refuses.
    ListOfOne {
        companion: &'static [&'static str],
        item: f64,
        bounds: (f64, f64),
    },
}

/// One relocation of a legacy node. `to` names the destination, created when
/// absent; `None` drops the node.
struct Relocation {
    from: &'static [&'static str],
    to: Option<&'static [&'static str]>,
    node: RelocationNode,
}

/// The `[chains.<id>]` table key that receives the legacy global sections.
const LEGACY_CHAIN_KEY: &str = crate::chains::PRE_CHAINS_LAYOUT_CHAIN.as_str();

/// The retired auto-trader switch that once had to be on, beside
/// [`STOP_LOSS_ALLOWANCE`], for a stop loss to sell part of a position.
const LEGACY_PARTIAL_SWITCH: &[&str] = &["positions", "partial_exit_enabled"];

/// The share of a position a partial stop loss sells; every canonical document
/// carries it, so its absence marks a document written before it existed.
const STOP_LOSS_PARTIAL_SHARE: &[&str] = &["trader", "stop_loss_partial_pct"];

/// The switch that lets a stop loss sell part of a position.
const STOP_LOSS_ALLOWANCE: &[&str] = &["trader", "stop_loss_allow_partial"];

/// Where the time-override unit is stored; [`relocate_legacy_layout`] rewrites it to
/// its canonical name.
pub(crate) const TIME_OVERRIDE_UNIT: &[&str] = &["trader", "time_override_unit"];

/// Every legacy node and where it lives now, applied in order. Order matters:
/// slippage leaves `swaps` before the rest of `swaps` moves.
const RELOCATIONS: &[Relocation] = &[
    Relocation {
        from: &["swaps", "slippage"],
        to: Some(&["trader", "slippage"]),
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["swaps"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "swaps"]),
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["rpc"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "rpc"]),
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["connectivity", "endpoints", "jupiter"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "connectivity", "jupiter"]),
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["connectivity", "endpoints", "raptor"],
        to: Some(&["chains", LEGACY_CHAIN_KEY, "connectivity", "raptor"]),
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["sol_price"],
        to: None,
        node: RelocationNode::Table,
    },
    Relocation {
        from: &["positions", "partial_exit_default_pct"],
        to: Some(STOP_LOSS_PARTIAL_SHARE),
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["positions", "trailing_stop_enabled"],
        to: Some(&["trader", "trailing_stop_enabled"]),
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["positions", "trailing_stop_activation_pct"],
        to: Some(&["trader", "trailing_stop_activation_pct"]),
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["positions", "trailing_stop_distance_pct"],
        to: Some(&["trader", "trailing_stop_distance_pct"]),
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: LEGACY_PARTIAL_SWITCH,
        to: None,
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["positions", "partial_exit_min_pct"],
        to: None,
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["positions", "partial_exit_max_pct"],
        to: None,
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["trader", "roi_exit_enabled"],
        to: Some(&["trader", "take_profit_enabled"]),
        node: RelocationNode::Leaf,
    },
    Relocation {
        from: &["trader", "roi_target_percent"],
        to: Some(&["trader", "take_profit_levels_pct"]),
        node: RelocationNode::ListOfOne {
            companion: &["trader", "take_profit_sell_pct"],
            item: super::schemas::TAKE_PROFIT_SELL_PCT_MAX,
            bounds: (
                super::schemas::TAKE_PROFIT_PROFIT_PCT_MIN,
                super::schemas::TAKE_PROFIT_PROFIT_PCT_MAX,
            ),
        },
    },
];

/// Every relocation that keeps its node, as (legacy path, canonical path), with
/// a list row's companion as a second destination of the same legacy path. A
/// config import uses it to keep the current value of a destination an older
/// export does not carry, and to follow a selected legacy section to the
/// sections its values feed.
pub(crate) fn relocation_destinations(
) -> impl Iterator<Item = (&'static [&'static str], &'static [&'static str])> {
    RELOCATIONS.iter().flat_map(|relocation| {
        let companion = match relocation.node {
            RelocationNode::ListOfOne { companion, .. } => Some(companion),
            RelocationNode::Table | RelocationNode::Leaf => None,
        };
        relocation
            .to
            .into_iter()
            .chain(companion)
            .map(|to| (relocation.from, to))
    })
}

/// An untyped config document the relocation rewrites: the TOML file at
/// load, and a JSON import at the config route. The file is relocated as TOML
/// because JSON has no `inf` or `nan`; a float holding one would not survive a
/// round trip through JSON and the typed parse would refuse the file.
pub(crate) trait ConfigTree: Clone + Sized {
    type Table: IntoIterator<Item = (String, Self)>;

    fn table(&self) -> Option<&Self::Table>;
    fn table_mut(&mut self) -> Option<&mut Self::Table>;
    fn into_table(self) -> Option<Self::Table>;
    fn empty_table() -> Self;
    fn get<'a>(table: &'a Self::Table, key: &str) -> Option<&'a Self>;
    fn get_mut<'a>(table: &'a mut Self::Table, key: &str) -> Option<&'a mut Self>;
    fn remove(table: &mut Self::Table, key: &str) -> Option<Self>;
    fn insert(table: &mut Self::Table, key: String, value: Self);
    /// The child at `key`, inserting an empty table when it is absent.
    fn child<'a>(table: &'a mut Self::Table, key: &str) -> &'a mut Self;
    fn as_bool(&self) -> Option<bool>;
    fn from_bool(value: bool) -> Self;
    fn as_f64(&self) -> Option<f64>;
    fn from_f64(value: f64) -> Self;
    fn as_str(&self) -> Option<&str>;
    fn from_string(value: String) -> Self;
    fn list_of(items: Vec<Self>) -> Self;
}

impl ConfigTree for toml::Value {
    type Table = toml::Table;

    fn table(&self) -> Option<&toml::Table> {
        self.as_table()
    }
    fn table_mut(&mut self) -> Option<&mut toml::Table> {
        self.as_table_mut()
    }
    fn into_table(self) -> Option<toml::Table> {
        match self {
            toml::Value::Table(table) => Some(table),
            _ => None,
        }
    }
    fn empty_table() -> Self {
        toml::Value::Table(toml::Table::new())
    }
    fn get<'a>(table: &'a toml::Table, key: &str) -> Option<&'a Self> {
        table.get(key)
    }
    fn get_mut<'a>(table: &'a mut toml::Table, key: &str) -> Option<&'a mut Self> {
        table.get_mut(key)
    }
    fn remove(table: &mut toml::Table, key: &str) -> Option<Self> {
        table.remove(key)
    }
    fn insert(table: &mut toml::Table, key: String, value: Self) {
        table.insert(key, value);
    }
    fn child<'a>(table: &'a mut toml::Table, key: &str) -> &'a mut Self {
        table
            .entry(key.to_owned())
            .or_insert_with(Self::empty_table)
    }
    fn as_bool(&self) -> Option<bool> {
        toml::Value::as_bool(self)
    }
    fn from_bool(value: bool) -> Self {
        toml::Value::Boolean(value)
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            toml::Value::Float(value) => Some(*value),
            toml::Value::Integer(value) => Some(*value as f64),
            _ => None,
        }
    }
    fn from_f64(value: f64) -> Self {
        toml::Value::Float(value)
    }
    fn as_str(&self) -> Option<&str> {
        toml::Value::as_str(self)
    }
    fn from_string(value: String) -> Self {
        toml::Value::String(value)
    }
    fn list_of(items: Vec<Self>) -> Self {
        toml::Value::Array(items)
    }
}

impl ConfigTree for Value {
    type Table = Map<String, Value>;

    fn table(&self) -> Option<&Map<String, Value>> {
        self.as_object()
    }
    fn table_mut(&mut self) -> Option<&mut Map<String, Value>> {
        self.as_object_mut()
    }
    fn into_table(self) -> Option<Map<String, Value>> {
        match self {
            Value::Object(table) => Some(table),
            _ => None,
        }
    }
    fn empty_table() -> Self {
        Value::Object(Map::new())
    }
    fn get<'a>(table: &'a Map<String, Value>, key: &str) -> Option<&'a Self> {
        table.get(key)
    }
    fn get_mut<'a>(table: &'a mut Map<String, Value>, key: &str) -> Option<&'a mut Self> {
        table.get_mut(key)
    }
    fn remove(table: &mut Map<String, Value>, key: &str) -> Option<Self> {
        table.remove(key)
    }
    fn insert(table: &mut Map<String, Value>, key: String, value: Self) {
        table.insert(key, value);
    }
    fn child<'a>(table: &'a mut Map<String, Value>, key: &str) -> &'a mut Self {
        table
            .entry(key.to_owned())
            .or_insert_with(Self::empty_table)
    }
    fn as_bool(&self) -> Option<bool> {
        Value::as_bool(self)
    }
    fn from_bool(value: bool) -> Self {
        Value::Bool(value)
    }
    fn as_f64(&self) -> Option<f64> {
        Value::as_f64(self)
    }
    fn from_f64(value: f64) -> Self {
        serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
    fn as_str(&self) -> Option<&str> {
        Value::as_str(self)
    }
    fn from_string(value: String) -> Self {
        Value::String(value)
    }
    fn list_of(items: Vec<Self>) -> Self {
        Value::Array(items)
    }
}

/// The node at `path`, or `None` when a segment is absent or a parent on the
/// way is not a table.
fn node_at<'a, T: ConfigTree>(doc: &'a T, path: &[&str]) -> Option<&'a T> {
    path.iter()
        .try_fold(doc, |node, segment| T::get(node.table()?, segment))
}

/// `path` written the way the config file names a table, e.g. `[chains.solana]`.
fn table_name(path: &[&str]) -> String {
    format!("[{}]", path.join("."))
}

/// The retired auto-trader partial-exit switch as `doc` holds it: `None` when
/// the document does not carry it. Fails when it is not a boolean.
pub(crate) fn read_legacy_partial_exit_switch<T: ConfigTree>(doc: &T) -> Result<Option<bool>> {
    node_at(doc, LEGACY_PARTIAL_SWITCH)
        .map(|node| {
            node.as_bool().ok_or_else(|| Error::ParseFailed {
                detail: format!("{} must be a boolean", LEGACY_PARTIAL_SWITCH.join(".")),
            })
        })
        .transpose()
}

/// Move every legacy node of `doc` to its canonical path ([`RELOCATIONS`], in
/// order) and keep the stop-loss behaviour the document ran with. Returns
/// `true` when `doc` changed; the caller persists the canonical config, which
/// drops what moved. Fails without modifying `doc` when a legacy node, the
/// retired switch or a destination table has the wrong shape.
///
/// - A table row merges leaf by leaf: a leaf already present at the destination
///   wins, and a legacy leaf fills only what the destination lacks. A leaf row
///   moves one value and keeps the canonical one when both exist. A list row
///   moves one value as a one-item list and writes its companion list beside
///   it; a canonical list already present keeps both lists as they are. A row
///   without a destination drops its node.
/// - The one rule that is not a move: a stop loss sold part of a position only
///   while both [`STOP_LOSS_ALLOWANCE`] and [`LEGACY_PARTIAL_SWITCH`] were on
///   (an absent switch meant off). A document without
///   [`STOP_LOSS_PARTIAL_SHARE`], with the allowance on and the switch off or
///   absent, has its allowance turned off, so it keeps selling the whole
///   position. The rule reads the document before any row moves the share or
///   drops the switch, and fires for a document with no legacy node at all.
///   Once persisted, the document carries the share and the rule never fires
///   again.
/// - The stored time-override unit is rewritten to its canonical name, hours for a
///   name that matches no unit ([`canonical_time_unit`]).
pub(crate) fn relocate_legacy_layout<T: ConfigTree>(doc: &mut T) -> Result<bool> {
    let keep_full_stop_loss = node_at(doc, STOP_LOSS_PARTIAL_SHARE).is_none()
        && node_at(doc, STOP_LOSS_ALLOWANCE).and_then(T::as_bool) == Some(true)
        && read_legacy_partial_exit_switch(doc)? != Some(true);

    let mut working = doc.clone();
    let mut changed = false;
    if keep_full_stop_loss {
        if let Some((leaf, section)) = STOP_LOSS_ALLOWANCE.split_last() {
            let destination = table_at(&mut working, section, &STOP_LOSS_ALLOWANCE.join("."))?;
            T::insert(destination, (*leaf).to_owned(), T::from_bool(false));
            changed = true;
        }
    }
    for relocation in RELOCATIONS {
        let Some(legacy) = take_node(&mut working, relocation.from) else {
            continue;
        };
        match relocation.node {
            RelocationNode::Table => {
                let source = table_name(relocation.from);
                let Some(legacy) = legacy.into_table() else {
                    return Err(Error::ParseFailed {
                        detail: format!("legacy {source} must be a table"),
                    });
                };
                if let Some(to) = relocation.to {
                    let destination = table_at(&mut working, to, &source)?;
                    merge_missing_leaves::<T>(destination, legacy);
                }
            }
            RelocationNode::Leaf => {
                let source = relocation.from.join(".");
                if legacy.table().is_some() {
                    return Err(Error::ParseFailed {
                        detail: format!("legacy {source} must be a value, not a table"),
                    });
                }
                if let Some((leaf, section)) = relocation.to.and_then(<[_]>::split_last) {
                    let destination = table_at(&mut working, section, &source)?;
                    if T::get(destination, leaf).is_none() {
                        T::insert(destination, (*leaf).to_owned(), legacy);
                    }
                }
            }
            RelocationNode::ListOfOne {
                companion,
                item,
                bounds,
            } => {
                let source = relocation.from.join(".");
                if legacy.table().is_some() {
                    return Err(Error::ParseFailed {
                        detail: format!("legacy {source} must be a value, not a table"),
                    });
                }
                if let Some((leaf, section)) = relocation.to.and_then(<[_]>::split_last) {
                    let destination = table_at(&mut working, section, &source)?;
                    if T::get(destination, leaf).is_none() {
                        let legacy = clamped_into(legacy, bounds, &source);
                        T::insert(destination, (*leaf).to_owned(), T::list_of(vec![legacy]));
                        if let Some((leaf, section)) = companion.split_last() {
                            let destination = table_at(&mut working, section, &source)?;
                            if T::get(destination, leaf).is_none() {
                                T::insert(
                                    destination,
                                    (*leaf).to_owned(),
                                    T::list_of(vec![T::from_f64(item)]),
                                );
                            }
                        }
                    }
                }
            }
        }
        changed = true;
    }
    if canonical_time_unit(&mut working) {
        changed = true;
    }
    if changed {
        *doc = working;
    }
    Ok(changed)
}

/// Rewrite the time-override unit of `doc` to its canonical lowercase name: an alias or
/// another letter case names its unit, and a name that matches no unit becomes hours,
/// with a warning naming the stored value. Returns `true` when the stored name changed.
/// A value that is not a string is left for the typed load to refuse.
fn canonical_time_unit<T: ConfigTree>(doc: &mut T) -> bool {
    let Some(stored) = node_at(doc, TIME_OVERRIDE_UNIT)
        .and_then(T::as_str)
        .map(str::to_owned)
    else {
        return false;
    };
    let unit = TimeUnit::from_name(&stored).unwrap_or_else(|| {
        let fallback = TimeUnit::default();
        crate::logger::warning(
            crate::logger::LogTag::System,
            &format!(
                "{} = \"{stored}\" names no time unit; migrated as {}",
                TIME_OVERRIDE_UNIT.join("."),
                fallback.as_str()
            ),
        );
        fallback
    });
    if stored == unit.as_str() {
        return false;
    }
    let Some((leaf, section)) = TIME_OVERRIDE_UNIT.split_last() else {
        return false;
    };
    let Some(table) = node_at_mut(doc, section).and_then(T::table_mut) else {
        return false;
    };
    T::insert(
        table,
        (*leaf).to_owned(),
        T::from_string(unit.as_str().to_owned()),
    );
    true
}

/// The node at `path`, mutable; `None` when a segment is absent or a parent on the way
/// is not a table.
fn node_at_mut<'a, T: ConfigTree>(doc: &'a mut T, path: &[&str]) -> Option<&'a mut T> {
    let mut node = doc;
    for segment in path {
        node = T::get_mut(node.table_mut()?, segment)?;
    }
    Some(node)
}

/// `value` clamped into `(min, max)`, with a warning naming `source` when it
/// moved. A value that is not a number, NaN included, is left for the typed
/// load to refuse.
fn clamped_into<T: ConfigTree>(value: T, (min, max): (f64, f64), source: &str) -> T {
    let Some(number) = value.as_f64() else {
        return value;
    };
    if number.is_nan() || (min..=max).contains(&number) {
        return value;
    }
    let clamped = number.clamp(min, max);
    crate::logger::warning(
        crate::logger::LogTag::System,
        &format!("Legacy {source} = {number} is outside {min}..={max}; migrated as {clamped}"),
    );
    T::from_f64(clamped)
}

/// Remove and return the node at `path`; `None` when it is absent.
fn take_node<T: ConfigTree>(doc: &mut T, path: &[&str]) -> Option<T> {
    let (last, parents) = path.split_last()?;
    T::remove(node_at_mut(doc, parents)?.table_mut()?, last)
}

/// The table at `path`, creating every missing table on the way. A node on the
/// way that exists but is not a table refuses the relocation of `source`.
fn table_at<'a, T: ConfigTree>(
    doc: &'a mut T,
    path: &[&str],
    source: &str,
) -> Result<&'a mut T::Table> {
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
        node = T::child(node.table_mut().ok_or_else(|| refused(depth))?, segment);
    }
    node.table_mut().ok_or_else(|| refused(path.len()))
}

/// Fill `destination` from `legacy`: tables merge key by key, recursively; a
/// key the destination already holds keeps the destination's value; any
/// other key takes the legacy value. Arrays are leaves.
fn merge_missing_leaves<T: ConfigTree>(destination: &mut T::Table, legacy: T::Table) {
    for (key, legacy_value) in legacy {
        match T::get_mut(destination, &key) {
            Some(existing) => {
                if let (Some(existing), Some(legacy_table)) =
                    (existing.table_mut(), legacy_value.into_table())
                {
                    merge_missing_leaves::<T>(existing, legacy_table);
                }
            }
            None => T::insert(destination, key, legacy_value),
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

        let written = table(&config);
        let pools = written["pools"].as_table().expect("pools table");
        assert!(!pools.contains_key("enable_raydium_discovery"));

        // Trading amounts keep their sections.
        assert_eq!(config.trader.max_open_positions, 4);
        assert_eq!(config.trader.trade_size_sol, 0.02);
        assert_eq!(config.trader.entry_sizes, vec![0.01, 0.03]);
        assert_eq!(config.trader.loss_limit_sol, 0.4);
        assert_eq!(config.positions.profit_extra_needed_sol, 0.0005);

        // Exit-rule leaves of [positions] land in [trader]; the allowance the
        // retired switch held back is turned off.
        assert!(config.trader.trailing_stop_enabled);
        assert_eq!(config.trader.trailing_stop_activation_pct, 12.0);
        assert_eq!(config.trader.trailing_stop_distance_pct, 4.0);
        assert_eq!(config.trader.stop_loss_partial_pct, 35.0);
        assert!(!config.trader.stop_loss_allow_partial);
        let positions = written["positions"].as_table().expect("positions table");
        for leaf in legacy_position_leaves() {
            assert!(
                !positions.contains_key(leaf),
                "[positions] {leaf} persisted"
            );
        }
        assert_eq!(parsed.legacy_partial_exit_switch, Some(false));

        // The ROI target becomes the one step of the take-profit ladder, selling
        // everything as it did, under the switch the file held.
        assert!(!config.trader.take_profit_enabled);
        assert_eq!(config.trader.take_profit_levels_pct, vec![35.0]);
        assert_eq!(config.trader.take_profit_sell_pct, vec![100.0]);
        let trader = written["trader"].as_table().expect("trader table");
        for leaf in ["roi_exit_enabled", "roi_target_percent"] {
            assert!(!trader.contains_key(leaf), "[trader] {leaf} persisted");
        }

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
        assert!(!second.config.trader.stop_loss_allow_partial);
        assert_eq!(second.config.trader.stop_loss_partial_pct, 35.0);
        assert_eq!(second.config.trader.take_profit_levels_pct, vec![35.0]);
        assert_eq!(second.config.trader.take_profit_sell_pct, vec![100.0]);
        assert_eq!(second.legacy_partial_exit_switch, None);
    }

    #[test]
    fn legacy_layout_keeps_non_finite_floats_in_moved_and_unmoved_sections() {
        let parsed = parse(
            r#"
[rpc]
max_retries = 4

[swaps.direct]
max_price_impact_pct = inf

[swaps.cost_guard]
max_extra_cost_pct = nan

[telegram]
significant_pnl_threshold = -inf
"#,
        );
        assert!(parsed.migrated);
        let config = &parsed.config;
        assert_eq!(config.chains.solana.rpc.max_retries, 4);
        assert_eq!(
            config.chains.solana.swaps.direct.max_price_impact_pct,
            f64::INFINITY
        );
        assert!(config
            .chains
            .solana
            .swaps
            .cost_guard
            .max_extra_cost_pct
            .is_nan());
        assert_eq!(config.telegram.significant_pnl_threshold, f64::NEG_INFINITY);

        let serialized = toml::to_string(config).expect("serialize");
        let second = parse(&serialized);
        assert!(!second.migrated);
        assert_eq!(
            second
                .config
                .chains
                .solana
                .swaps
                .direct
                .max_price_impact_pct,
            f64::INFINITY
        );
        assert!(second
            .config
            .chains
            .solana
            .swaps
            .cost_guard
            .max_extra_cost_pct
            .is_nan());
        assert_eq!(
            second.config.telegram.significant_pnl_threshold,
            f64::NEG_INFINITY
        );
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
            let err = relocate_legacy_layout(&mut document).unwrap_err();
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
stop_loss_enabled = true

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
        assert!(relocate_legacy_layout(&mut document).expect("relocate"));
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

    // ------------------------------------------------------------------
    // Values implied by omitted keys whose default changed
    // ------------------------------------------------------------------

    #[test]
    fn a_new_config_starts_with_the_stop_loss_on_at_half_the_entry() {
        let trader = Config::default().trader;
        assert!(trader.stop_loss_enabled);
        assert_eq!(trader.stop_loss_threshold_pct, 50.0);
    }

    #[test]
    fn a_written_stop_loss_switch_is_kept_either_way() {
        for enabled in [true, false] {
            let parsed = parse(&format!(
                "[trader]\nstop_loss_enabled = {enabled}\nstop_loss_threshold_pct = 30.0\n"
            ));
            assert!(!parsed.migrated, "stop_loss_enabled = {enabled}");
            assert_eq!(parsed.config.trader.stop_loss_enabled, enabled);
            assert_eq!(parsed.config.trader.stop_loss_threshold_pct, 30.0);
        }
    }

    #[test]
    fn a_file_without_the_stop_loss_switch_keeps_it_off_once_written() {
        for raw in ["[trader]\nmax_open_positions = 3\n", "[positions]\n", ""] {
            let parsed = parse(raw);
            assert!(parsed.migrated, "{raw:?} is written back with the switch");
            assert!(!parsed.config.trader.stop_loss_enabled, "{raw:?}");
            assert_eq!(parsed.config.trader.stop_loss_threshold_pct, 50.0);

            let written = toml::to_string(&parsed.config).expect("serialize");
            assert!(has_key(
                &table(&parsed.config),
                "trader",
                "stop_loss_enabled"
            ));
            let second = parse(&written);
            assert!(!second.migrated, "{raw:?} migrates once");
            assert!(!second.config.trader.stop_loss_enabled);
        }
        assert!(!parse(V0_2_13_CONFIG).config.trader.stop_loss_enabled);
    }

    // ------------------------------------------------------------------
    // Exit-rule leaves of [positions] moving into [trader]
    // ------------------------------------------------------------------

    /// Every `[positions]` leaf a relocation row moves or drops.
    fn legacy_position_leaves() -> impl Iterator<Item = &'static str> {
        RELOCATIONS
            .iter()
            .filter(|relocation| relocation.from[0] == "positions")
            .map(|relocation| relocation.from[1])
    }

    /// The `[trader]` and `[positions]` tables `config` persists as.
    fn exit_tables(config: &Config) -> (toml::Table, toml::Table) {
        let written = table(config);
        let section = |name: &str| {
            written[name]
                .as_table()
                .unwrap_or_else(|| panic!("[{name}] table"))
                .clone()
        };
        (section("trader"), section("positions"))
    }

    #[test]
    fn an_allowance_without_the_legacy_switch_keeps_selling_the_whole_position() {
        for switch in ["partial_exit_enabled = false\n", ""] {
            let raw = format!(
                "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\n[positions]\n{switch}"
            );
            let parsed = parse(&raw);
            assert!(parsed.migrated, "{switch:?}");
            assert!(!parsed.config.trader.stop_loss_allow_partial, "{switch:?}");
        }
    }

    #[test]
    fn an_allowance_with_the_legacy_switch_on_keeps_selling_part() {
        let parsed = parse(
            "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\n\
             [positions]\npartial_exit_enabled = true\npartial_exit_default_pct = 30.0\n",
        );
        assert!(parsed.migrated);
        assert!(parsed.config.trader.stop_loss_allow_partial);
        assert_eq!(parsed.config.trader.stop_loss_partial_pct, 30.0);
    }

    #[test]
    fn an_allowance_off_stays_off_whatever_the_legacy_switch() {
        for switch in ["true", "false"] {
            let parsed = parse(&format!(
                "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = false\n\n\
                 [positions]\npartial_exit_enabled = {switch}\n"
            ));
            assert!(parsed.migrated, "switch {switch}");
            assert!(
                !parsed.config.trader.stop_loss_allow_partial,
                "switch {switch}"
            );
        }
    }

    #[test]
    fn a_document_with_the_partial_share_never_has_its_allowance_changed() {
        let parsed = parse(
            "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\
             stop_loss_partial_pct = 40.0\n\n[positions]\npartial_exit_enabled = false\n",
        );
        assert!(parsed.migrated, "the retired switch is still dropped");
        assert!(parsed.config.trader.stop_loss_allow_partial);
        assert_eq!(parsed.config.trader.stop_loss_partial_pct, 40.0);
    }

    #[test]
    fn the_partial_share_and_trailing_stop_move_with_their_values() {
        let parsed = parse(
            "[trader]\nstop_loss_enabled = true\n\n[positions]\n\
             partial_exit_enabled = true\npartial_exit_default_pct = 35.0\n\
             partial_exit_min_pct = 10.0\npartial_exit_max_pct = 80.0\n\
             trailing_stop_enabled = true\ntrailing_stop_activation_pct = 0.5\n\
             trailing_stop_distance_pct = 7.0\nposition_open_cooldown_secs = 9\n",
        );
        assert!(parsed.migrated);
        let trader = &parsed.config.trader;
        assert_eq!(trader.stop_loss_partial_pct, 35.0);
        assert!(trader.trailing_stop_enabled);
        assert_eq!(trader.trailing_stop_activation_pct, 0.5);
        assert_eq!(trader.trailing_stop_distance_pct, 7.0);
        assert_eq!(parsed.config.positions.position_open_cooldown_secs, 9);

        let (trader, positions) = exit_tables(&parsed.config);
        for leaf in [
            "stop_loss_partial_pct",
            "trailing_stop_enabled",
            "trailing_stop_activation_pct",
            "trailing_stop_distance_pct",
        ] {
            assert!(trader.contains_key(leaf), "[trader] {leaf}");
        }
        for legacy in legacy_position_leaves() {
            assert!(!positions.contains_key(legacy), "[positions] {legacy}");
        }
    }

    #[test]
    fn a_canonical_exit_leaf_wins_over_its_legacy_twin() {
        let parsed = parse(
            "[trader]\nstop_loss_enabled = true\nstop_loss_partial_pct = 60.0\n\
             trailing_stop_distance_pct = 12.0\n\n[positions]\n\
             partial_exit_default_pct = 20.0\ntrailing_stop_distance_pct = 3.0\n\
             trailing_stop_activation_pct = 4.0\n",
        );
        assert!(parsed.migrated);
        let trader = &parsed.config.trader;
        assert_eq!(trader.stop_loss_partial_pct, 60.0);
        assert_eq!(trader.trailing_stop_distance_pct, 12.0);
        assert_eq!(trader.trailing_stop_activation_pct, 4.0);
    }

    #[test]
    fn the_exit_migration_fires_once() {
        let raw = "[trader]\nstop_loss_enabled = true\nstop_loss_allow_partial = true\n\n\
                   [positions]\npartial_exit_enabled = false\ntrailing_stop_enabled = true\n";
        let first = parse(raw);
        assert!(first.migrated);
        assert!(!first.config.trader.stop_loss_allow_partial);

        let written = toml::to_string(&first.config).expect("serialize");
        let second = parse(&written);
        assert!(!second.migrated);
        assert_eq!(json(&second.config), json(&first.config));

        let mut document = toml::Value::Table(toml::from_str(&written).expect("toml"));
        assert!(!relocate_legacy_layout(&mut document).expect("migrate"));
    }

    #[test]
    fn a_json_import_migrates_like_the_config_file() {
        let mut document = serde_json::json!({
            "trader": { "stop_loss_allow_partial": true },
            "positions": {
                "partial_exit_enabled": false,
                "partial_exit_default_pct": 25.0,
                "partial_exit_max_pct": 90.0,
                "trailing_stop_enabled": true,
                "trailing_stop_distance_pct": 6.0,
                "position_open_cooldown_secs": 4
            }
        });
        assert!(relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(
            document,
            serde_json::json!({
                "trader": {
                    "stop_loss_allow_partial": false,
                    "stop_loss_partial_pct": 25.0,
                    "trailing_stop_enabled": true,
                    "trailing_stop_distance_pct": 6.0
                },
                "positions": { "position_open_cooldown_secs": 4 }
            })
        );
        assert!(!relocate_legacy_layout(&mut document).expect("second pass"));
    }

    #[test]
    fn a_json_import_without_exit_leaves_is_untouched() {
        let original = serde_json::json!({
            "trader": { "stop_loss_allow_partial": false },
            "positions": { "position_open_cooldown_secs": 4 }
        });
        let mut document = original.clone();
        assert!(!relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(document, original);
    }

    #[test]
    fn a_stored_time_unit_loads_under_its_canonical_name_once() {
        for (stored, unit) in [
            ("weeks", TimeUnit::Hours),
            ("MINUTES", TimeUnit::Minutes),
            ("h", TimeUnit::Hours),
            ("Day", TimeUnit::Days),
        ] {
            let first = parse(&format!(
                "[trader]\ntime_override_unit = \"{stored}\"\ntime_override_duration = 2.0\n"
            ));
            assert!(first.migrated, "{stored} is rewritten");
            assert_eq!(first.config.trader.time_override_unit, unit, "{stored}");
            assert_eq!(first.config.trader.time_override_duration, 2.0);

            let written = toml::to_string(&first.config).expect("serialize");
            let second = parse(&written);
            assert!(!second.migrated, "{stored}: the canonical name stays");
            assert_eq!(second.config.trader.time_override_unit, unit);

            let mut document = toml::Value::Table(toml::from_str(&written).expect("toml"));
            assert!(!relocate_legacy_layout(&mut document).expect("second pass"));
        }
        let canonical = parse("[trader]\ntime_override_unit = \"days\"\n");
        assert!(!canonical.migrated, "a canonical name is no migration");
    }

    #[test]
    fn a_json_document_takes_the_canonical_time_unit() {
        let mut document = serde_json::json!({ "trader": { "time_override_unit": "fortnights" } });
        assert!(relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(
            document,
            serde_json::json!({ "trader": { "time_override_unit": "hours" } })
        );
        assert!(!relocate_legacy_layout(&mut document).expect("second pass"));

        let original = serde_json::json!({ "trader": { "time_override_unit": 3 } });
        let mut document = original.clone();
        assert!(!relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(
            document, original,
            "a value that is not a name is left to the typed load"
        );
    }

    #[test]
    fn a_misshapen_legacy_switch_is_refused_without_touching_the_document() {
        let original = serde_json::json!({
            "trader": { "stop_loss_allow_partial": true },
            "positions": { "partial_exit_enabled": "yes", "trailing_stop_enabled": true }
        });
        let mut document = original.clone();
        let err = relocate_legacy_layout(&mut document).unwrap_err();
        assert!(matches!(err, Error::ParseFailed { .. }));
        assert_eq!(document, original);
        assert!(matches!(
            super::super::utils::parse_config_document(
                "[positions]\npartial_exit_enabled = \"yes\"\n"
            ),
            Err(Error::ParseFailed { .. })
        ));
    }

    #[test]
    fn a_legacy_table_where_a_leaf_moves_is_refused_without_touching_the_document() {
        for raw in [
            "[positions.trailing_stop_enabled]\nvalue = true\n",
            "[positions.partial_exit_min_pct]\nvalue = 10.0\n",
        ] {
            let table: toml::Table = toml::from_str(raw).expect("toml");
            let original = serde_json::to_value(&table).expect("json");
            let mut document = original.clone();
            let err = relocate_legacy_layout(&mut document).unwrap_err();
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
    fn a_leaf_row_lands_under_the_last_segment_of_its_destination() {
        for relocation in RELOCATIONS {
            let (RelocationNode::Leaf, Some(to)) = (relocation.node, relocation.to) else {
                continue;
            };
            let mut document = serde_json::Value::Object(serde_json::Map::new());
            let value = serde_json::json!(17.0);
            let (leaf, parents) = relocation.from.split_last().expect("leaf path");
            let mut node = &mut document;
            for segment in parents {
                node = node
                    .as_object_mut()
                    .expect("object")
                    .entry(*segment)
                    .or_insert_with(|| serde_json::json!({}));
            }
            node.as_object_mut()
                .expect("object")
                .insert((*leaf).to_owned(), value.clone());

            assert!(relocate_legacy_layout(&mut document).expect("relocate"));
            assert_eq!(node_at(&document, to), Some(&value), "{}", to.join("."));
            assert_eq!(node_at(&document, relocation.from), None);
        }
    }
    #[test]
    fn a_list_row_lands_as_a_one_item_list_beside_its_companion() {
        for relocation in RELOCATIONS {
            let (
                RelocationNode::ListOfOne {
                    companion, item, ..
                },
                Some(to),
            ) = (relocation.node, relocation.to)
            else {
                continue;
            };
            let mut document = serde_json::Value::Object(serde_json::Map::new());
            let value = serde_json::json!(35.0);
            let (leaf, parents) = relocation.from.split_last().expect("leaf path");
            let mut node = &mut document;
            for segment in parents {
                node = node
                    .as_object_mut()
                    .expect("object")
                    .entry(*segment)
                    .or_insert_with(|| serde_json::json!({}));
            }
            node.as_object_mut()
                .expect("object")
                .insert((*leaf).to_owned(), value.clone());

            assert!(relocate_legacy_layout(&mut document).expect("relocate"));
            assert_eq!(
                node_at(&document, to),
                Some(&serde_json::json!([value])),
                "{}",
                to.join(".")
            );
            assert_eq!(
                node_at(&document, companion),
                Some(&serde_json::json!([item])),
                "{}",
                companion.join(".")
            );
            assert_eq!(node_at(&document, relocation.from), None);
            assert!(!relocate_legacy_layout(&mut document).expect("second pass"));
        }
    }

    #[test]
    fn the_roi_target_becomes_a_one_step_take_profit_ladder() {
        let parsed = parse("[trader]\nroi_exit_enabled = true\nroi_target_percent = 42.0\n");
        assert!(parsed.migrated);
        let trader = &parsed.config.trader;
        assert!(trader.take_profit_enabled);
        assert_eq!(
            trader.take_profit_levels(),
            vec![crate::config::TakeProfitLevel {
                profit_pct: 42.0,
                sell_pct: 100.0,
            }]
        );
        assert!(super::super::utils::validate_config(&parsed.config).is_ok());

        let serialized = toml::to_string(&parsed.config).expect("serialize");
        let second = parse(&serialized);
        assert!(!second.migrated);
        assert_eq!(json(&second.config), json(&parsed.config));
    }

    #[test]
    fn a_canonical_take_profit_ladder_wins_over_the_roi_target() {
        let raw = "[trader]\nroi_exit_enabled = false\nroi_target_percent = 42.0\n\
                   take_profit_enabled = true\n\
                   take_profit_levels_pct = [20.0, 60.0]\n\
                   take_profit_sell_pct = [50.0, 30.0]\n";
        let parsed = parse(raw);
        assert!(parsed.migrated, "the legacy leaves are dropped");
        let trader = &parsed.config.trader;
        assert!(trader.take_profit_enabled);
        assert_eq!(trader.take_profit_levels_pct, vec![20.0, 60.0]);
        assert_eq!(trader.take_profit_sell_pct, vec![50.0, 30.0]);
    }

    #[test]
    fn a_json_import_turns_the_roi_target_into_a_ladder_like_the_config_file() {
        let mut document = serde_json::json!({
            "trader": { "roi_exit_enabled": false, "roi_target_percent": 15.0 }
        });
        assert!(relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(
            document,
            serde_json::json!({
                "trader": {
                    "take_profit_enabled": false,
                    "take_profit_levels_pct": [15.0],
                    "take_profit_sell_pct": [100.0]
                }
            })
        );
        assert!(!relocate_legacy_layout(&mut document).expect("second pass"));

        // A canonical sell list already present is kept beside the moved level.
        let mut document = serde_json::json!({
            "trader": { "roi_target_percent": 15.0, "take_profit_sell_pct": [40.0] }
        });
        assert!(relocate_legacy_layout(&mut document).expect("migrate"));
        assert_eq!(
            document,
            serde_json::json!({
                "trader": {
                    "take_profit_levels_pct": [15.0],
                    "take_profit_sell_pct": [40.0]
                }
            })
        );
    }

    #[test]
    fn a_roi_target_table_is_refused_without_touching_the_document() {
        let original = serde_json::json!({
            "trader": { "roi_target_percent": { "value": 20.0 }, "roi_exit_enabled": true }
        });
        let mut document = original.clone();
        let err = relocate_legacy_layout(&mut document).unwrap_err();
        assert!(matches!(err, Error::ParseFailed { .. }));
        assert_eq!(document, original);
    }
}
