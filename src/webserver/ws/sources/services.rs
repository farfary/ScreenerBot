use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
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
    let interval_secs = config::with_config(|cfg| cfg.webserver.websocket.heartbeat_secs.max(3));
    let mut ticker = interval(Duration::from_secs(interval_secs));
    loop {
        ticker.tick().await;
        let snapshot =
            crate::webserver::routes::services::gather_services_overview_snapshot().await;
        let envelope =
            topics::services::services_to_envelope(&snapshot, hub.next_seq("services.metrics"));
        hub.broadcast(envelope).await;
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "ws.sources.services snapshot: services={}, unhealthy={}",
                    snapshot.services.len(),
                    snapshot.summary.unhealthy_services
                ),
            );
        }
    }
}
