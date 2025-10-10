use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    config::with_config,
    logger::{log, LogTag},
    transactions::{database::TransactionListFilters, get_transaction_database},
    webserver::ws::{hub::WsHub, topics},
};

pub fn start(hub: Arc<WsHub>) {
    tokio::spawn(run(hub));
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "ws.sources.transactions started (polling mode)",
        );
    }
}

async fn run(hub: Arc<WsHub>) {
    // Get poll interval from config (default 2s)
    let poll_interval_ms = with_config(|cfg| cfg.webserver.transactions_poll_interval_ms);

    let mut ticker = interval(Duration::from_millis(poll_interval_ms));
    let mut last_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;

    loop {
        ticker.tick().await;

        let db = match get_transaction_database().await {
            Some(db) => db,
            None => {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "DEBUG",
                        "ws.sources.transactions: DB not available, skipping poll",
                    );
                }
                continue;
            }
        };

        // Build filters for new/updated transactions
        let mut filters = TransactionListFilters::default();
        filters.time_from = last_timestamp;

        // Fetch recent transactions (limit to 50 to avoid large payloads)
        let result = match db.list_transactions(&filters, None, 50).await {
            Ok(r) => r,
            Err(e) => {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "ERROR",
                        &format!(
                            "ws.sources.transactions: Failed to list transactions: {}",
                            e
                        ),
                    );
                }
                continue;
            }
        };

        if result.items.is_empty() {
            continue;
        }

        // Update watermark to most recent transaction
        if let Some(newest) = result.items.first() {
            last_timestamp = Some(newest.timestamp);
        }

        // Publish each new transaction as an activity event
        let count = result.items.len();
        for item in &result.items {
            let seq = hub.next_seq("transactions.activity").await;
            let envelope = topics::transactions::transaction_to_envelope(item, seq);
            hub.broadcast(envelope).await;
        }

        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!("ws.sources.transactions: Published {} activities", count),
            );
        }
    }
}
