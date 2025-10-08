use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    logger::{log, LogTag},
    webserver::ws::{hub::WsHub, snapshots, topics},
};

pub fn start(hub: Arc<WsHub>) {
    tokio::spawn(run(hub));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "ws.sources.status started");
    }
}

async fn run(hub: Arc<WsHub>) {
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
                    "ws.sources.status snapshot: positions={}, ws_connections={}",
                    snapshot.open_positions, snapshot.ws_connections
                ),
            );
        }
    }
}
