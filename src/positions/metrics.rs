use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use chrono::Utc;

#[derive(Debug, Clone, Default)]
pub struct ProceedsMetricsSnapshot {
    pub accepted_quotes: u64,
    pub rejected_quotes: u64,
    pub accepted_profit_quotes: u64,
    pub accepted_loss_quotes: u64,
    pub total_shortfall_bps_sum: u64,
    pub worst_shortfall_bps: u64,
    pub average_shortfall_bps: f64,
    pub last_update_unix: i64,
}

struct ProceedsMetricsInternal {
    accepted_quotes: AtomicU64,
    rejected_quotes: AtomicU64,
    accepted_profit_quotes: AtomicU64,
    accepted_loss_quotes: AtomicU64,
    total_shortfall_bps_sum: AtomicU64,
    worst_shortfall_bps: AtomicU64,
    last_update_unix: AtomicU64,
}

impl ProceedsMetricsInternal {
    const fn new() -> Self {
        Self {
            accepted_quotes: AtomicU64::new(0),
            rejected_quotes: AtomicU64::new(0),
            accepted_profit_quotes: AtomicU64::new(0),
            accepted_loss_quotes: AtomicU64::new(0),
            total_shortfall_bps_sum: AtomicU64::new(0),
            worst_shortfall_bps: AtomicU64::new(0),
            last_update_unix: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> ProceedsMetricsSnapshot {
        let profit_count = self.accepted_profit_quotes.load(Ordering::Relaxed);
        let total_shortfall_sum = self.total_shortfall_bps_sum.load(Ordering::Relaxed);
        
        ProceedsMetricsSnapshot {
            accepted_quotes: self.accepted_quotes.load(Ordering::Relaxed),
            rejected_quotes: self.rejected_quotes.load(Ordering::Relaxed),
            accepted_profit_quotes: profit_count,
            accepted_loss_quotes: self.accepted_loss_quotes.load(Ordering::Relaxed),
            total_shortfall_bps_sum: total_shortfall_sum,
            worst_shortfall_bps: self.worst_shortfall_bps.load(Ordering::Relaxed),
            average_shortfall_bps: if profit_count > 0 {
                (total_shortfall_sum as f64) / (profit_count as f64)
            } else {
                0.0
            },
            last_update_unix: self.last_update_unix.load(Ordering::Relaxed) as i64,
        }
    }

    pub fn record_accepted_quote(&self, is_loss: bool, shortfall_bps: Option<u64>) {
        self.accepted_quotes.fetch_add(1, Ordering::Relaxed);
        self.last_update_unix.store(Utc::now().timestamp() as u64, Ordering::Relaxed);
        
        if is_loss {
            self.accepted_loss_quotes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.accepted_profit_quotes.fetch_add(1, Ordering::Relaxed);
            
            if let Some(shortfall) = shortfall_bps {
                self.total_shortfall_bps_sum.fetch_add(shortfall, Ordering::Relaxed);
                
                // Update worst shortfall
                loop {
                    let current_worst = self.worst_shortfall_bps.load(Ordering::Relaxed);
                    if shortfall <= current_worst {
                        break;
                    }
                    if self.worst_shortfall_bps.compare_exchange(
                        current_worst,
                        shortfall,
                        Ordering::Relaxed,
                        Ordering::Relaxed
                    ).is_ok() {
                        break;
                    }
                }
            }
        }
    }

    pub fn record_rejected_quote(&self) {
        self.rejected_quotes.fetch_add(1, Ordering::Relaxed);
        self.last_update_unix.store(Utc::now().timestamp() as u64, Ordering::Relaxed);
    }
}

static PROCEEDS_METRICS: LazyLock<ProceedsMetricsInternal> = 
    LazyLock::new(|| ProceedsMetricsInternal::new());

/// Get current proceeds metrics snapshot
pub async fn get_proceeds_metrics_snapshot() -> ProceedsMetricsSnapshot {
    PROCEEDS_METRICS.snapshot()
}

/// Record an accepted quote
pub fn record_accepted_quote(is_loss: bool, shortfall_bps: Option<u64>) {
    PROCEEDS_METRICS.record_accepted_quote(is_loss, shortfall_bps);
}

/// Record a rejected quote
pub fn record_rejected_quote() {
    PROCEEDS_METRICS.record_rejected_quote();
}