//! Recorders whose payload is caller-supplied metadata plus catalog display
//! text: OHLCV, filtering, trader, RPC, API and scheduled-task events.

use super::super::display_text::{write_default_text, write_text, ScheduledTaskOutcome};
use super::super::maintenance::is_category_enabled;
use crate::events::{Event, EventCategory, Severity};
use crate::i18n::{ids, UiArg, UiText};
use chrono::Utc;
use serde_json::{Map, Value};

// =============================================================================
// OHLCV EVENTS
// =============================================================================

/// Record an OHLCV monitoring event with flexible payload metadata
pub async fn record_ohlcv_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Ohlcv) {
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
    write_default_text(&EventCategory::Ohlcv, &mut payload_obj, || {
        UiText::new(ids::EVENTS_OHLCV_DEFAULT).arg("subtype", UiArg::Text(subtype.to_string()))
    });

    let event = Event::new(
        EventCategory::Ohlcv,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// FILTERING EVENTS
// =============================================================================

/// Record a filtering event with flexible payload metadata
pub async fn record_filtering_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Filtering) {
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
    write_default_text(&EventCategory::Filtering, &mut payload_obj, || {
        UiText::new(ids::EVENTS_FILTERING_DEFAULT).arg("subtype", UiArg::Text(subtype.to_string()))
    });

    let event = Event::new(
        EventCategory::Filtering,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// TRADER EVENTS
// =============================================================================

/// Record a trader event with flexible payload metadata
pub async fn record_trader_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Trader) {
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
    write_default_text(&EventCategory::Trader, &mut payload_obj, || {
        UiText::new(ids::EVENTS_TRADER_DEFAULT).arg("subtype", UiArg::Text(subtype.to_string()))
    });

    let event = Event::new(
        EventCategory::Trader,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// RPC EVENTS (Solana RPC client)
// =============================================================================

/// Record an RPC client event (Solana RPC requests, responses, errors)
pub async fn record_rpc_event(method: &str, action: &str, severity: Severity, payload: Value) {
    if !is_category_enabled(&EventCategory::Rpc) {
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
        .entry("method".to_owned())
        .or_insert_with(|| Value::String(method.to_string()));
    payload_obj
        .entry("action".to_owned())
        .or_insert_with(|| Value::String(action.to_string()));
    payload_obj
        .entry("event_time".to_owned())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
    write_default_text(&EventCategory::Rpc, &mut payload_obj, || {
        UiText::new(ids::EVENTS_RPC_DEFAULT)
            .arg("method", UiArg::Text(method.to_string()))
            .arg("action", UiArg::Text(action.to_string()))
    });

    let event = Event::new(
        EventCategory::Rpc,
        Some(action.to_string()),
        severity,
        None,
        None,
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// API EVENTS (External APIs - DexScreener, GeckoTerminal, Jupiter API, etc.)
// =============================================================================

/// Record an external API event (DexScreener, GeckoTerminal, Jupiter API, RugCheck, etc.)
pub async fn record_api_event(api_name: &str, action: &str, severity: Severity, payload: Value) {
    if !is_category_enabled(&EventCategory::Api) {
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
        .entry("api".to_owned())
        .or_insert_with(|| Value::String(api_name.to_string()));
    payload_obj
        .entry("action".to_owned())
        .or_insert_with(|| Value::String(action.to_string()));
    payload_obj
        .entry("event_time".to_owned())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
    write_default_text(&EventCategory::Api, &mut payload_obj, || {
        UiText::new(ids::EVENTS_API_DEFAULT)
            .arg("api", UiArg::Text(api_name.to_string()))
            .arg("action", UiArg::Text(action.to_string()))
    });

    let event = Event::new(
        EventCategory::Api,
        Some(action.to_string()),
        severity,
        None,
        None,
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// SCHEDULED TASK EVENTS
// =============================================================================

/// Record a scheduled task event. The subtype is the stable outcome code; the
/// title is catalog text and `description` (model output or error) stays data.
pub fn record_scheduled_task_event(
    outcome: ScheduledTaskOutcome,
    task_name: &str,
    description: &str,
) {
    let title = outcome.title(task_name);
    let description = description.to_string();

    tokio::spawn(async move {
        if !is_category_enabled(&EventCategory::ScheduledTask) {
            return;
        }

        let mut payload = Map::new();
        payload.insert(
            "title".to_owned(),
            Value::String(title.render_source_plain()),
        );
        payload.insert("description".to_owned(), Value::String(description));
        payload.insert(
            "event_time".to_owned(),
            Value::String(Utc::now().to_rfc3339()),
        );
        write_text(&mut payload, &title);

        let event = Event::new(
            EventCategory::ScheduledTask,
            Some(outcome.code().to_owned()),
            outcome.severity(),
            None,
            None,
            Value::Object(payload),
        );

        crate::events::record_safe(event).await;
    });
}
