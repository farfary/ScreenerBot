//! Canonical config access for every agent-facing surface.
//!
//! An agent is allowed to read and change *all* of ScreenerBot's configuration
//! — RPC endpoints, trading parameters, filters, provider credentials — with
//! exactly one carve-out: the wallet private key material (`wallet_encrypted`
//! and its nonce). That material is never returned and never writable through
//! any agent surface; it is imported by the owner in the app.
//!
//! Everything here works on the serialized `Config` JSON, so a new config field
//! becomes agent-controllable the moment it is added to the schema. There is no
//! hand-maintained allowlist of settable keys to drift out of date.

use serde_json::{Map, Value};

use crate::agent_control::error::{Error, Result};
use crate::config::{self, metadata::collect_config_metadata, schemas::Config};

/// Substituted for wallet key material in every agent-visible read.
pub const REDACTED: &str = "[redacted]";

/// The only config paths agents may not read or write. Dotted paths into the
/// serialized `Config`.
pub const SECRET_PATHS: &[&str] = &["wallet_encrypted", "wallet_nonce"];

/// One applied config change, reported back to the caller.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppliedChange {
    pub path: String,
    pub previous: Value,
    pub value: Value,
}

/// True when `path` is a secret path, sits inside one, or is an ancestor of one
/// (writing an ancestor would clobber the secret). The empty path is the root,
/// and therefore an ancestor of every secret.
pub fn touches_secret(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty() {
        return true;
    }
    SECRET_PATHS.iter().any(|secret| {
        path == *secret
            || path.starts_with(&format!("{secret}."))
            || secret.starts_with(&format!("{path}."))
    })
}

/// The live configuration as JSON. Fails (rather than panicking) before the
/// config is loaded — the MCP adapter runs in that pre-boot state.
fn snapshot() -> Result<Value> {
    if !config::is_config_initialized() {
        return Err(Error::Config(config::Error::NotLoaded));
    }
    serde_json::to_value(config::get_config_clone()).map_err(|e| Error::InvalidParameters {
        detail: format!("configuration could not be serialized: {e}"),
    })
}

/// Replace wallet key material with [`REDACTED`]. An unset secret stays an
/// empty string so an agent can still tell that no wallet is configured.
fn redact(root: &mut Value) {
    let Some(object) = root.as_object_mut() else {
        return;
    };
    for secret in SECRET_PATHS {
        if let Some(slot) = object.get_mut(*secret) {
            if !matches!(slot, Value::String(s) if s.is_empty()) {
                *slot = Value::String(REDACTED.to_owned());
            }
        }
    }
}

/// The keys reachable one level below `value`, for a useful "unknown path" error.
fn child_keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        Value::Array(items) => (0..items.len()).map(|i| i.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn unknown_path(path: &str, walked: &str, parent: &Value) -> Error {
    let available = child_keys(parent).join(", ");
    let location = if walked.is_empty() {
        "the configuration root".to_owned()
    } else {
        format!("'{walked}'")
    };
    Error::InvalidParameters {
        detail: format!(
            "unknown config path '{path}': {location} has no such key. Available: {available}"
        ),
    }
}

/// Walk a dotted path. Numeric segments index into arrays.
fn resolve<'a>(root: &'a Value, path: &str) -> Result<&'a Value> {
    let mut current = root;
    let mut walked = String::new();
    for segment in path.split('.') {
        let next = match current {
            Value::Object(map) => map.get(segment),
            Value::Array(items) => segment.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        };
        current = next.ok_or_else(|| unknown_path(path, &walked, current))?;
        if !walked.is_empty() {
            walked.push('.');
        }
        walked.push_str(segment);
    }
    Ok(current)
}

/// Overwrite an existing leaf or subtree, returning what was there before. The
/// path must already exist: agents configure the schema, they do not extend it.
fn set_at(root: &mut Value, path: &str, value: Value) -> Result<Value> {
    let segments: Vec<&str> = path.split('.').collect();
    let (last, parents) = segments
        .split_last()
        .expect("split always yields at least one segment");

    let mut walked = String::new();
    let mut current = root;
    for segment in parents {
        // Borrow-checker dance: probe for existence, then descend.
        let missing = match &*current {
            Value::Object(map) => !map.contains_key(*segment),
            Value::Array(items) => !segment
                .parse::<usize>()
                .is_ok_and(|index| index < items.len()),
            _ => true,
        };
        if missing {
            return Err(unknown_path(path, &walked, current));
        }
        current = match current {
            Value::Object(map) => map.get_mut(*segment).expect("presence checked above"),
            Value::Array(items) => {
                let index: usize = segment.parse().expect("presence checked above");
                &mut items[index]
            }
            _ => unreachable!("presence check rejects scalars"),
        };
        if !walked.is_empty() {
            walked.push('.');
        }
        walked.push_str(segment);
    }

    match current {
        Value::Object(map) => match map.get_mut(*last) {
            Some(slot) => Ok(std::mem::replace(slot, value)),
            None => Err(unknown_path(path, &walked, current)),
        },
        Value::Array(items) => match last.parse::<usize>() {
            Ok(index) if index < items.len() => Ok(std::mem::replace(&mut items[index], value)),
            _ => Err(unknown_path(path, &walked, current)),
        },
        _ => Err(unknown_path(path, &walked, current)),
    }
}

/// Read the whole configuration, or the subtree at a dotted path, with wallet
/// key material redacted.
pub fn read(path: Option<&str>) -> Result<Value> {
    let mut root = snapshot()?;
    redact(&mut root);
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => resolve(&root, path).cloned(),
        None => Ok(root),
    }
}

/// Field metadata for every config section the app renders: type, label, unit,
/// bounds and default. This is how an agent discovers what it may set.
pub fn schema(section: Option<&str>) -> Result<Value> {
    let metadata =
        serde_json::to_value(collect_config_metadata()).map_err(|e| Error::InvalidParameters {
            detail: format!("config metadata could not be serialized: {e}"),
        })?;
    match section.map(str::trim).filter(|s| !s.is_empty()) {
        Some(section) => resolve(&metadata, section).cloned(),
        None => Ok(metadata),
    }
}

/// Build the candidate configuration for a batch of changes, from the config
/// the caller is holding the write lock on.
fn build_updated(
    current: &Config,
    updates: &[(String, Value)],
) -> Result<(Config, Vec<AppliedChange>)> {
    let mut working = serde_json::to_value(current).map_err(|e| Error::InvalidParameters {
        detail: format!("configuration could not be serialized: {e}"),
    })?;
    let mut applied = Vec::with_capacity(updates.len());

    for (path, value) in updates {
        let path = path.trim();
        let previous = set_at(&mut working, path, value.clone())?;
        applied.push(AppliedChange {
            path: path.to_owned(),
            previous,
            value: value.clone(),
        });
    }

    // Deserializing the whole document is the type check: a wrong-typed value
    // fails here rather than being written into a half-updated config.
    let candidate: Config =
        serde_json::from_value(working).map_err(|e| Error::InvalidParameters {
            detail: format!("rejected by the configuration schema: {e}"),
        })?;
    config::validate_config(&candidate)?;
    Ok((candidate, applied))
}

/// Apply a batch of `path -> value` changes atomically: every path is resolved,
/// type-checked against the schema and validated before anything is written, so
/// a bad entry leaves the live configuration untouched. On success the change is
/// persisted to `config.toml`.
///
/// The read-modify-write runs inside the config write lock, so two concurrent
/// callers cannot each serialize the same starting config and have the later
/// write silently drop the earlier one's change.
pub fn apply(updates: &[(String, Value)]) -> Result<Vec<AppliedChange>> {
    if updates.is_empty() {
        return Err(Error::InvalidParameters {
            detail: "no configuration changes were supplied".to_owned(),
        });
    }
    // Key material is refused before the lock is taken and before any path is
    // resolved, so no code path can reach it.
    for (path, _) in updates {
        let path = path.trim();
        if touches_secret(path) {
            return Err(Error::SecretPath {
                path: path.to_owned(),
            });
        }
    }
    if !config::is_config_initialized() {
        return Err(Error::Config(config::Error::NotLoaded));
    }

    let mut outcome: Result<Vec<AppliedChange>> = Err(Error::InvalidParameters {
        detail: "the configuration update did not run".to_owned(),
    });
    config::update_config_section(
        |cfg| {
            outcome = build_updated(cfg, updates).map(|(candidate, applied)| {
                *cfg = candidate;
                applied
            });
        },
        // Saved below, and only once the update actually succeeded.
        false,
    )?;

    let applied = outcome?;
    config::save_config(None)?;
    Ok(applied)
}

/// Convenience for the single-path form of [`apply`].
pub fn set_one(path: &str, value: Value) -> Result<AppliedChange> {
    let mut applied = apply(&[(path.to_owned(), value)])?;
    Ok(applied.remove(0))
}

/// Parse the `updates` object of a tool call into ordered path/value pairs.
pub fn updates_from_object(object: &Map<String, Value>) -> Vec<(String, Value)> {
    object
        .iter()
        .map(|(path, value)| (path.clone(), value.clone()))
        .collect()
}
