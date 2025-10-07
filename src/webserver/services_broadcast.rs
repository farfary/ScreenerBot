use once_cell::sync::OnceCell;
use tokio::sync::broadcast;
use tokio::time::{ interval, Duration };

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{ log, LogTag },
    webserver::routes::services::{ gather_services_overview_snapshot, ServicesOverviewResponse },
};

const SERVICES_BROADCAST_CAPACITY: usize = 64;

static SERVICES_BROADCAST_TX: OnceCell<broadcast::Sender<ServicesOverviewResponse>> =
    OnceCell::new();

/// Initialize services broadcaster and return the first receiver
pub fn initialize_services_broadcaster() -> broadcast::Receiver<ServicesOverviewResponse> {
    let (tx, rx) = broadcast::channel(SERVICES_BROADCAST_CAPACITY);

    match SERVICES_BROADCAST_TX.set(tx) {
        Ok(_) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Services broadcaster initialized (capacity: {})", SERVICES_BROADCAST_CAPACITY)
                );
            }
            rx
        }
        Err(_) => SERVICES_BROADCAST_TX.get().expect("Services broadcaster exists").subscribe(),
    }
}

/// Subscribe to services snapshot stream
pub fn subscribe() -> Option<broadcast::Receiver<ServicesOverviewResponse>> {
    let rx = SERVICES_BROADCAST_TX.get().map(|tx| tx.subscribe());
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "New services broadcast subscriber created (active_tx={}, has_rx={})",
                SERVICES_BROADCAST_TX.get().is_some(),
                rx.is_some()
            )
        );
    }
    rx
}

/// Start periodic broadcast of services overview snapshots
pub fn start_services_broadcaster(interval_secs: u64) -> tokio::task::JoinHandle<()> {
    if SERVICES_BROADCAST_TX.get().is_none() {
        initialize_services_broadcaster();
    }

    let tx = SERVICES_BROADCAST_TX.get().expect("Services broadcaster initialized").clone();

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!("Starting services broadcaster task (interval: {}s)", interval_secs.max(1))
        );
    }

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs.max(1)));
        let mut tick_count: u64 = 0;

        loop {
            ticker.tick().await;
            tick_count += 1;

            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Services broadcaster tick #{}", tick_count)
                );
            }

            let snapshot = gather_services_overview_snapshot().await;
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Broadcasting services snapshot (services={}, unhealthy={})",
                        snapshot.services.len(),
                        snapshot.summary.unhealthy_services + snapshot.summary.degraded_services
                    )
                );
            }

            if let Err(error) = tx.send(snapshot) {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "DEBUG",
                        &format!("No active listeners for services snapshot broadcast: {}", error)
                    );
                }
            }
        }
    })
}
