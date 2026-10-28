// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of canonical JSON: key order never changes the bytes, a value always does.

use serde_json::json;

use super::*;

#[test]
fn canonicalize_is_key_order_independent() {
    let a = json!({ "b": 1, "a": { "y": 2, "x": 3 } });
    let b = json!({ "a": { "x": 3, "y": 2 }, "b": 1 });
    assert_eq!(canonicalize(&a), canonicalize(&b));
}

#[test]
fn canonicalize_differs_when_a_value_changes() {
    assert_ne!(
        canonicalize(&json!({ "sol": 1 })),
        canonicalize(&json!({ "sol": 9 }))
    );
}
