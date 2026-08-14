//! Promo generator for the events monitor.
//!
//! The events table is the one monitoring surface with nothing behind it in a
//! fresh session: the real feed only fills as a long-running instance trades, so
//! a capture of a freshly started app photographs "No results found". These rows
//! describe the same session every other fixture describes — the open positions
//! from `data.rs` and the services the app actually runs.

use chrono::{Duration, Utc};
use serde_json::json;

use crate::webserver::routes::events::types::{EventResponse, EventsListResponse};

use super::data::PROMO_OPEN_TOKENS;

/// One event line: (seconds ago, category, subtype, severity, message, mint index).
///
/// The mint index points into `PROMO_OPEN_TOKENS` so a token-scoped event names a
/// token the rest of the session already holds a position in.
type PromoEvent = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<usize>,
);

const PROMO_EVENTS: &[PromoEvent] = &[
    (
        12,
        "swap",
        "buy_executed",
        "info",
        "Buy filled at 0.5% slippage via Jupiter",
        Some(0),
    ),
    (
        48,
        "position",
        "opened",
        "info",
        "Position opened — manual management off",
        Some(0),
    ),
    (
        95,
        "filtering",
        "passed",
        "info",
        "Token passed all filter stages",
        Some(1),
    ),
    (
        140,
        "pool",
        "registered",
        "info",
        "Pool registered for live pricing (Raydium CPMM)",
        Some(1),
    ),
    (
        183,
        "trader",
        "evaluated",
        "info",
        "Exit conditions evaluated for 10 open positions",
        None,
    ),
    (
        226,
        "ohlcv",
        "backfill_complete",
        "info",
        "Backfill complete for 5m candles",
        Some(2),
    ),
    (
        271,
        "token",
        "decimals_cached",
        "debug",
        "Token decimals resolved and cached",
        Some(2),
    ),
    (
        318,
        "rpc",
        "rate_limited",
        "warn",
        "RPC provider throttled — backing off 2s",
        None,
    ),
    (
        364,
        "swap",
        "quote_refreshed",
        "debug",
        "Swap quote refreshed before submit",
        Some(3),
    ),
    (
        410,
        "position",
        "partial_exit",
        "info",
        "Partial exit executed at +18.4%",
        Some(3),
    ),
    (
        455,
        "wallet",
        "balance_changed",
        "info",
        "Wallet balance updated after settlement",
        None,
    ),
    (
        502,
        "filtering",
        "rejected",
        "info",
        "Token rejected — liquidity below minimum",
        None,
    ),
    (
        548,
        "connectivity",
        "restored",
        "info",
        "Connectivity restored — all services healthy",
        None,
    ),
    (
        593,
        "transaction",
        "confirmed",
        "info",
        "Transaction confirmed in 2 slots",
        Some(4),
    ),
    (
        640,
        "security",
        "authority_flagged",
        "warn",
        "Mint authority still active — token skipped",
        None,
    ),
    (
        688,
        "pool",
        "price_stale",
        "warn",
        "Pool price older than 5s — refetching accounts",
        Some(5),
    ),
    (
        735,
        "system",
        "service_started",
        "info",
        "OHLCV monitoring service started",
        None,
    ),
    (
        781,
        "trader",
        "loss_limiter",
        "warn",
        "Daily loss limiter armed at 0.35 SOL",
        None,
    ),
    (
        828,
        "ohlcv",
        "gap_filled",
        "info",
        "Interior candle gap bridged (15m)",
        Some(6),
    ),
    (
        874,
        "scheduled_task",
        "cleanup",
        "info",
        "Database cleanup removed 1,204 stale rows",
        None,
    ),
];

/// Generate the promo events feed, honouring the list filters the page sends.
pub fn get_promo_events(
    limit: usize,
    category: Option<&str>,
    severity: Option<&str>,
    mint: Option<&str>,
    search: Option<&str>,
) -> EventsListResponse {
    let now = Utc::now();
    let search_lower = search.map(str::to_lowercase);

    let mut events = Vec::new();
    // Newest first, and the id descends with age so the page's cursor logic sees
    // the same ordering the database would give it.
    let mut id = PROMO_EVENTS.len() as i64;

    for (age_secs, event_category, subtype, event_severity, message, token) in PROMO_EVENTS.iter() {
        let event_mint = token.map(|index| PROMO_OPEN_TOKENS[index].2.to_owned());
        let symbol = token.map(|index| PROMO_OPEN_TOKENS[index].0);

        let matches = category.is_none_or(|value| value == *event_category)
            && severity.is_none_or(|value| value == *event_severity)
            && mint.is_none_or(|value| event_mint.as_deref() == Some(value))
            && search_lower
                .as_deref()
                .is_none_or(|needle| message.to_lowercase().contains(needle));

        if !matches {
            id -= 1;
            continue;
        }

        let event_time = now - Duration::seconds(*age_secs);
        events.push(EventResponse {
            id,
            event_time: event_time.to_rfc3339(),
            category: (*event_category).to_owned(),
            subtype: Some((*subtype).to_owned()),
            severity: (*event_severity).to_owned(),
            mint: event_mint,
            reference_id: None,
            message: (*message).to_owned(),
            payload: json!({ "message": message, "symbol": symbol }),
            created_at: event_time.to_rfc3339(),
        });
        id -= 1;
    }

    events.truncate(limit);
    let count = events.len();
    let max_id = events.first().map(|event| event.id).unwrap_or(0);

    EventsListResponse {
        events,
        count,
        total_count: Some(count as i64),
        max_id,
        timestamp: now.to_rfc3339(),
    }
}
