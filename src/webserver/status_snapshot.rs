use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use sysinfo::System;

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    global::{
        are_core_services_ready, get_pending_services, POOL_SERVICE_READY, POSITIONS_SYSTEM_READY,
        SECURITY_ANALYZER_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
    },
    logger::{log, LogTag},
    trader::is_trader_running,
    wallet::{get_current_wallet_status, get_snapshot_token_balances},
    webserver::{
        state::{get_app_state, AppState},
        utils::format_duration,
    },
};

use crate::rpc::get_global_rpc_stats;

const MAX_WALLET_TOKENS: usize = 128;
const MAX_PENDING_QUEUE_SAMPLE: usize = 10;

#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub trading_enabled: bool,
    pub trader_mode: String,
    pub trader_running: bool,
    pub open_positions: usize,
    pub closed_positions_today: usize,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub services: ServiceStatusSnapshot,
    pub metrics: SystemMetricsSnapshot,
    pub rpc_stats: Option<RpcStatsSnapshot>,
    pub wallet: Option<WalletStatusSnapshot>,
    pub ohlcv_stats: Option<OhlcvStatsSnapshot>,
    pub pools: Option<PoolServiceStatusSnapshot>,
    pub discovery: Option<TokenDiscoveryStatusSnapshot>,
    pub events: Option<EventsStatusSnapshot>,
    pub transactions: Option<TransactionsStatusSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatusSnapshot {
    pub tokens_system: ServiceStateSnapshot,
    pub positions_system: ServiceStateSnapshot,
    pub pool_service: ServiceStateSnapshot,
    pub security_analyzer: ServiceStateSnapshot,
    pub transactions_system: ServiceStateSnapshot,
    pub all_ready: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStateSnapshot {
    pub ready: bool,
    pub status: String,
    pub last_check: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemMetricsSnapshot {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub system_memory_used_mb: u64,
    pub system_memory_total_mb: u64,
    pub process_memory_mb: u64,
    pub cpu_system_percent: f32,
    pub cpu_process_percent: f32,
    pub active_threads: usize,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub rpc_success_rate: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletStatusSnapshot {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub usdc_balance: f64,
    pub total_tokens_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_time: Option<DateTime<Utc>>,
    pub token_balances: Vec<WalletTokenBalanceSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletTokenBalanceSnapshot {
    pub mint: String,
    pub balance: u64,
    pub balance_ui: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct OhlcvStatsSnapshot {
    pub total_tokens: usize,
    pub critical_tokens: usize,
    pub high_tokens: usize,
    pub medium_tokens: usize,
    pub low_tokens: usize,
    pub cache_hit_rate: f64,
    pub api_calls_per_minute: f64,
    pub queue_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolServiceStatusSnapshot {
    pub running: bool,
    pub system_ready: bool,
    pub single_pool_mode: bool,
    pub monitored_tokens: usize,
    pub monitored_capacity: usize,
    pub price_subscribers: usize,
    pub cache: PoolCacheSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzer: Option<PoolAnalyzerSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetcher: Option<PoolFetcherSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery: Option<PoolDiscoverySnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolCacheSnapshot {
    pub total_prices: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolAnalyzerSnapshot {
    pub total_pools: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub program_distribution: Vec<PoolProgramCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolProgramCount {
    pub program: String,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolFetcherSnapshot {
    pub total_bundles: usize,
    pub bundles_with_data: usize,
    pub total_accounts_tracked: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolDiscoverySnapshot {
    pub sources_enabled: Vec<String>,
    pub debug_override_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_override_count: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenDiscoveryStatusSnapshot {
    pub running: bool,
    pub total_cycles: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cycle_started: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cycle_completed: Option<String>,
    pub last_processed: usize,
    pub last_added: usize,
    pub last_deduplicated_removed: usize,
    pub last_blacklist_removed: usize,
    pub total_processed: u64,
    pub total_added: u64,
    pub sources: DiscoverySourceSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiscoverySourceSnapshot {
    pub profiles: usize,
    pub boosted: usize,
    pub top_boosts: usize,
    pub rug_new: usize,
    pub rug_viewed: usize,
    pub rug_trending: usize,
    pub rug_verified: usize,
    pub gecko_updated: usize,
    pub gecko_trending: usize,
    pub jupiter_tokens: usize,
    pub jupiter_top_organic: usize,
    pub jupiter_top_traded: usize,
    pub jupiter_top_trending: usize,
    pub coingecko_markets: usize,
    pub defillama_protocols: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventsStatusSnapshot {
    pub running: bool,
    pub total_events: i64,
    pub events_24h: i64,
    pub db_size_bytes: i64,
    pub category_counts: HashMap<String, u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_events: Vec<EventSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventSnapshot {
    pub id: i64,
    pub event_time: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionsStatusSnapshot {
    pub running: bool,
    pub system_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_signature_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_known_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_known_signature: Option<String>,
    pub stats: crate::transactions::TransactionStats,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub queue: TransactionQueueSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<TransactionDatabaseSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<TransactionBootstrapSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionQueueSnapshot {
    pub pending_local: u64,
    pub pending_global: u64,
    pub deferred_retries: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<TransactionPendingSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_age_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionPendingSnapshot {
    pub signature: String,
    pub age_seconds: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionDatabaseSnapshot {
    pub raw_transactions: u64,
    pub processed_transactions: u64,
    pub known_signatures: u64,
    pub pending_records: u64,
    pub deferred_retry_records: u64,
    pub size_bytes: u64,
    pub schema_version: u32,
    pub last_updated: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionBootstrapSnapshot {
    pub full_history_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backfill_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct RpcStatsSnapshot {
    pub total_calls: u64,
    pub total_errors: u64,
    pub success_rate: f32,
    pub calls_per_second: f64,
    pub average_response_time_ms: f64,
    pub calls_per_url: HashMap<String, u64>,
    pub errors_per_url: HashMap<String, u64>,
    pub calls_per_method: HashMap<String, u64>,
    pub errors_per_method: HashMap<String, u64>,
    pub uptime_seconds: i64,
}

/// Gather current status snapshot (aggregates data from multiple sources)
pub async fn gather_status_snapshot() -> StatusSnapshot {
    let trading_enabled = config::with_config(|cfg| cfg.trader.enabled);
    let trader_mode = "Normal".to_string();
    let trader_running = is_trader_running();

    let open_positions = crate::positions::db::get_open_positions()
        .await
        .map(|positions| positions.len())
        .unwrap_or(0);

    let day_start_naive = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| Utc::now().naive_utc());
    let day_start = DateTime::<Utc>::from_naive_utc_and_offset(day_start_naive, Utc);

    let closed_positions_today = crate::positions::get_db_closed_positions_count_since(day_start)
        .await
        .map(|count| std::cmp::max(count, 0) as usize)
        .unwrap_or(0);

    let app_state = get_app_state().await;
    let uptime_seconds = app_state
        .as_ref()
        .map(|state| state.uptime_seconds())
        .unwrap_or(0);
    let uptime_formatted = format_duration(uptime_seconds);

    let rpc_stats_raw = get_global_rpc_stats();

    let services = collect_service_status_snapshot();
    let metrics = collect_system_metrics_snapshot(rpc_stats_raw.as_ref()).await;

    let rpc_stats = rpc_stats_raw.as_ref().map(|stats| RpcStatsSnapshot {
        total_calls: stats.total_calls(),
        total_errors: stats.total_errors(),
        success_rate: stats.success_rate(),
        calls_per_second: stats.calls_per_second(),
        average_response_time_ms: stats.average_response_time_ms_global(),
        calls_per_url: stats.calls_per_url.clone(),
        errors_per_url: stats.errors_per_url.clone(),
        calls_per_method: stats.calls_per_method.clone(),
        errors_per_method: stats.errors_per_method.clone(),
        uptime_seconds: Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds(),
    });

    let wallet = collect_wallet_snapshot().await;
    let sol_balance = wallet.as_ref().map(|w| w.sol_balance).unwrap_or(0.0);
    let usdc_balance = wallet.as_ref().map(|w| w.usdc_balance).unwrap_or(0.0);

    let ohlcv_stats = crate::ohlcvs::get_monitor_stats()
        .await
        .map(|stats| OhlcvStatsSnapshot {
            total_tokens: stats.total_tokens,
            critical_tokens: stats.critical_tokens,
            high_tokens: stats.high_tokens,
            medium_tokens: stats.medium_tokens,
            low_tokens: stats.low_tokens,
            cache_hit_rate: (stats.cache_hit_rate * 100.0).clamp(0.0, 100.0),
            api_calls_per_minute: stats.api_calls_per_minute,
            queue_size: stats.queue_size,
        });

    let pools = collect_pool_service_snapshot();
    let discovery = collect_token_discovery_snapshot().await;
    let events = collect_events_snapshot().await;
    let transactions = collect_transactions_snapshot().await;

    StatusSnapshot {
        timestamp: Utc::now(),
        uptime_seconds,
        uptime_formatted,
        trading_enabled,
        trader_mode,
        trader_running,
        open_positions,
        closed_positions_today,
        sol_balance,
        usdc_balance,
        services,
        metrics,
        rpc_stats,
        wallet,
        ohlcv_stats,
        pools,
        discovery,
        events,
        transactions,
    }
}

fn collect_service_status_snapshot() -> ServiceStatusSnapshot {
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
    let security_ready = SECURITY_ANALYZER_READY.load(Ordering::SeqCst);
    let transactions_ready = TRANSACTIONS_SYSTEM_READY.load(Ordering::SeqCst);

    ServiceStatusSnapshot {
        tokens_system: ServiceStateSnapshot::new(tokens_ready, now, pending_message.clone()),
        positions_system: ServiceStateSnapshot::new(positions_ready, now, pending_message.clone()),
        pool_service: ServiceStateSnapshot::new(pool_ready, now, pending_message.clone()),
        security_analyzer: ServiceStateSnapshot::new(security_ready, now, pending_message.clone()),
        transactions_system: ServiceStateSnapshot::new(transactions_ready, now, pending_message),
        all_ready,
    }
}

impl ServiceStateSnapshot {
    fn new(ready: bool, last_check: DateTime<Utc>, error: Option<String>) -> Self {
        let status = if ready { "healthy" } else { "starting" }.to_string();
        let error = if ready { None } else { error };
        Self {
            ready,
            status,
            last_check,
            error,
        }
    }
}

async fn collect_system_metrics_snapshot(
    rpc_stats: Option<&crate::rpc::RpcStats>,
) -> SystemMetricsSnapshot {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_system_percent = sys.global_cpu_info().cpu_usage();
    let system_memory_total_mb = (sys.total_memory() / 1024 / 1024) as u64;
    let system_memory_used_mb = (sys.used_memory() / 1024 / 1024) as u64;

    let (process_memory_mb, cpu_process_percent) = match sysinfo::get_current_pid() {
        Ok(pid) => match sys.process(pid) {
            Some(process) => ((process.memory() / 1024 / 1024) as u64, process.cpu_usage()),
            None => (0, 0.0),
        },
        Err(_) => (0, 0.0),
    };

    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let (rpc_calls_total, rpc_calls_failed, rpc_success_rate) = if let Some(stats) = rpc_stats {
        (
            stats.total_calls(),
            stats.total_errors(),
            stats.success_rate(),
        )
    } else {
        (0, 0, 100.0)
    };

    SystemMetricsSnapshot {
        memory_usage_mb: system_memory_used_mb,
        cpu_usage_percent: cpu_system_percent,
        system_memory_used_mb,
        system_memory_total_mb,
        process_memory_mb,
        cpu_system_percent,
        cpu_process_percent,
        active_threads: thread_count,
        rpc_calls_total,
        rpc_calls_failed,
        rpc_success_rate,
    }
}

async fn collect_wallet_snapshot() -> Option<WalletStatusSnapshot> {
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
                        log(
                            LogTag::Webserver,
                            "WARN",
                            &format!("Failed to load wallet token balances: {}", err),
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
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load current wallet snapshot: {}", err),
            );
            None
        }
    }
}

fn collect_pool_service_snapshot() -> Option<PoolServiceStatusSnapshot> {
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
            *program_counts.entry(label).or_insert(0) += 1;
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
        .unwrap_or(0);

    let (dexs_enabled, gecko_enabled, raydium_enabled) =
        crate::pools::PoolDiscovery::get_source_config();
    let mut sources_enabled = Vec::new();
    if dexs_enabled {
        sources_enabled.push("DexScreener".to_string());
    }
    if gecko_enabled {
        sources_enabled.push("GeckoTerminal".to_string());
    }
    if raydium_enabled {
        sources_enabled.push("Raydium".to_string());
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
        monitored_capacity: crate::pools::types::MAX_WATCHED_TOKENS,
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

async fn collect_token_discovery_snapshot() -> Option<TokenDiscoveryStatusSnapshot> {
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
    let stats = crate::tokens::discovery::get_discovery_stats().await;

    Some(TokenDiscoveryStatusSnapshot {
        running,
        total_cycles: stats.total_cycles,
        last_cycle_started: stats.last_cycle_started.map(|dt| dt.to_rfc3339()),
        last_cycle_completed: stats.last_cycle_completed.map(|dt| dt.to_rfc3339()),
        last_processed: stats.last_processed,
        last_added: stats.last_added,
        last_deduplicated_removed: stats.last_deduplicated_removed,
        last_blacklist_removed: stats.last_blacklist_removed,
        total_processed: stats.total_processed,
        total_added: stats.total_added,
        sources: DiscoverySourceSnapshot {
            profiles: stats.per_source.profiles,
            boosted: stats.per_source.boosted,
            top_boosts: stats.per_source.top_boosts,
            rug_new: stats.per_source.rug_new,
            rug_viewed: stats.per_source.rug_viewed,
            rug_trending: stats.per_source.rug_trending,
            rug_verified: stats.per_source.rug_verified,
            gecko_updated: stats.per_source.gecko_updated,
            gecko_trending: stats.per_source.gecko_trending,
            jupiter_tokens: stats.per_source.jupiter_tokens,
            jupiter_top_organic: stats.per_source.jupiter_top_organic,
            jupiter_top_traded: stats.per_source.jupiter_top_traded,
            jupiter_top_trending: stats.per_source.jupiter_top_trending,
            coingecko_markets: stats.per_source.coingecko_markets,
            defillama_protocols: stats.per_source.defillama_protocols,
        },
        last_error: stats.last_error,
    })
}

async fn collect_events_snapshot() -> Option<EventsStatusSnapshot> {
    // Check if events system is initialized
    let running = crate::events::EVENTS_DB.get().is_some();

    if !running {
        return None;
    }

    // Get database stats
    let db_stats = match crate::events::EVENTS_DB.get() {
        Some(db) => match db.get_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Failed to load events database stats: {}", e),
                );
                HashMap::new()
            }
        },
        None => HashMap::new(),
    };

    let total_events = db_stats.get("total_events").copied().unwrap_or(0);
    let events_24h = db_stats.get("events_24h").copied().unwrap_or(0);
    let db_size_bytes = db_stats.get("db_size_bytes").copied().unwrap_or(0);

    // Get category counts for last 24 hours
    let category_counts = match crate::events::count_by_category(24).await {
        Ok(counts) => counts,
        Err(e) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load events category counts: {}", e),
            );
            HashMap::new()
        }
    };

    // Get recent events (last 10)
    let recent_events_raw = match crate::events::recent_all(10).await {
        Ok(events) => events,
        Err(e) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load recent events: {}", e),
            );
            Vec::new()
        }
    };

    let recent_events = recent_events_raw
        .into_iter()
        .map(|event| EventSnapshot {
            id: event.id.unwrap_or(0),
            event_time: event.event_time.to_rfc3339(),
            category: event.category.to_string(),
            subtype: event.subtype,
            severity: event.severity.to_string(),
            mint: event.mint,
            reference_id: event.reference_id,
        })
        .collect();

    Some(EventsStatusSnapshot {
        running,
        total_events,
        events_24h,
        db_size_bytes,
        category_counts,
        recent_events,
    })
}

async fn collect_transactions_snapshot() -> Option<TransactionsStatusSnapshot> {
    let running = crate::transactions::is_global_transaction_service_running().await;
    let system_ready = TRANSACTIONS_SYSTEM_READY.load(Ordering::SeqCst);
    let global_pending = crate::transactions::utils::get_pending_transactions_count().await as u64;

    let mut stats = crate::transactions::TransactionStats::default();
    let mut wallet_pubkey: Option<String> = None;
    let mut last_signature_check: Option<String> = None;
    let mut pending_entries: Vec<(String, DateTime<Utc>)> = Vec::new();
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
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("Failed to load transactions database stats: {}", err),
                );
            }
        }

        match db.get_successful_transactions_count().await {
            Ok(count) => stats.successful_transactions_count = count,
            Err(err) => log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load successful transaction count: {}", err),
            ),
        }

        match db.get_failed_transactions_count().await {
            Ok(count) => stats.failed_transactions_count = count,
            Err(err) => log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load failed transaction count: {}", err),
            ),
        }

        match db.get_bootstrap_state().await {
            Ok(state) => {
                bootstrap_snapshot = Some(TransactionBootstrapSnapshot {
                    full_history_completed: state.full_history_completed,
                    backfill_cursor: state.backfill_before_cursor,
                });
            }
            Err(err) => log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load transactions bootstrap state: {}", err),
            ),
        }

        match db.get_newest_known_signature().await {
            Ok(sig) => newest_signature = sig,
            Err(err) => log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load newest known signature: {}", err),
            ),
        }

        match db.get_oldest_known_signature().await {
            Ok(sig) => oldest_signature = sig,
            Err(err) => log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load oldest known signature: {}", err),
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
