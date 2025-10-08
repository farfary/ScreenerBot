/// WebSocket producers - Bridge internal broadcasts to WsHub
///
/// Subscribes to internal broadcast channels and feeds the WsHub with
/// typed topic messages. Handles broadcast errors gracefully without
/// breaking the WebSocket connection.

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    arguments::is_debug_webserver_enabled,
    events,
    logger::{log, LogTag},
    pools, positions,
    webserver::{
        services_broadcast, status_broadcast,
        ws::{hub::WsHub, topics},
    },
};

/// Start all producers (spawn background tasks)
pub fn start_producers(hub: Arc<WsHub>) {
    // Events producer
    if let Some(rx) = events::subscribe() {
        tokio::spawn(events_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Events producer started");
        }
    }

    // Positions producer
    if let Some(rx) = positions::subscribe_positions() {
        tokio::spawn(positions_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Positions producer started");
        }
    }

    // Prices producer
    if let Some(rx) = pools::subscribe_prices() {
        tokio::spawn(prices_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Prices producer started");
        }
    }

    // Status producer
    if let Some(rx) = status_broadcast::subscribe() {
        tokio::spawn(status_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Status producer started");
        }
    }

    // Services producer
    if let Some(rx) = services_broadcast::subscribe() {
        tokio::spawn(services_producer(hub.clone(), rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Services producer started");
        }
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

/// Status producer task
async fn status_producer(
    hub: Arc<WsHub>,
    mut rx: broadcast::Receiver<status_broadcast::StatusSnapshot>,
) {
    loop {
        match rx.recv().await {
            Ok(snapshot) => {
                let envelope = topics::status::status_to_envelope(&snapshot, hub.next_seq("system.status"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Status producer lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "Status broadcaster closed, producer exiting",
                );
                break;
            }
        }
    }
}

/// Services producer task
async fn services_producer(
    hub: Arc<WsHub>,
    mut rx: broadcast::Receiver<crate::webserver::routes::services::ServicesOverviewResponse>,
) {
    loop {
        match rx.recv().await {
            Ok(snapshot) => {
                let envelope = topics::services::services_to_envelope(&snapshot, hub.next_seq("services.metrics"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Services producer lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "Services broadcaster closed, producer exiting",
                );
                break;
            }
        }
    }
}
