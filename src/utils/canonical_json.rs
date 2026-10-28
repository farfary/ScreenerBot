// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Canonical JSON: one byte form for every JSON value, whatever the key order it was built or received in.

use serde_json::Value;

/// Deterministic JSON: object keys sorted recursively, no insignificant
/// whitespace. Two equal values always produce the same bytes, and so the same
/// digest, regardless of key order on the wire or in a map.
pub fn canonicalize(value: &Value) -> String {
    fn norm(v: &Value) -> Value {
        match v {
            Value::Array(items) => Value::Array(items.iter().map(norm).collect()),
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    out.insert(k.clone(), norm(&map[k]));
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
    norm(value).to_string()
}

#[cfg(test)]
#[path = "canonical_json_tests.rs"]
mod tests;
