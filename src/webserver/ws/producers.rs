/// WebSocket producers - Feed data to WsHub
///
/// Subscribes to internal broadcast channels and periodically gathers
/// system state, then feeds the WsHub with typed topic messages.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    events,
    logger::{log, LogTag},
    pools, positions,
    webserver::{
        routes::services::gather_services_overview_snapshot,
        ws::{hub::WsHub, snapshots, topics},
    },
};

/// Start all producers (spawn background tasks)
pub fn start_producers(hub: Arc<WsHub>) {
    // Events producer (from broadcast)
    if let Some(rx) = events::subscribe() {
        tokio::spawn(events_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Events producer started");
        }
    }

    // Positions producer (from broadcast)
    if let Some(rx) = positions::subscribe_positions() {
        tokio::spawn(positions_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Positions producer started");
        }
    }

    // Prices producer (from broadcast)
    if let Some(rx) = pools::subscribe_prices() {
        tokio::spawn(prices_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Prices producer started");
        }
    }

    // Status producer (periodic gather)
    tokio::spawn(status_producer(hub.clone()));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Status producer started");
    }

    // Services producer (periodic gather)
    tokio::spawn(services_producer(hub.clone()));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Services producer started");
    }
}

/// Events producer task
async fn events_producer(
    hub: Arc<WsHub>,
    mut rx: broadcast::Receiver<events::Event>,
) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                let envelope = topics::events::event_to_envelope(&event, hub.next_seq("events.new"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Events producer lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "Events broadcaster closed, producer exiting",
                );
                break;
            }
        }
    }
}

/// Positions producer task
async fn positions_producer(
    hub: Arc<WsHub>,
    mut rx: broadcast::Receiver<positions::PositionUpdate>,
) {
    loop {
        match rx.recv().await {
            Ok(update) => {
                let envelope = topics::positions::position_to_envelope(&update, hub.next_seq("positions.update"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Positions producer lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "Positions broadcaster closed, producer exiting",
                );
                break;
            }
        }
    }
}

/// Prices producer task
async fn prices_producer(
    hub: Arc<WsHub>,
    mut rx: broadcast::Receiver<pools::PriceUpdate>,
) {
    loop {
        match rx.recv().await {
            Ok(update) => {
                let envelope = topics::prices::price_to_envelope(&update, hub.next_seq("prices.update"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Prices producer lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "Prices broadcaster closed, producer exiting",
                );
                break;
            }
        }
    }
}

/// Status producer task (periodic gather)
async fn status_producer(hub: Arc<WsHub>) {
    let interval_secs = config::with_config(|cfg| cfg.webserver.websocket.heartbeat_secs.max(2));
    let mut ticker = interval(Duration::from_secs(interval_secs));
    
    loop {
        ticker.tick().await;
        
        let snapshot = snapshots::gather_status_snapshot().await;
        let envelope = topics::status::status_to_envelope(&snapshot, hub.next_seq("system.status"));
        hub.broadcast(envelope).await;
        
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "Status snapshot broadcast (positions={}, ws_connections={})",
                    snapshot.open_positions, snapshot.ws_connections
                ),
            );
        }
    }
}

/// Services producer task (periodic gather)
async fn services_producer(hub: Arc<WsHub>) {
    let interval_secs = config::with_config(|cfg| cfg.webserver.websocket.heartbeat_secs.max(3));
    let mut ticker = interval(Duration::from_secs(interval_secs));
    
    loop {
        ticker.tick().await;
        
        let snapshot = gather_services_overview_snapshot().await;
        let envelope = topics::services::services_to_envelope(&snapshot, hub.next_seq("services.metrics"));
        hub.broadcast(envelope).await;
        
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "Services snapshot broadcast (services={}, unhealthy={})",
                    snapshot.services.len(),
                    snapshot.summary.unhealthy_services
                ),
            );
        }
    }
}
