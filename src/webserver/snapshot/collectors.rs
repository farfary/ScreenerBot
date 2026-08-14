//! Snapshot collectors — gathers system state snapshots for SSE broadcast.

use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use sysinfo::System;
use tokio::task::spawn_blocking;

use super::types::*;
use super::{
    CachedSystemMetrics, MAX_PENDING_QUEUE_SAMPLE, MAX_WALLET_TOKENS, SYSTEM_METRICS_CACHE,
    SYSTEM_METRICS_CACHE_SECS,
};
use crate::{
    config,
    global::{
        are_core_services_ready, get_pending_services, POOL_SERVICE_READY, POSITIONS_SYSTEM_READY,
        TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
    },
    logger::{self, LogTag},
    wallet::{get_current_wallet_status, get_snapshot_token_balances},
};

pub fn collect_service_status_snapshot() -> ServiceStatusSnapshot {
    let now = Utc::now();
    let all_ready = are_core_services_ready();
    let pending = get_pending_services();
    let pending_message = if pending.is_empty() {
        None
    } else {
        Some(format!("Waiting for: {}", pending.join(", ")))
    };

    let tokens_ready = TOKENS_SYSTEM_READY.load(Ordering::SeqCst);
    let positions_ready = POSITIONS_SYSTEM_READY.load(Ordering::SeqCst);
    let pool_ready = POOL_SERVICE_READY.load(Ordering::SeqCst);
    let transactions_ready = TRANSACTIONS_SYSTEM_READY.load(Ordering::SeqCst);

    ServiceStatusSnapshot {
        tokens_system: ServiceStateSnapshot::new(tokens_ready, now, pending_message.clone()),
        positions_system: ServiceStateSnapshot::new(positions_ready, now, pending_message.clone()),
        pool_service: ServiceStateSnapshot::new(pool_ready, now, pending_message.clone()),
        transactions_system: ServiceStateSnapshot::new(transactions_ready, now, pending_message),
        all_ready,
    }
}

/// Get cached system metrics for dashboard endpoints
/// Uses the same 5-second cache as collect_system_metrics_snapshot
pub async fn get_cached_system_metrics() -> SystemMetricsSnapshot {
    collect_system_metrics_snapshot(None).await
}

pub(super) async fn collect_system_metrics_snapshot(
    rpc_metrics: Option<RpcMetricsSummary>,
) -> SystemMetricsSnapshot {
    // Check cache first
    {
        let cache = SYSTEM_METRICS_CACHE.read().unwrap();
        if let Some(ref cached) = *cache {
            if cached.last_updated.elapsed() < Duration::from_secs(SYSTEM_METRICS_CACHE_SECS) {
                // Return cached metrics with updated RPC stats
                let mut metrics = cached.metrics.clone();
                if let Some(rpc) = rpc_metrics {
                    metrics.rpc_calls_total = rpc.total_calls;
                    metrics.rpc_calls_failed = rpc.total_errors;
                    metrics.rpc_success_rate = rpc.success_rate;
                    metrics.rpc_calls_per_minute_recent = rpc.recent_calls_per_minute;
                }
                return metrics;
            }
        }
    }

    // Cache miss or stale - compute fresh metrics
    let fresh_metrics = spawn_blocking(move || {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_system_percent = sys.global_cpu_info().cpu_usage();
        // sysinfo returns memory in bytes, convert to MB (bytes / 1024 / 1024)
        let system_memory_total_mb = (sys.total_memory() / 1024 / 1024) as u64;
        let system_memory_used_mb = (sys.used_memory() / 1024 / 1024) as u64;

        let (process_memory_mb, cpu_process_percent) = match sysinfo::get_current_pid() {
            Ok(pid) => match sys.process(pid) {
                // process.memory() returns bytes, convert to MB
                Some(process) => ((process.memory() / 1024 / 1024) as u64, process.cpu_usage()),
                None => (0, 0.0),
            },
            Err(_) => (0, 0.0),
        };

        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        SystemMetricsSnapshot {
            memory_usage_mb: system_memory_used_mb,
            cpu_usage_percent: cpu_system_percent,
            system_memory_used_mb,
            system_memory_total_mb,
            process_memory_mb,
            cpu_system_percent,
            cpu_process_percent,
            active_threads: thread_count,
            rpc_calls_total: 0,
            rpc_calls_failed: 0,
            rpc_success_rate: 100.0,
            rpc_calls_per_minute_recent: 0.0,
        }
    })
    .await
    .unwrap_or_default();

    // Update cache
    {
        let mut cache = SYSTEM_METRICS_CACHE.write().unwrap();
        *cache = Some(CachedSystemMetrics {
            metrics: fresh_metrics.clone(),
            last_updated: std::time::Instant::now(),
        });
    }

    // Apply RPC metrics
    let mut result = fresh_metrics;
    if let Some(rpc) = rpc_metrics {
        result.rpc_calls_total = rpc.total_calls;
        result.rpc_calls_failed = rpc.total_errors;
        result.rpc_success_rate = rpc.success_rate;
        result.rpc_calls_per_minute_recent = rpc.recent_calls_per_minute;
    }

    result
}

pub(super) async fn collect_wallet_snapshot() -> Option<WalletStatusSnapshot> {
    if crate::global::is_explore_mode() {
        return None;
    }

    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            let mut token_balances = Vec::new();

            if let Some(id) = snapshot.id {
                match get_snapshot_token_balances(id).await {
                    Ok(tokens) => {
                        token_balances = tokens
                            .into_iter()
                            .take(MAX_WALLET_TOKENS)
                            .map(|token| WalletTokenBalanceSnapshot {
                                mint: token.mint,
                                balance: token.balance,
                                balance_ui: token.balance_ui,
                                decimals: token.decimals,
                                is_token_2022: token.is_token_2022,
                            })
                            .collect();
                    }
                    Err(err) => {
                        logger::warning(
                            LogTag::Webserver,
                            &format!("Failed to load wallet token balances: {err}"),
                        );
                    }
                }
            }

            Some(WalletStatusSnapshot {
                sol_balance: snapshot.sol_balance,
                sol_balance_lamports: snapshot.sol_balance_lamports,
                usdc_balance: 0.0,
                total_tokens_count: snapshot.total_tokens_count,
                snapshot_time: Some(snapshot.snapshot_time),
                token_balances,
            })
        }
        Ok(None) => None,
        Err(err) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to load current wallet snapshot: {err}"),
            );
            None
        }
    }
}

pub(super) async fn collect_ohlcv_stats_snapshot() -> Option<OhlcvStatsSnapshot> {
    crate::ohlcvs::get_monitor_stats().await.map(|stats| {
        let telemetry = stats.telemetry.clone();
        let telemetry_snapshot = OhlcvTelemetrySnapshot {
            monitor_cycle_started_at: telemetry.monitor_cycle_started_at,
            monitor_cycle_completed_at: telemetry.monitor_cycle_completed_at,
            monitor_cycle_duration_ms: telemetry.monitor_cycle_duration_ms,
            monitor_cycle_tokens_processed: telemetry.monitor_cycle_tokens_processed,
            monitor_cycle_total: telemetry.monitor_cycle_total,
            gap_cycle_started_at: telemetry.gap_cycle_started_at,
            gap_cycle_completed_at: telemetry.gap_cycle_completed_at,
            gap_cycle_duration_ms: telemetry.gap_cycle_duration_ms,
            gap_cycle_tokens_processed: telemetry.gap_cycle_tokens_processed,
            gap_cycle_total: telemetry.gap_cycle_total,
            last_rate_limit_at: telemetry.last_rate_limit_at,
            rate_limit_events: telemetry.rate_limit_events,
            total_backfills_scheduled: telemetry.total_backfills_scheduled,
            total_backfills_completed: telemetry.total_backfills_completed,
            total_backfills_failed: telemetry.total_backfills_failed,
            last_backfill_started_at: telemetry.last_backfill_started_at,
            last_backfill_completed_at: telemetry.last_backfill_completed_at,
            last_backfill_duration_ms: telemetry.last_backfill_duration_ms,
            last_backfill_points: telemetry.last_backfill_points,
            last_backfill_error: telemetry.last_backfill_error.clone(),
        };

        let top_open_gaps = stats
            .top_open_gaps
            .iter()
            .map(|gap| OhlcvGapSummarySnapshot {
                mint: gap.mint.clone(),
                open_gaps: gap.open_gaps,
                largest_gap_seconds: gap.largest_gap_seconds,
                latest_gap_end: gap.latest_gap_end,
            })
            .collect::<Vec<_>>();

        OhlcvStatsSnapshot {
            total_tokens: stats.total_tokens,
            critical_tokens: stats.critical_tokens,
            high_tokens: stats.high_tokens,
            medium_tokens: stats.medium_tokens,
            low_tokens: stats.low_tokens,
            cache_hit_rate: (stats.cache_hit_rate * 100.0).clamp(0.0, 100.0),
            api_calls_per_minute: stats.api_calls_per_minute,
            queue_size: stats.queue_size,
            telemetry: telemetry_snapshot,
            backfills_in_progress: stats.backfills_in_progress,
            open_gap_tokens: stats.open_gap_tokens,
            open_gap_total: stats.open_gap_total,
            top_open_gaps,
        }
    })
}

pub(super) fn collect_pool_service_snapshot() -> Option<PoolServiceStatusSnapshot> {
    let running = crate::pools::is_pool_service_running();
    let system_ready = POOL_SERVICE_READY.load(Ordering::SeqCst);

    let cache_stats = crate::pools::get_cache_stats();
    let monitored_tokens_count = crate::pools::get_available_tokens().len();
    let price_subscribers = 0;

    let analyzer_snapshot = crate::pools::get_pool_analyzer().and_then(|analyzer| {
        let directory = analyzer.get_pool_directory();
        let guard = directory.read().ok()?;

        let total_pools = guard.len();
        let mut program_counts: HashMap<String, usize> = HashMap::new();
        for descriptor in guard.values() {
            let label = descriptor.program_kind.display_name().to_string();
            *program_counts.entry(label).or_default() += 1;
        }

        let mut program_distribution: Vec<PoolProgramCount> = program_counts
            .into_iter()
            .map(|(program, count)| PoolProgramCount { program, count })
            .collect();
        program_distribution.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.program.cmp(&b.program))
        });

        Some(PoolAnalyzerSnapshot {
            total_pools,
            program_distribution,
        })
    });

    let fetcher_snapshot = crate::pools::get_account_fetcher().map(|fetcher| {
        let stats = fetcher.get_fetch_stats();
        PoolFetcherSnapshot {
            total_bundles: stats.total_bundles,
            bundles_with_data: stats.bundles_with_data,
            total_accounts_tracked: stats.total_accounts_tracked,
        }
    });

    let debug_override_tokens = crate::pools::get_debug_token_override();
    let debug_override_count = debug_override_tokens
        .as_ref()
        .map(|tokens| tokens.len())
        .unwrap_or_default();

    let (dexs_enabled, gecko_enabled, raydium_enabled) =
        crate::pools::PoolDiscovery::get_source_config();
    let mut sources_enabled = Vec::new();
    if dexs_enabled {
        sources_enabled.push("DexScreener".to_owned());
    }
    if gecko_enabled {
        sources_enabled.push("GeckoTerminal".to_owned());
    }
    if raydium_enabled {
        sources_enabled.push("Raydium".to_owned());
    }

    let discovery_snapshot = PoolDiscoverySnapshot {
        sources_enabled,
        debug_override_active: debug_override_count > 0,
        debug_override_count: if debug_override_count > 0 {
            Some(debug_override_count)
        } else {
            None
        },
    };

    Some(PoolServiceStatusSnapshot {
        running,
        system_ready,
        single_pool_mode: crate::pools::is_single_pool_mode_enabled(),
        monitored_tokens: monitored_tokens_count,
        monitored_capacity: crate::pools::types::max_watched_tokens(),
        price_subscribers,
        cache: PoolCacheSnapshot {
            total_prices: cache_stats.total_prices,
            fresh_prices: cache_stats.fresh_prices,
            history_entries: cache_stats.history_entries,
        },
        analyzer: analyzer_snapshot,
        fetcher: fetcher_snapshot,
        discovery: Some(discovery_snapshot),
    })
}

pub(super) async fn collect_token_discovery_snapshot() -> Option<TokenDiscoveryStatusSnapshot> {
    // Check if discovery service is registered and enabled
    // Note: We always return a snapshot if the service exists, even if no stats yet
    // This allows the UI to show "Waiting for first cycle..." messages
    let running = if let Some(manager_lock) = crate::services::get_service_manager().await {
        if let Some(manager) = manager_lock.read().await.as_ref() {
            manager.is_service_enabled("token_discovery")
        } else {
            false
        }
    } else {
        false
    };

    // If service is not registered, return None to hide the tab
    if !running {
        return None;
    }

    // Get stats (may be empty if first cycle hasn't completed yet)
    // Token discovery service stats not available; hide section
    return None;

    Some(TokenDiscoveryStatusSnapshot {
        running,
        total_cycles: 0,
        last_cycle_started: None,
        last_cycle_completed: None,
        last_processed: 0,
        last_added: 0,
        last_deduplicated_removed: 0,
        last_blacklist_removed: 0,
        total_processed: 0,
        total_added: 0,
        sources: DiscoverySourceSnapshot {
            profiles: 0,
            boosted: 0,
            top_boosts: 0,
            rug_new: 0,
            rug_viewed: 0,
            rug_trending: 0,
            rug_verified: 0,
            gecko_updated: 0,
            gecko_trending: 0,
            jupiter_tokens: 0,
            jupiter_top_organic: 0,
            jupiter_top_traded: 0,
            jupiter_top_trending: 0,
            coingecko_markets: 0,
            defillama_protocols: 0,
        },
        last_error: None,
    })
}

pub(super) async fn collect_dexscreener_status_snapshot() -> Option<DexscreenerStatusSnapshot> {
    // Placeholder snapshot shows disabled/zeroed metrics
    return Some(DexscreenerStatusSnapshot {
        enabled: config::with_config(|cfg| cfg.tokens.sources.dexscreener.enabled),
        initialized: false,
        rate_limit_per_minute: 0,
        discovery_rate_limit_per_minute: 0,
        max_tokens_per_call: 0,
        token_cache_entries: 0,
        token_cache_fresh: 0,
        pool_cache_entries: 0,
        pool_cache_fresh: 0,
        price_cache_ttl_secs: 0,
        pool_cache_ttl_secs: 0,
        api_total_requests: 0,
        api_successful_requests: 0,
        api_failed_requests: 0,
        api_success_rate: 0.0,
        api_cache_hits: 0,
        api_cache_misses: 0,
        api_cache_hit_rate: 0.0,
        api_average_response_ms: 0.0,
        last_request_time: None,
    });
}

pub(super) async fn collect_gecko_terminal_status_snapshot() -> Option<GeckoTerminalStatusSnapshot>
{
    // TODO: Rewire to new API stats trackers under tokens::api
    let enabled = config::with_config(|cfg| cfg.tokens.sources.geckoterminal.enabled);
    Some(GeckoTerminalStatusSnapshot {
        enabled,
        initialized: false,
        rate_limit_per_minute: 0,
        max_tokens_per_batch: 0,
        cache_entries: 0,
        cache_fresh: 0,
        cache_ttl_secs: 0,
        api_total_requests: 0,
        api_successful_requests: 0,
        api_failed_requests: 0,
        api_success_rate: 0.0,
        api_cache_hits: 0,
        api_cache_misses: 0,
        api_cache_hit_rate: 0.0,
        api_average_response_ms: 0.0,
        last_request_time: None,
        last_success_time: None,
        last_error_time: None,
        last_error_message: None,
        current_rate_limit_calls: 0,
        current_rate_limit_max: 0,
        rate_limit_resets_in_ms: Some(0),
    })
}

pub(super) async fn collect_events_snapshot() -> Option<EventsStatusSnapshot> {
    // Check if events system is initialized
    let db = match crate::events::EVENTS_DB.get() {
        Some(db) => db,
        None => return None,
    };

    // Get database stats
    let db_stats = match db.get_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to load events database stats: {e}"),
            );
            HashMap::new()
        }
    };

    let total_events = db_stats.get("total_events").copied().unwrap_or_default();
    let events_24h = db_stats.get("events_24h").copied().unwrap_or_default();
    let db_size_bytes = db_stats.get("db_size_bytes").copied().unwrap_or_default();

    // Get category counts for last 24 hours
    let category_counts = match crate::events::count_by_category(24).await {
        Ok(counts) => counts,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to load events category counts: {e}"),
            );
            HashMap::new()
        }
    };

    // Get recent events (last 10)
    let recent_events_raw = match crate::events::recent_all(10).await {
        Ok(events) => events,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to load recent events: {e}"),
            );
            Vec::new()
        }
    };

    let recent_events = recent_events_raw
        .into_iter()
        .map(|event| EventSnapshot {
            id: event.id.unwrap_or_default(),
            event_time: event.event_time.to_rfc3339(),
            category: event.category.to_string(),
            subtype: event.subtype,
            severity: event.severity.to_string(),
            mint: event.mint,
            reference_id: event.reference_id,
        })
        .collect();

    Some(EventsStatusSnapshot {
        running: true,
        total_events,
        events_24h,
        db_size_bytes,
        category_counts,
        recent_events,
    })
}

pub(super) async fn collect_transactions_snapshot() -> Option<TransactionsStatusSnapshot> {
    let running = crate::transactions::is_global_transaction_service_running().await;
    let system_ready = TRANSACTIONS_SYSTEM_READY.load(Ordering::SeqCst);
    let global_pending = crate::transactions::utils::get_pending_transactions_count().await as u64;

    let mut stats = crate::transactions::TransactionStats::default();
    let mut wallet_pubkey: Option<String> = None;
    let mut last_signature_check: Option<String> = None;
    let mut pending_entries: Vec<(String, chrono::DateTime<Utc>)> = Vec::new();
    let mut deferred_retries: u64 = 0;
    let mut db_arc = None;

    if let Some(manager_arc) = crate::transactions::get_global_transaction_manager().await {
        let manager = manager_arc.lock().await;
        stats = manager.get_stats();
        wallet_pubkey = Some(manager.wallet_pubkey.to_string());
        last_signature_check = manager.last_signature_check.clone();
        deferred_retries = manager.get_deferred_retries_count() as u64;
        pending_entries = manager
            .pending_transactions
            .iter()
            .map(|(sig, ts)| (sig.clone(), *ts))
            .collect();
        db_arc = manager.get_transaction_database();
    } else {
        db_arc = crate::transactions::get_transaction_database().await;
    }

    let now = Utc::now();
    let mut queue_snapshot = TransactionQueueSnapshot {
        pending_local: pending_entries.len() as u64,
        pending_global: global_pending,
        deferred_retries,
        sample: Vec::new(),
        oldest_age_seconds: None,
    };

    if !pending_entries.is_empty() {
        pending_entries.sort_by_key(|(_, ts)| *ts);
        queue_snapshot.sample = pending_entries
            .iter()
            .take(MAX_PENDING_QUEUE_SAMPLE)
            .map(|(sig, ts)| TransactionPendingSnapshot {
                signature: sig.clone(),
                age_seconds: (now - *ts).num_seconds().max(0),
            })
            .collect();
        if let Some((_, ts)) = pending_entries.first() {
            queue_snapshot.oldest_age_seconds = Some((now - *ts).num_seconds().max(0));
        }
    }

    if db_arc.is_none() {
        db_arc = crate::transactions::get_transaction_database().await;
    }

    let mut database_snapshot = None;
    let mut bootstrap_snapshot = None;
    let mut newest_signature: Option<String> = None;
    let mut oldest_signature: Option<String> = None;

    if let Some(db) = db_arc {
        match db.get_stats().await {
            Ok(db_stats) => {
                stats.total_transactions = db_stats.total_raw_transactions;
                stats.known_signatures_count = db_stats.total_known_signatures;
                stats.pending_transactions_count = db_stats.total_pending_transactions;
                // Treat raw minus processed rows as "new" transactions still awaiting analysis.
                stats.new_transactions_count = db_stats
                    .total_raw_transactions
                    .saturating_sub(db_stats.total_processed_transactions);

                database_snapshot = Some(TransactionDatabaseSnapshot {
                    raw_transactions: db_stats.total_raw_transactions,
                    processed_transactions: db_stats.total_processed_transactions,
                    known_signatures: db_stats.total_known_signatures,
                    pending_records: db_stats.total_pending_transactions,
                    deferred_retry_records: db_stats.total_deferred_retries,
                    size_bytes: db_stats.database_size_bytes,
                    schema_version: db_stats.schema_version,
                    last_updated: db_stats.last_updated.to_rfc3339(),
                });
            }
            Err(err) => {
                logger::warning(
                    LogTag::Webserver,
                    &format!("Failed to load transactions database stats: {err}"),
                );
            }
        }

        // Known-signature bookkeeping is scoped to a subject. This snapshot describes
        // OUR history, so it asks for our own wallet -- never for any other wallet the
        // app may be watching.
        let own_subject = crate::transactions::Subject::own().ok();
        let (
            successful_count_result,
            failed_count_result,
            bootstrap_state_result,
            newest_sig_result,
            oldest_sig_result,
        ) = tokio::join!(
            db.get_successful_transactions_count(),
            db.get_failed_transactions_count(),
            db.get_bootstrap_state(),
            async {
                match own_subject {
                    Some(subject) => db.get_newest_known_signature(subject).await,
                    None => Ok(None),
                }
            },
            async {
                match own_subject {
                    Some(subject) => db.get_oldest_known_signature(subject).await,
                    None => Ok(None),
                }
            }
        );

        match successful_count_result {
            Ok(count) => stats.successful_transactions_count = count,
            Err(err) => logger::warning(
                LogTag::Webserver,
                &format!("Failed to load successful transaction count: {err}"),
            ),
        }

        match failed_count_result {
            Ok(count) => stats.failed_transactions_count = count,
            Err(err) => logger::warning(
                LogTag::Webserver,
                &format!("Failed to load failed transaction count: {err}"),
            ),
        }

        match bootstrap_state_result {
            Ok(state) => {
                bootstrap_snapshot = Some(TransactionBootstrapSnapshot {
                    full_history_completed: state.full_history_completed,
                    backfill_cursor: state.backfill_before_cursor,
                });
            }
            Err(err) => logger::warning(
                LogTag::Webserver,
                &format!("Failed to load transactions bootstrap state: {err}"),
            ),
        }

        match newest_sig_result {
            Ok(sig) => newest_signature = sig,
            Err(err) => logger::warning(
                LogTag::Webserver,
                &format!("Failed to load newest known signature: {err}"),
            ),
        }

        match oldest_sig_result {
            Ok(sig) => oldest_signature = sig,
            Err(err) => logger::warning(
                LogTag::Webserver,
                &format!("Failed to load oldest known signature: {err}"),
            ),
        }
    }

    let success_rate = stats.success_rate();
    let failure_rate = stats.failure_rate();

    Some(TransactionsStatusSnapshot {
        running,
        system_ready,
        wallet_pubkey,
        last_signature_check,
        newest_known_signature: newest_signature,
        oldest_known_signature: oldest_signature,
        stats,
        success_rate,
        failure_rate,
        queue: queue_snapshot,
        database: database_snapshot,
        bootstrap: bootstrap_snapshot,
    })
}
