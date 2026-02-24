//! Wallet balance monitor service metrics

static WALLET_METRICS_OPERATIONS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_ERRORS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_SNAPSHOTS_TAKEN: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_FLOW_SYNCS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

pub(in super::super) fn increment_operations() {
    WALLET_METRICS_OPERATIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(in super::super) fn increment_errors() {
    WALLET_METRICS_ERRORS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(in super::super) fn increment_snapshots() {
    WALLET_METRICS_SNAPSHOTS_TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(in super::super) fn increment_flow_syncs() {
    WALLET_METRICS_FLOW_SYNCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub fn get_wallet_service_metrics() -> (u64, u64, u64, u64) {
    (
        WALLET_METRICS_OPERATIONS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_ERRORS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_SNAPSHOTS_TAKEN.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_FLOW_SYNCS.load(std::sync::atomic::Ordering::Relaxed),
    )
}
