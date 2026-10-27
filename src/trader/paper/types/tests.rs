// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Persisted paper fill provenance compatibility tests.

use super::*;

#[test]
fn quote_metadata_keeps_old_json_and_round_trips_raw_values_above_f64_precision() {
    let old = serde_json::json!({
        "router_id": "fixture",
        "initial_at": "2026-01-01T00:00:00Z",
        "landing_at": "2026-01-01T00:00:01Z",
        "requested_delay_ms": 0,
        "queue_duration_ms": 0,
        "minimum_output_amount": "1",
        "final_output_amount": "9007199254741010",
        "epoch": 0,
        "network_fee_estimated": true,
        "platform_fee_included_in_output": true
    });
    let legacy: PaperQuoteMetadata = serde_json::from_value(old).unwrap();
    assert_eq!(legacy.input_amount, None);
    assert_eq!(legacy.token_decimals, None);
    assert_eq!(legacy.final_output_amount.to_string(), "9007199254741010");
    let encoded = serde_json::to_value(&legacy).unwrap();
    assert!(encoded.get("input_amount").is_none());
    assert!(encoded.get("token_decimals").is_none());
}
