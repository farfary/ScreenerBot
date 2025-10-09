use std::sync::Arc;
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::{hub::WsHub, topics},
};

pub fn start(hub: Arc<WsHub>) {
    tokio::spawn(run(hub));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "ws.sources.services started");
    }
}

async fn run(hub: Arc<WsHub>) {
    const TOPIC: &str = "services.metrics";

    loop {
        hub.wait_for_subscribers(TOPIC).await;

        let active = hub.topic_subscriber_count(TOPIC).await;
        log(
            LogTag::Webserver,
            "INFO",
            &format!(
                "ws.sources.services streaming activated (subscribers={})",
                active
            ),
        );

        // NOTE: Initial snapshot is now handled by connection.rs::send_services_snapshot()
        // with proper SnapshotBegin/End messages and request correlation.
        // This source only sends periodic delta updates (if needed in future).

        let mut ticker = interval(Duration::from_secs(10));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;

            if !hub.has_subscribers(TOPIC).await {
                let remaining = hub.topic_subscriber_count(TOPIC).await;
                log(
                    LogTag::Webserver,
                    "INFO",
                    &format!(
                        "ws.sources.services streaming paused (subscribers={})",
                        remaining
                    ),
                );
                break;
            }

            // Periodic updates: send full snapshot as a delta update (no begin/end markers)
            // Frontend will merge this with existing state
            publish_delta(&hub).await;
        }
    }
}

async fn publish_delta(hub: &Arc<WsHub>) {
    let snapshot = crate::webserver::routes::services::gather_services_overview_snapshot().await;
    let seq = hub.next_seq("services.metrics").await;
    let envelope = topics::services::services_to_envelope(&snapshot, seq);
    hub.broadcast(envelope).await;

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "ws.sources.services delta: services={}, unhealthy={}",
                snapshot.services.len(),
                snapshot.summary.unhealthy_services
            ),
        );
    }
}
