// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Events Module - Persistent Event Recording System
//!
//! This module provides a comprehensive event recording system that captures all bot activities
//! in a structured, queryable format. Events are stored in a dedicated SQLite database
//! separate from logging to enable analytics, debugging, and historical analysis.
//!
//! ## Features:
//! - **Non-blocking recording**: Uses async channels to avoid blocking hot paths
//! - **Categorized events**: Organized by system component (swap, pool, transaction, etc.)
//! - **Structured data**: JSON payloads with typed metadata for flexible queries
//! - **High performance**: Connection pooling and batched writes
//! - **Maintenance**: Automatic cleanup of old events
//!
//! ## Usage:
//! ```rust
//! use screenerbot::events::{self, Event, EventCategory, Severity};
//! use serde_json::json;
//!
//! // Initialize at startup
//! events::init().await?;
//!
//! // Record events
//! let event = Event::new(
//!     EventCategory::Swap,
//!     Some("JupiterQuote".to_owned()),
//!     Severity::Info,
//!     Some(mint.to_string()),
//!     Some(tx_signature.to_string()),
//!     json!({
//!         "amount_in": amount_in,
//!         "amount_out": amount_out,
//!         "slippage": slippage_bps
//!     })
//! );
//! events::record(event).await?;
//!
//! // Query events
//! let recent_swaps = events::recent(EventCategory::Swap, 100).await?;
//! ```
//!
//! ## Integration:
//! Events complement but do not replace the logging system. Logs are for real-time
//! monitoring and debugging; events are for persistent analysis and metrics.

pub mod database;
mod display_text;
mod error;
#[cfg(test)]
mod label_tests;
pub use database as db;
pub use display_text::{with_text, ScheduledTaskOutcome};
pub mod maintenance;
pub mod recorders;
pub mod types;

#[cfg(test)]
mod producer_guard_tests;
#[cfg(test)]
mod producer_text_tests;

use crate::logger::{self, LogTag};
use db::EventsDatabase;
pub use error::{Error, Result};
pub use maintenance::{get_events_summary, search_events};
pub use recorders::{
    record_api_event, record_connectivity_event, record_filtering_event, record_ohlcv_event,
    record_pool_event, record_position_event, record_position_event_flexible, record_rpc_event,
    record_scheduled_task_event, record_security_event, record_swap_event, record_system_event,
    record_token_event, record_trader_event, record_transaction_event, record_wallet_event,
};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
pub use types::{Event, EventCategory, Severity};

// =============================================================================
// GLOBAL EVENT SYSTEM
// =============================================================================

/// Channel capacity for event recording (prevents memory buildup under high load)
const EVENT_CHANNEL_CAPACITY: usize = 10000;

/// Event writer handle for async recording
struct EventWriter {
    sender: mpsc::Sender<Event>,
    _handle: tokio::task::JoinHandle<()>,
}

/// Global event writer instance
static EVENT_WRITER: LazyLock<Arc<Mutex<Option<EventWriter>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Set once an initialization attempt failed and was reported; the next
/// successful attempt clears it. Recording stops retrying while it is set, so
/// one failure is one log line and not one per event.
static INIT_FAILED: AtomicBool = AtomicBool::new(false);

/// Global database handle
pub static EVENTS_DB: OnceLock<Arc<EventsDatabase>> = OnceLock::new();

/// The events store, or why there is none: switched off in settings reads
/// differently from a store that has not started yet.
pub(crate) fn database() -> Result<&'static Arc<EventsDatabase>> {
    if let Some(db) = EVENTS_DB.get() {
        return Ok(db);
    }
    if crate::config::with_config(|c| c.events.enabled) {
        Err(Error::NotInitialized)
    } else {
        Err(Error::Disabled)
    }
}

/// Run one synchronous events store operation on the blocking pool, so the runtime worker
/// that awaits it never holds a SQLite connection.
pub(crate) async fn on_blocking_pool<T, F>(db: &Arc<EventsDatabase>, operation: F) -> Result<T>
where
    F: FnOnce(&EventsDatabase) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let store = Arc::clone(db);
    tokio::task::spawn_blocking(move || operation(&store))
        .await
        .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))
        .and_then(|result| result)
}

/// Global broadcaster for real-time event delivery
static EVENTS_BROADCAST_TX: OnceLock<broadcast::Sender<Event>> = OnceLock::new();

/// Ring buffer cache of recent events
const EVENTS_CACHE_CAPACITY: usize = 5000;
static EVENTS_CACHE: LazyLock<Arc<RwLock<VecDeque<Event>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(VecDeque::with_capacity(EVENTS_CACHE_CAPACITY))));

// =============================================================================
// PUBLIC API
// =============================================================================

/// Initialize the events system.
///
/// Idempotent: the first call opens the store, starts the writer and the
/// maintenance task; later calls return immediately. Boot and a runtime switch
/// of `events.enabled` share this one path.
pub async fn init() -> Result<()> {
    let mut writer_guard = EVENT_WRITER.lock().await;

    if writer_guard.is_some() {
        return Ok(()); // Already initialized
    }

    // Initialize database (fresh schema)
    let db = tokio::task::spawn_blocking(EventsDatabase::new)
        .await
        .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))??;
    let db = Arc::new(db);
    let _ = EVENTS_DB.set(db.clone());

    // Initialize broadcaster with larger capacity to reduce lag frequency
    // (was 1000, increased to 5000 to handle bursts of events during active trading/discovery)
    let (tx, _rx) = broadcast::channel::<Event>(5000);
    let _ = EVENTS_BROADCAST_TX.set(tx);

    // Create channel for async event recording
    let (sender, receiver) = mpsc::channel::<Event>(EVENT_CHANNEL_CAPACITY);

    // Spawn background writer task
    let handle = tokio::spawn(async move {
        event_writer_task(receiver, db).await;
    });

    // Store writer handle
    *writer_guard = Some(EventWriter {
        sender,
        _handle: handle,
    });

    maintenance::start_maintenance_task();

    logger::info(LogTag::System, "Events system initialized successfully");
    Ok(())
}

/// [`init`] with the failure reported once: the first failure after a success
/// (or after boot) is logged, repeats stay silent until an attempt succeeds.
/// Callers that must make the store exist (the record path, the service health
/// check) go through this entry point.
pub async fn ensure_initialized() -> Result<()> {
    match init().await {
        Ok(()) => {
            INIT_FAILED.store(false, Ordering::Relaxed);
            Ok(())
        }
        Err(error) => {
            if !INIT_FAILED.swap(true, Ordering::Relaxed) {
                logger::error(
                    LogTag::System,
                    &format!("Events system could not start: {error}"),
                );
            }
            Err(error)
        }
    }
}

/// The writer's channel, cloned out so no caller holds the writer lock while
/// sending or while initializing.
async fn writer_sender() -> Option<mpsc::Sender<Event>> {
    EVENT_WRITER
        .lock()
        .await
        .as_ref()
        .map(|writer| writer.sender.clone())
}

/// Record an event asynchronously
/// Non-blocking: events are queued and written by background task
/// Returns Ok(()) silently if events system is disabled in config
///
/// Enabled in settings but not started yet (switched on while the app runs):
/// the store is initialized here, once, then the event is queued. After a
/// failed initialization the call returns [`Error::NotInitialized`] without
/// retrying; [`ensure_initialized`] from the service health check retries.
pub async fn record(event: Event) -> Result<()> {
    // Silently skip if events system is disabled
    if !crate::config::with_config(|c| c.events.enabled) {
        return Ok(());
    }

    let sender = match writer_sender().await {
        Some(sender) => sender,
        None => {
            if INIT_FAILED.load(Ordering::Relaxed) {
                return Err(Error::NotInitialized);
            }
            ensure_initialized().await?;
            writer_sender().await.ok_or(Error::NotInitialized)?
        }
    };

    sender.send(event).await.map_err(|_| {
        Error::Internal(crate::errors::InternalError::InvariantViolation {
            message: "event channel closed".to_owned(),
        })
    })?;
    Ok(())
}

/// Record an event with automatic error handling
/// Logs errors instead of propagating them to avoid disrupting main operations
/// Silently returns if events system is disabled in config
pub async fn record_safe(event: Event) {
    // Silently skip if events system is disabled
    if !crate::config::with_config(|c| c.events.enabled) {
        return;
    }

    match record(event).await {
        Ok(()) => {}
        // Initialization failure is reported once by `ensure_initialized`.
        Err(Error::NotInitialized) => {}
        Err(e) => logger::warning(LogTag::System, &format!("Failed to record event: {e}")),
    }
}

/// Get recent events by category
pub async fn recent(category: EventCategory, limit: usize) -> Result<Vec<Event>> {
    let db = database()?;
    on_blocking_pool(db, move |store| {
        store.get_recent_events(Some(category), limit)
    })
    .await
}

/// Get recent events across all categories
pub async fn recent_all(limit: usize) -> Result<Vec<Event>> {
    let db = database()?;
    on_blocking_pool(db, move |store| store.get_recent_events(None, limit)).await
}

/// Get event counts by category for the last N hours
pub async fn count_by_category(since_hours: u64) -> Result<HashMap<String, u64>> {
    let db = database()?;
    on_blocking_pool(db, move |store| {
        store.get_event_counts_by_category(since_hours)
    })
    .await
}

/// Get events for a specific reference ID (e.g., transaction signature, pool address)
pub async fn by_reference(reference_id: &str, limit: usize) -> Result<Vec<Event>> {
    let db = database()?;
    let reference_id = reference_id.to_owned();
    on_blocking_pool(db, move |store| {
        store.get_events_by_reference(&reference_id, limit)
    })
    .await
}

/// Get events for a specific token mint
pub async fn by_mint(mint: &str, limit: usize) -> Result<Vec<Event>> {
    let db = database()?;
    let mint = mint.to_owned();
    on_blocking_pool(db, move |store| store.get_events_by_mint(&mint, limit)).await
}

/// Force cleanup of old events (normally handled automatically)
pub async fn cleanup_old_events() -> Result<usize> {
    let db = database()?;
    on_blocking_pool(db, EventsDatabase::cleanup_old_events).await
}

// =============================================================================
// HELPER MACROS
// =============================================================================

/// Convenience macro for recording info events
#[macro_export]
macro_rules! event_info {
    ($category:expr, $subtype:expr, $mint:expr, $reference_id:expr, $payload:expr) => {
        $crate::events::record_safe($crate::events::Event::new(
            $category,
            $subtype.map(|s| s.to_string()),
            $crate::events::Severity::Info,
            $mint.map(|m| m.to_string()),
            $reference_id.map(|r| r.to_string()),
            $payload,
        ))
        .await
    };
}

/// Convenience macro for recording warning events
#[macro_export]
macro_rules! event_warn {
    ($category:expr, $subtype:expr, $mint:expr, $reference_id:expr, $payload:expr) => {
        $crate::events::record_safe($crate::events::Event::new(
            $category,
            $subtype.map(|s| s.to_string()),
            $crate::events::Severity::Warn,
            $mint.map(|m| m.to_string()),
            $reference_id.map(|r| r.to_string()),
            $payload,
        ))
        .await
    };
}

/// Convenience macro for recording error events
#[macro_export]
macro_rules! event_error {
    ($category:expr, $subtype:expr, $mint:expr, $reference_id:expr, $payload:expr) => {
        $crate::events::record_safe($crate::events::Event::new(
            $category,
            $subtype.map(|s| s.to_string()),
            $crate::events::Severity::Error,
            $mint.map(|m| m.to_string()),
            $reference_id.map(|r| r.to_string()),
            $payload,
        ))
        .await
    };
}

// =============================================================================
// BACKGROUND WRITER TASK
// =============================================================================

/// Background task that writes events to database
/// Handles batching and error recovery
async fn event_writer_task(mut receiver: mpsc::Receiver<Event>, db: Arc<EventsDatabase>) {
    let mut batch = Vec::new();
    const BATCH_SIZE: usize = 100;
    const BATCH_TIMEOUT_MS: u64 = 1000;

    let mut interval = tokio::time::interval(Duration::from_millis(BATCH_TIMEOUT_MS));

    loop {
        tokio::select! {
            // Receive events
            maybe_event = receiver.recv() => {
                match maybe_event {
                    Some(event) => {
                        batch.push(event);

                        // Write batch when full
                        if batch.len() >= BATCH_SIZE {
                            write_batch(&db, &mut batch).await;
                        }
                    }
                    None => {
                        // Channel closed, write remaining events and exit
                        if !batch.is_empty() {
                            write_batch(&db, &mut batch).await;
                        }
                        break;
                    }
                }
            }

            // Timeout: flush any pending events
            _ = interval.tick() => {
                if !batch.is_empty() { write_batch(&db, &mut batch).await; }
            }
        }
    }

    logger::info(LogTag::System, "Events writer task stopped");
}

/// Write a batch of events to the database in one transaction on the blocking pool, so
/// the writer task never holds a SQLite connection on a runtime worker.
async fn write_batch(db: &Arc<EventsDatabase>, batch: &mut Vec<Event>) {
    if batch.is_empty() {
        return;
    }

    let mut events = std::mem::take(batch);
    let store = Arc::clone(db);
    let written = tokio::task::spawn_blocking(move || {
        let outcome = store.insert_events(&mut events);
        (events, outcome)
    })
    .await;
    let events = match written {
        Ok((events, outcome)) => {
            if let Err(e) = outcome {
                logger::error(LogTag::System, &format!("Failed to write event batch: {e}"));
            }
            events
        }
        Err(e) => {
            logger::error(
                LogTag::System,
                &format!("Event batch write did not complete: {e}"),
            );
            return;
        }
    };

    // On success (or even if some failed), push to cache and broadcast
    push_to_cache_and_broadcast(&events).await;
}

/// Push newly written events to in-memory cache and broadcast to subscribers
async fn push_to_cache_and_broadcast(events: &[Event]) {
    // Update cache
    {
        let mut cache = EVENTS_CACHE.write().await;
        for e in events {
            cache.push_front(e.clone());
            while cache.len() > EVENTS_CACHE_CAPACITY {
                cache.pop_back();
            }
        }
    }

    // Broadcast
    if let Some(tx) = EVENTS_BROADCAST_TX.get() {
        for e in events {
            let _ = tx.send(e.clone());
        }
    }
}

/// Subscribe to the global events broadcaster
pub fn subscribe() -> Option<broadcast::Receiver<Event>> {
    EVENTS_BROADCAST_TX.get().map(|tx| tx.subscribe())
}

/// Access the in-memory recent events cache (front = newest)
pub async fn cached_events_head(limit: usize) -> Vec<Event> {
    let cache = EVENTS_CACHE.read().await;
    cache.iter().take(limit).cloned().collect()
}
