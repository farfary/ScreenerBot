/// Events Module - Persistent Event Recording System
///
/// This module provides a comprehensive event recording system that captures all bot activities
/// in a structured, queryable format. Events are stored in a dedicated SQLite database
/// separate from logging to enable analytics, debugging, and historical analysis.
///
/// ## Features:
/// - **Non-blocking recording**: Uses async channels to avoid blocking hot paths
/// - **Categorized events**: Organized by system component (swap, pool, transaction, etc.)
/// - **Structured data**: JSON payloads with typed metadata for flexible queries
/// - **High performance**: Connection pooling and batched writes
/// - **Maintenance**: Automatic cleanup of old events
///
/// ## Usage:
/// ```rust
/// use screenerbot::events::{self, Event, EventCategory, Severity};
/// use serde_json::json;
///
/// // Initialize at startup
/// events::init().await?;
///
/// // Record events
/// let event = Event::new(
///     EventCategory::Swap,
///     Some("JupiterQuote".to_string()),
///     Severity::Info,
///     Some(mint.to_string()),
///     Some(tx_signature.to_string()),
///     json!({
///         "amount_in": amount_in,
///         "amount_out": amount_out,
///         "slippage": slippage_bps
///     })
/// );
/// events::record(event).await?;
///
/// // Query events
/// let recent_swaps = events::recent(EventCategory::Swap, 100).await?;
/// ```
///
/// ## Integration:
/// Events complement but do not replace the logging system. Logs are for real-time
/// monitoring and debugging; events are for persistent analysis and metrics.
pub mod db;
pub mod maintenance;
pub mod types;

use crate::logger::{ log, LogTag };
use db::EventsDatabase;
pub use maintenance::{
    get_events_summary,
    record_entry_event,
    record_pool_event,
    record_position_event,
    record_security_event,
    record_swap_event,
    record_system_event,
    record_token_event,
    record_transaction_event,
    search_events,
    start_maintenance_task,
};
use once_cell::sync::{ Lazy, OnceCell };
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{ broadcast, mpsc, Mutex, RwLock };
pub use types::{ Event, EventCategory, Severity };

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
static EVENT_WRITER: Lazy<Arc<Mutex<Option<EventWriter>>>> = Lazy::new(||
    Arc::new(Mutex::new(None))
);

/// Global database handle
pub static EVENTS_DB: OnceCell<Arc<EventsDatabase>> = OnceCell::new();

/// Global broadcaster for real-time event delivery
static EVENTS_BROADCAST_TX: OnceCell<broadcast::Sender<Event>> = OnceCell::new();

/// Ring buffer cache of recent events
const EVENTS_CACHE_CAPACITY: usize = 5000;
static EVENTS_CACHE: Lazy<Arc<RwLock<VecDeque<Event>>>> = Lazy::new(||
    Arc::new(RwLock::new(VecDeque::with_capacity(EVENTS_CACHE_CAPACITY)))
);

// =============================================================================
// PUBLIC API
// =============================================================================

/// Initialize the events system
/// Must be called once at application startup
pub async fn init() -> Result<(), String> {
    let mut writer_guard = EVENT_WRITER.lock().await;

    if writer_guard.is_some() {
        return Ok(()); // Already initialized
    }

    // Initialize database (fresh schema)
    let db = Arc::new(
        EventsDatabase::new().await.map_err(|e|
            format!("Failed to initialize events database: {}", e)
        )?
    );
    let _ = EVENTS_DB.set(db.clone());

    // Initialize broadcaster
    let (tx, _rx) = broadcast::channel::<Event>(1000);
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

    log(LogTag::System, "READY", "Events system initialized successfully");
    Ok(())
}

/// Record an event asynchronously
/// Non-blocking: events are queued and written by background task
pub async fn record(event: Event) -> Result<(), String> {
    let writer_guard = EVENT_WRITER.lock().await;

    if let Some(ref writer) = *writer_guard {
        writer.sender.send(event).await.map_err(|_| "Event channel closed".to_string())?;
        Ok(())
    } else {
        Err("Events system not initialized".to_string())
    }
}

/// Record an event with automatic error handling
/// Logs errors instead of propagating them to avoid disrupting main operations
pub async fn record_safe(event: Event) {
    if let Err(e) = record(event).await {
        log(LogTag::System, "WARN", &format!("Failed to record event: {}", e));
    }
}

/// Get recent events by category
pub async fn recent(category: EventCategory, limit: usize) -> Result<Vec<Event>, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.get_recent_events(Some(category), limit).await
}

/// Get recent events across all categories
pub async fn recent_all(limit: usize) -> Result<Vec<Event>, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.get_recent_events(None, limit).await
}

/// Get event counts by category for the last N hours
pub async fn count_by_category(
    since_hours: u64
) -> Result<std::collections::HashMap<String, u64>, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.get_event_counts_by_category(since_hours).await
}

/// Get events for a specific reference ID (e.g., transaction signature, pool address)
pub async fn by_reference(reference_id: &str, limit: usize) -> Result<Vec<Event>, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.get_events_by_reference(reference_id, limit).await
}

/// Get events for a specific token mint
pub async fn by_mint(mint: &str, limit: usize) -> Result<Vec<Event>, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.get_events_by_mint(mint, limit).await
}

/// Force cleanup of old events (normally handled automatically)
pub async fn cleanup_old_events() -> Result<usize, String> {
    let db = EVENTS_DB.get().ok_or_else(|| "Events system not initialized".to_string())?;
    db.cleanup_old_events().await
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

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(BATCH_TIMEOUT_MS));

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

    log(LogTag::System, "INFO", "Events writer task stopped");
}

/// Write a batch of events to database
async fn write_batch(db: &EventsDatabase, batch: &mut Vec<Event>) {
    if batch.is_empty() {
        return;
    }

    if let Err(e) = db.insert_events(batch.as_mut_slice()).await {
        log(LogTag::System, "ERROR", &format!("Failed to write event batch: {}", e));
    }

    // On success (or even if some failed), push to cache and broadcast
    push_to_cache_and_broadcast(batch).await;
    batch.clear();
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
