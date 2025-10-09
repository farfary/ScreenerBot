use std::sync::Arc;
use tokio::time::{interval, Duration};

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
    // Phase 1 cleanup: slow cadence to 10s until Phase 2 demand-gating is wired.
    // TODO(Phase 2): restore dynamic cadence with explicit subscription tracking instead of fixed sleep.
    let mut ticker = interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        let snapshot =
            crate::webserver::routes::services::gather_services_overview_snapshot().await;
        let seq = hub.next_seq("services.metrics").await;
        let envelope = topics::services::services_to_envelope(&snapshot, seq);
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
