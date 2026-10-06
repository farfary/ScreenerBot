// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The pools maintenance task: one process-wide loop that runs every chain's
//! periodic cache and database upkeep.
//!
//! One tick every [`TICK`]; each step runs on a multiple of it, and every step
//! runs on the first tick:
//! - every tick (30 s): the health debug line per chain;
//! - every 2 ticks (60 s): stale price and history eviction;
//! - every 60 ticks (30 min): gapped-history cleanup, memory then database;
//! - every 720 ticks (6 h): database retention cleanup.

use super::{cache, db};
use crate::chains::{enabled_chains, ChainId};
use crate::logger::{self, LogTag};
use crate::utils::run_or_shutdown;

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

/// The maintenance cadence; every step period is a multiple of it.
const TICK: Duration = Duration::from_secs(30);
/// Ticks between stale cache evictions (60 s).
const EVICTION_EVERY_TICKS: u64 = 2;
/// Ticks between gapped-history cleanups (30 min).
const GAP_CLEANUP_EVERY_TICKS: u64 = 60;
/// Ticks between database retention cleanups (6 h).
const RETENTION_EVERY_TICKS: u64 = 720;

/// Spawn the maintenance task under the service manager's `shutdown` and
/// `monitor`, then mark the pool service ready. The returned handle is the
/// task's only handle.
pub fn start_maintenance_task(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> JoinHandle<()> {
    let handle = tokio::spawn(monitor.instrument(run_maintenance(shutdown)));
    crate::global::POOL_SERVICE_READY.store(true, Ordering::SeqCst);
    handle
}

async fn run_maintenance(shutdown: Arc<Notify>) {
    logger::info(LogTag::PoolService, "Starting pool maintenance task");

    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut tick: u64 = 0;

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = interval.tick() => {}
        }
        if run_or_shutdown(&shutdown, run_tick(tick)).await.is_none() {
            break;
        }
        tick = tick.wrapping_add(1);
    }

    logger::info(LogTag::PoolService, "Pool maintenance task shutting down");
}

/// Which periodic steps run on a given tick (the health line runs on every one).
#[derive(Debug, PartialEq, Eq)]
struct DueSteps {
    eviction: bool,
    gap_cleanup: bool,
    retention: bool,
}

fn due_steps(tick: u64) -> DueSteps {
    DueSteps {
        eviction: tick % EVICTION_EVERY_TICKS == 0,
        gap_cleanup: tick % GAP_CLEANUP_EVERY_TICKS == 0,
        retention: tick % RETENTION_EVERY_TICKS == 0,
    }
}

async fn run_tick(tick: u64) {
    let chains = enabled_chains();
    let due = due_steps(tick);

    for &chain in chains {
        emit_health_stats(chain);
    }

    if due.eviction {
        // Open positions carry no chain yet, so one set protects their mints
        // on every chain. A superset only keeps a history longer than needed;
        // exact per-chain protection needs positions to carry their chain.
        let open: HashSet<String> = crate::positions::get_open_mints()
            .await
            .into_iter()
            .collect();
        for &chain in chains {
            cache::evict_stale(chain, &open);
        }
    }

    if due.gap_cleanup {
        for &chain in chains {
            cleanup_gaps(chain).await;
        }
    }

    if due.retention {
        for &chain in chains {
            match db::cleanup_old_entries(chain).await {
                Ok(_) => logger::info(
                    LogTag::PoolService,
                    &format!("Database cleanup completed successfully on {chain}"),
                ),
                Err(e) => logger::error(
                    LogTag::PoolService,
                    &format!("Database cleanup failed on {chain}: {e}"),
                ),
            }
        }
    }
}

fn emit_health_stats(chain: ChainId) {
    let cache_stats = cache::stats(chain);

    logger::debug(
        LogTag::PoolService,
        &format!(
            "Pool service health on {}: {} total prices, {} fresh prices, {} history entries",
            chain, cache_stats.total_prices, cache_stats.fresh_prices, cache_stats.history_entries
        ),
    );
}

async fn cleanup_gaps(chain: ChainId) {
    let (total_removed, tokens_cleaned) = cache::cleanup_memory_gaps(chain);
    if total_removed > 0 {
        logger::info(
            LogTag::PoolCache,
            &format!(
                "Cleaned {} gapped entries from memory across {} tokens on {}",
                total_removed, tokens_cleaned, chain
            ),
        );
    }

    match db::cleanup_all_gapped_data(chain).await {
        Ok(deleted) if deleted > 0 => logger::info(
            LogTag::PoolService,
            &format!("Gap cleanup completed on {chain}: removed {deleted} gapped entries"),
        ),
        Ok(_) => logger::info(
            LogTag::PoolService,
            &format!("Gap cleanup completed on {chain}: no gapped data found"),
        ),
        Err(e) => logger::error(
            LogTag::PoolService,
            &format!("Gap cleanup failed on {chain}: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn due(eviction: bool, gap_cleanup: bool, retention: bool) -> DueSteps {
        DueSteps {
            eviction,
            gap_cleanup,
            retention,
        }
    }

    #[test]
    fn every_step_runs_on_the_first_tick_and_then_at_its_own_cadence() {
        assert_eq!(due_steps(0), due(true, true, true));
        assert_eq!(due_steps(1), due(false, false, false));
        assert_eq!(due_steps(2), due(true, false, false));
        assert_eq!(due_steps(60), due(true, true, false));
        assert_eq!(due_steps(720), due(true, true, true));
        assert_eq!(TICK * EVICTION_EVERY_TICKS as u32, Duration::from_secs(60));
        assert_eq!(
            TICK * GAP_CLEANUP_EVERY_TICKS as u32,
            Duration::from_secs(30 * 60)
        );
        assert_eq!(
            TICK * RETENTION_EVERY_TICKS as u32,
            Duration::from_secs(6 * 60 * 60)
        );
    }
}
