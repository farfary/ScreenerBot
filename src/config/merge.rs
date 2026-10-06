// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Partial-update merge for config sections serialized as JSON documents.

use serde_json::Value;

/// Merge `patch` into `base`: objects merge key by key, recursively; any
/// other value (array, scalar, null) replaces the base value.
pub(crate) fn merge_document(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_document(base_value, patch_value),
                    None => {
                        base_map.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::merge_document;
    use serde_json::json;

    #[test]
    fn nested_object_keeps_siblings() {
        let mut base = json!({"a": {"x": 1, "y": {"p": 1, "q": 2}}, "b": 3});
        merge_document(&mut base, &json!({"a": {"y": {"p": 9}}}));
        assert_eq!(base, json!({"a": {"x": 1, "y": {"p": 9, "q": 2}}, "b": 3}));
    }

    #[test]
    fn array_replaces() {
        let mut base = json!({"list": [1, 2, 3]});
        merge_document(&mut base, &json!({"list": [4]}));
        assert_eq!(base, json!({"list": [4]}));
    }

    #[test]
    fn null_replaces() {
        let mut base = json!({"a": {"x": 1}, "b": 2});
        merge_document(&mut base, &json!({"a": null}));
        assert_eq!(base, json!({"a": null, "b": 2}));
    }

    #[test]
    fn scalar_over_object_replaces() {
        let mut base = json!({"a": {"x": 1}});
        merge_document(&mut base, &json!({"a": 5}));
        assert_eq!(base, json!({"a": 5}));
    }

    #[test]
    fn object_over_scalar_replaces() {
        let mut base = json!({"a": 5});
        merge_document(&mut base, &json!({"a": {"x": 1}}));
        assert_eq!(base, json!({"a": {"x": 1}}));
    }

    #[test]
    fn new_key_is_inserted() {
        let mut base = json!({"a": 1});
        merge_document(&mut base, &json!({"b": 2}));
        assert_eq!(base, json!({"a": 1, "b": 2}));
    }
}
