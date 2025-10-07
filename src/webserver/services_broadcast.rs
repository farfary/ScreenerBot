use once_cell::sync::OnceCell;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

use crate::webserver::routes::services::{
    gather_services_overview_snapshot, ServicesOverviewResponse,
};

const SERVICES_BROADCAST_CAPACITY: usize = 64;

static SERVICES_BROADCAST_TX: OnceCell<broadcast::Sender<ServicesOverviewResponse>> =
    OnceCell::new();

/// Initialize services broadcaster and return the first receiver
pub fn initialize_services_broadcaster() -> broadcast::Receiver<ServicesOverviewResponse> {
    let (tx, rx) = broadcast::channel(SERVICES_BROADCAST_CAPACITY);

    match SERVICES_BROADCAST_TX.set(tx) {
        Ok(_) => rx,
        Err(_) => SERVICES_BROADCAST_TX
            .get()
            .expect("Services broadcaster exists")
            .subscribe(),
    }
}

/// Subscribe to services snapshot stream
pub fn subscribe() -> Option<broadcast::Receiver<ServicesOverviewResponse>> {
    SERVICES_BROADCAST_TX.get().map(|tx| tx.subscribe())
}

/// Start periodic broadcast of services overview snapshots
pub fn start_services_broadcaster(interval_secs: u64) -> tokio::task::JoinHandle<()> {
    if SERVICES_BROADCAST_TX.get().is_none() {
        initialize_services_broadcaster();
    }

    let tx = SERVICES_BROADCAST_TX
        .get()
        .expect("Services broadcaster initialized")
        .clone();

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs.max(1)));

        loop {
            ticker.tick().await;

            let snapshot = gather_services_overview_snapshot().await;
            let _ = tx.send(snapshot);
        }
    })
}
