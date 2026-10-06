// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recorders for transactions, swaps, pools and positions.

use super::super::maintenance::is_category_enabled;
use crate::chains::RawAmount;
use crate::events::{Event, EventCategory, Severity};
use chrono::Utc;
use serde_json::{json, Map, Value};

// =============================================================================
// TRANSACTION EVENTS
// =============================================================================

/// Record a transaction event (submission/confirmation/failure)
pub async fn record_transaction_event(
    signature: &str,
    confirmation_status: &str,
    success: bool,
    fee: Option<u64>,
    slot: Option<u64>,
    error_message: Option<&str>,
) {
    if !is_category_enabled(&EventCategory::Transaction) {
        return;
    }

    let payload = json!({
        "signature": signature,
        "confirmation_status": confirmation_status,
        "fee": fee,
        "slot": slot,
        "success": success,
        "error_message": error_message,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = if success {
        Severity::Info
    } else {
        Severity::Error
    };

    let event = Event::new(
        EventCategory::Transaction,
        Some(confirmation_status.to_string()),
        severity,
        None,
        Some(signature.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// SWAP EVENTS
// =============================================================================

/// Record a swap event with standardized payload
pub async fn record_swap_event(
    signature: &str,
    input_mint: &str,
    output_mint: &str,
    amount_in: u64,
    amount_out: u64,
    success: bool,
    error_message: Option<&str>,
) {
    if !is_category_enabled(&EventCategory::Swap) {
        return;
    }

    let payload = json!({
        "signature": signature,
        "input_mint": input_mint,
        "output_mint": output_mint,
        "amount_in": amount_in,
        "amount_out": amount_out,
        "success": success,
        "error_message": error_message,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = if success {
        Severity::Info
    } else {
        Severity::Error
    };
    let mint = if !crate::chains::adapter().is_native_asset(input_mint) {
        Some(input_mint.to_string())
    } else {
        Some(output_mint.to_string())
    };

    let event = Event::new(
        EventCategory::Swap,
        Some("swap".to_owned()),
        severity,
        mint,
        Some(signature.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// POOL EVENTS
// =============================================================================

/// Record a pool discovery or analysis event
pub async fn record_pool_event(
    pool_address: &str,
    program_id: &str,
    pool_type: &str,
    token_mint: &str,
    action: &str,
    details: Value,
) {
    if !is_category_enabled(&EventCategory::Pool) {
        return;
    }

    let payload = json!({
        "pool_address": pool_address,
        "program_id": program_id,
        "pool_type": pool_type,
        "action": action,
        "details": details,
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::Pool,
        Some(format!("{pool_type}_{action}")),
        Severity::Info,
        Some(token_mint.to_string()),
        Some(pool_address.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// POSITION EVENTS
// =============================================================================

/// Record a position lifecycle event
pub async fn record_position_event(
    position_id: &str,
    mint: &str,
    action: &str,
    entry_signature: Option<&str>,
    exit_signature: Option<&str>,
    amount_sol: f64,
    amount_tokens: impl Into<RawAmount>,
    pnl_sol: Option<f64>,
    pnl_percent: Option<f64>,
) {
    if !is_category_enabled(&EventCategory::Position) {
        return;
    }

    let payload = json!({
        "position_id": position_id,
        "action": action,
        "entry_signature": entry_signature,
        "exit_signature": exit_signature,
        "amount_sol": amount_sol,
        "amount_tokens": raw_amount_payload(amount_tokens.into()),
        "pnl_sol": pnl_sol,
        "pnl_percent": pnl_percent,
        "event_time": Utc::now().to_rfc3339()
    });

    let reference_id = entry_signature.or(exit_signature).map(|s| s.to_string());

    let event = Event::new(
        EventCategory::Position,
        Some(action.to_string()),
        Severity::Info,
        Some(mint.to_string()),
        reference_id,
        payload,
    );

    crate::events::record_safe(event).await;
}

/// A token amount as a JSON number while it fits u64, otherwise as its exact decimal string.
fn raw_amount_payload(amount: RawAmount) -> Value {
    u64::try_from(amount).map_or_else(|_| Value::String(amount.to_string()), Value::from)
}

/// Record a position event with flexible payload (for complex position operations)
pub async fn record_position_event_flexible(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Position) {
        return;
    }

    let mut payload_obj: Map<String, Value> = match payload {
        Value::Object(obj) => obj,
        Value::Null => Map::new(),
        other => {
            let mut map = Map::new();
            map.insert("details".to_owned(), other);
            map
        }
    };

    payload_obj
        .entry("subtype".to_owned())
        .or_insert_with(|| Value::String(subtype.to_string()));
    payload_obj
        .entry("event_time".to_owned())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));

    let event = Event::new(
        EventCategory::Position,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_event_amounts_stay_numbers_within_u64() {
        assert_eq!(
            json!({ "amount_tokens": raw_amount_payload(2_167_137u64.into()) }).to_string(),
            json!({ "amount_tokens": 2_167_137u64 }).to_string()
        );
        assert_eq!(raw_amount_payload(u64::MAX.into()), json!(u64::MAX));
        assert_eq!(
            raw_amount_payload(RawAmount::new(u128::from(u64::MAX) + 1)),
            json!("18446744073709551616")
        );
    }
}
