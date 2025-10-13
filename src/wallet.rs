use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
/// Wallet Balance Monitoring Module
///
/// This module provides wallet balance monitoring with historical snapshots stored in SQLite database.
/// It monitors both SOL balance and token balances for the configured wallet address.
///
/// Features:
/// - Background service that checks wallet balance every minute
/// - Delayed RPC calls to avoid overwhelming the global RPC client
/// - Historical snapshots stored in data/wallet.db
/// - Tracks both SOL and token balances
/// - Integration with existing RPC infrastructure
/// - Pure wallet monitoring without position management interference
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify};

use crate::config::with_config;
use crate::global::is_debug_wallet_enabled;
use crate::logger::{log, LogTag};
use crate::rpc::{get_rpc_client, TokenAccountInfo};
use crate::tokens::store::get_global_token_store;
use crate::transactions::get_transaction_database;
use crate::utils::get_wallet_address;

// Database schema version
const WALLET_SCHEMA_VERSION: u32 = 2;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_WALLET_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address TEXT NOT NULL,
    snapshot_time TEXT NOT NULL,
    sol_balance REAL NOT NULL,
    sol_balance_lamports INTEGER NOT NULL,
    total_tokens_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_TOKEN_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_WALLET_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Cache table for pre-aggregated SOL flows (one row per processed transaction)
const SCHEMA_SOL_FLOW_CACHE: &str = r#"
CREATE TABLE IF NOT EXISTS sol_flow_cache (
    signature TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    sol_delta REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Indexes for fast range aggregation on cache
const FLOW_CACHE_INDEXES: &[&str] =
    &["CREATE INDEX IF NOT EXISTS idx_flow_cache_timestamp ON sol_flow_cache(timestamp DESC);"];

// Performance indexes
const WALLET_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_address ON wallet_snapshots(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_time ON wallet_snapshots(snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_id ON token_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON token_balances(mint);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_mint ON token_balances(snapshot_id, mint);",
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Wallet balance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub id: Option<i64>,
    pub wallet_address: String,
    pub snapshot_time: DateTime<Utc>,
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub token_balances: Vec<TokenBalance>,
}

/// Token balance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub id: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub mint: String,
    pub balance: u64,    // Raw token amount
    pub balance_ui: f64, // UI amount (adjusted for decimals)
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
}

/// Wallet monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMonitorStats {
    pub total_snapshots: u64,
    pub latest_snapshot_time: Option<DateTime<Utc>>,
    pub wallet_address: String,
    pub current_sol_balance: Option<f64>,
    pub current_tokens_count: Option<u32>,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

/// High level wallet summary for dashboard consumers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummarySnapshot {
    pub window_hours: i64,
    pub current_sol_balance: f64,
    pub previous_sol_balance: Option<f64>,
    pub sol_change: f64,
    pub sol_change_percent: Option<f64>,
    pub token_count: u32,
    pub last_snapshot_time: Option<String>,
}

/// Aggregated wallet flow metrics calculated from transactions module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowMetrics {
    pub window_hours: i64,
    pub inflow_sol: f64,
    pub outflow_sol: f64,
    pub net_sol: f64,
    pub transactions_analyzed: usize,
}

/// Data point for SOL balance trend chart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalancePoint {
    pub timestamp: i64,
    pub sol_balance: f64,
}

/// Daily flow data point for time-series chart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyFlowPoint {
    pub date: String,    // YYYY-MM-DD
    pub timestamp: i64,  // Unix timestamp for charting
    pub inflow: f64,     // SOL inflow that day
    pub outflow: f64,    // SOL outflow that day
    pub net: f64,        // inflow - outflow
    pub tx_count: usize, // Number of transactions
}

/// Token row with enriched metadata for wallet table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTokenOverview {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub balance_ui: f64,
    pub balance_raw: u64,
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub value_sol: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub last_updated: Option<String>,
    pub dex_id: Option<String>,
}

/// Complete dashboard payload for wallet UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletDashboardData {
    pub summary: WalletSummarySnapshot,
    pub flows: WalletFlowMetrics,
    pub balance_trend: Vec<WalletBalancePoint>,
    pub daily_flows: Vec<DailyFlowPoint>, // NEW: Time-series flow data
    pub tokens: Vec<WalletTokenOverview>,
    pub last_updated: Option<String>,
}

/// Flow cache stats for diagnostics/UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowCacheStats {
    pub rows: u64,
    pub max_timestamp: Option<String>,
}

fn clamp_window_hours(window_hours: i64) -> i64 {
    // 0 = All Time (no filter)
    // Otherwise clamp to reasonable range (1 hour to 2 years)
    if window_hours == 0 {
        0
    } else {
        window_hours.clamp(1, 24 * 365 * 2)
    }
}

fn clamp_snapshot_limit(limit: usize) -> usize {
    limit.clamp(16, 2880)
}

fn clamp_token_limit(limit: usize) -> usize {
    limit.clamp(10, 1000)
}

fn calc_change_percent(current: f64, previous: f64) -> Option<f64> {
    if previous.abs() < f64::EPSILON {
        None
    } else {
        Some((current - previous) / previous * 100.0)
    }
}

fn short_mint_label(mint: &str) -> String {
    if mint.len() <= 4 {
        mint.to_string()
    } else {
        format!("{}…", &mint[..4])
    }
}

async fn compute_flow_metrics(window_hours: i64) -> Result<WalletFlowMetrics, String> {
    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "FLOW_START",
            &format!("Computing flow metrics for window: {} hours", window_hours),
        );
    }

    // All-time mode when window_hours <= 0
    if window_hours <= 0 {
        if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
            if let Ok(Some(min_ts)) = db.get_flow_cache_min_ts_sync() {
                if let Ok((inflow, outflow, tx_count)) =
                    db.aggregate_cached_flows_sync(min_ts, None)
                {
                    if tx_count > 0 {
                        if is_debug_wallet_enabled() {
                            log(
                                LogTag::Wallet,
                                "FLOW_CACHE_ALL",
                                &format!(
                                    "All-time cached: inflow={:.6}, outflow={:.6}, txs={}",
                                    inflow, outflow, tx_count
                                ),
                            );
                        }
                        return Ok(WalletFlowMetrics {
                            window_hours: 0,
                            inflow_sol: inflow,
                            outflow_sol: outflow,
                            net_sol: inflow - outflow,
                            transactions_analyzed: tx_count,
                        });
                    }
                }
            }
        }
        // Fallback to full aggregation from transactions DB (from epoch)
        let tx_db = get_transaction_database()
            .await
            .ok_or_else(|| "Transaction database not initialized".to_string())?;
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let (inflow, outflow, tx_count) = tx_db
            .aggregate_sol_flows_since(epoch, None)
            .await
            .map_err(|e| format!("Failed to aggregate all-time SOL flows: {}", e))?;
        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "FLOW_DB_ALL",
                &format!(
                    "All-time DB: inflow={:.6}, outflow={:.6}, txs={}",
                    inflow, outflow, tx_count
                ),
            );
        }
        return Ok(WalletFlowMetrics {
            window_hours: 0,
            inflow_sol: inflow,
            outflow_sol: outflow,
            net_sol: inflow - outflow,
            transactions_analyzed: tx_count,
        });
    }

    let window_hours = clamp_window_hours(window_hours);
    let window_start = Utc::now() - ChronoDuration::hours(window_hours);

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "FLOW_WINDOW",
            &format!("Window start: {}", window_start.to_rfc3339()),
        );
    }

    // Try cached aggregation first
    if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
        match db.aggregate_cached_flows_sync(window_start, None) {
            Ok((inflow, outflow, tx_count)) => {
                if is_debug_wallet_enabled() {
                    log(
                        LogTag::Wallet,
                        "FLOW_CACHE",
                        &format!(
                            "Cached: inflow={:.6}, outflow={:.6}, txs={}",
                            inflow, outflow, tx_count
                        ),
                    );
                }
                if tx_count > 0 {
                    return Ok(WalletFlowMetrics {
                        window_hours,
                        inflow_sol: inflow,
                        outflow_sol: outflow,
                        net_sol: inflow - outflow,
                        transactions_analyzed: tx_count,
                    });
                }
            }
            Err(e) => {
                if is_debug_wallet_enabled() {
                    log(
                        LogTag::Wallet,
                        "FLOW_CACHE_ERR",
                        &format!("Cache aggregation failed: {}", e),
                    );
                }
            }
        }
    }

    // Fallback to live aggregation from transactions DB
    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "FLOW_FALLBACK",
            "Using live aggregation from transactions DB",
        );
    }

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_string())?;
    let (inflow, outflow, tx_count) = tx_db
        .aggregate_sol_flows_since(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate SOL flows: {}", e))?;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "FLOW_DB",
            &format!(
                "DB aggregation: inflow={:.6}, outflow={:.6}, txs={}",
                inflow, outflow, tx_count
            ),
        );
    }

    Ok(WalletFlowMetrics {
        window_hours,
        inflow_sol: inflow,
        outflow_sol: outflow,
        net_sol: inflow - outflow,
        transactions_analyzed: tx_count,
    })
}

async fn compute_daily_flows(window_hours: i64) -> Result<Vec<DailyFlowPoint>, String> {
    use chrono::NaiveDate;

    let window_hours = clamp_window_hours(window_hours);
    let (window_start, is_all_time) = if window_hours == 0 {
        (DateTime::<Utc>::from(std::time::UNIX_EPOCH), true)
    } else {
        (Utc::now() - ChronoDuration::hours(window_hours), false)
    };

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_string())?;

    let daily_data = tx_db
        .aggregate_daily_flows(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate daily flows: {}", e))?;

    // Convert to DailyFlowPoint with timestamps
    let mut result: Vec<DailyFlowPoint> = daily_data
        .into_iter()
        .filter_map(|(date_str, inflow, outflow, tx_count)| {
            // Parse date string and convert to timestamp
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
                .map(|dt| DailyFlowPoint {
                    date: date_str,
                    timestamp: dt.timestamp(),
                    inflow,
                    outflow,
                    net: inflow - outflow,
                    tx_count,
                })
        })
        .collect();

    // Apply payload cap/decimation for very long ranges to avoid huge responses
    let (max_days, decimate_threshold_days) = with_config(|cfg| {
        (
            cfg.wallet.max_daily_flow_days,
            cfg.wallet.daily_flow_decimate_threshold_days,
        )
    });

    if result.len() > max_days {
        // Keep most recent max_days points
        result.sort_by_key(|p| p.timestamp);
        result = result.split_off(result.len() - max_days);
    }

    if result.len() > decimate_threshold_days {
        // Decimate older half to every Nth point while keeping recent quarter dense
        let len = result.len();
        let recent_keep = len / 4; // keep last quarter in full resolution
        let (older, recent) = result.split_at(len - recent_keep);
        // Choose stride to reduce older to about half of decimate_threshold_days
        let target_older = decimate_threshold_days - recent_keep.min(decimate_threshold_days / 2);
        let stride = ((older.len() as f64) / (target_older as f64))
            .ceil()
            .max(1.0) as usize;
        let decimated_older: Vec<DailyFlowPoint> = older
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if i % stride == 0 {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut merged = decimated_older;
        merged.extend_from_slice(recent);
        result = merged;
    }

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DAILY_FLOW",
            &format!("Computed {} daily flow points", result.len()),
        );
    }

    Ok(result)
}

async fn enrich_token_overview(
    balances: Vec<TokenBalance>,
    max_tokens: usize,
) -> Vec<WalletTokenOverview> {
    let mut rows = Vec::with_capacity(balances.len());

    let mut unique_mints: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for balance in &balances {
        if seen.insert(balance.mint.clone()) {
            unique_mints.push(balance.mint.clone());
        }
    }

    let metadata_map: HashMap<String, crate::tokens::types::Token> = if unique_mints.is_empty() {
        HashMap::new()
    } else {
        // Fetch tokens from cache - instant memory access
        let store = get_global_token_store();
        unique_mints
            .iter()
            .filter_map(|mint| {
                store
                    .get(mint)
                    .map(|snapshot| (mint.clone(), snapshot.data.clone()))
            })
            .collect()
    };

    for balance in balances {
        let token_meta = metadata_map.get(&balance.mint);

        let (symbol, name, price_sol, price_usd, liquidity_usd, volume_24h, last_updated, dex_id) =
            if let Some(meta) = token_meta {
                let price_sol = meta.price_dexscreener_sol.or(meta.price_pool_sol);
                let price_usd = meta.price_dexscreener_usd.or(meta.price_pool_usd);
                let liquidity_usd = meta.liquidity.as_ref().and_then(|l| l.usd);
                let volume_24h = meta.volume.as_ref().and_then(|v| v.h24);
                let last_updated = Some(meta.last_updated.to_rfc3339());
                let dex_id = meta.dex_id.clone();

                let symbol = if meta.symbol.trim().is_empty() {
                    short_mint_label(&balance.mint)
                } else {
                    meta.symbol.clone()
                };

                (
                    symbol,
                    Some(meta.name.clone()),
                    price_sol,
                    price_usd,
                    liquidity_usd,
                    volume_24h,
                    last_updated,
                    dex_id,
                )
            } else {
                (
                    short_mint_label(&balance.mint),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            };

        let value_sol = price_sol.map(|price| price * balance.balance_ui);

        rows.push(WalletTokenOverview {
            mint: balance.mint.clone(),
            symbol,
            name,
            balance_ui: balance.balance_ui,
            balance_raw: balance.balance,
            decimals: balance.decimals,
            is_token_2022: balance.is_token_2022,
            price_sol,
            price_usd,
            value_sol,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        });
    }

    rows.sort_by(|a, b| {
        let a_key = a.value_sol.unwrap_or(a.balance_ui);
        let b_key = b.value_sol.unwrap_or(b.balance_ui);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_tokens = clamp_token_limit(max_tokens);
    if rows.len() > max_tokens {
        rows.truncate(max_tokens);
    }

    rows
}

pub async fn get_wallet_dashboard_data(
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
) -> Result<WalletDashboardData, String> {
    let window_hours = clamp_window_hours(window_hours);
    let snapshot_limit = clamp_snapshot_limit(snapshot_limit);

    let mut snapshots = match get_recent_wallet_snapshots(snapshot_limit).await {
        Ok(snaps) => snaps,
        Err(err) => {
            if err.contains("not initialized") {
                if let Err(init_err) = initialize_wallet_database().await {
                    return Err(format!("Wallet database unavailable: {}", init_err));
                }

                match get_recent_wallet_snapshots(snapshot_limit).await {
                    Ok(snaps) => snaps,
                    Err(retry_err) => {
                        if retry_err.contains("not initialized") {
                            Vec::new()
                        } else {
                            return Err(retry_err);
                        }
                    }
                }
            } else {
                return Err(err);
            }
        }
    };
    if snapshots.is_empty() {
        let flows = compute_flow_metrics(window_hours).await?;
        let daily_flows = compute_daily_flows(window_hours)
            .await
            .unwrap_or_else(|_| Vec::new());
        return Ok(WalletDashboardData {
            summary: WalletSummarySnapshot {
                window_hours,
                current_sol_balance: 0.0,
                previous_sol_balance: None,
                sol_change: 0.0,
                sol_change_percent: None,
                token_count: 0,
                last_snapshot_time: None,
            },
            flows,
            balance_trend: Vec::new(),
            daily_flows,
            tokens: Vec::new(),
            last_updated: None,
        });
    }

    snapshots.sort_by(|a, b| a.snapshot_time.cmp(&b.snapshot_time));

    let latest_snapshot = snapshots
        .last()
        .cloned()
        .ok_or_else(|| "Latest snapshot unavailable".to_string())?;
    // Determine window_start for trend; for all-time (0), include all loaded snapshots
    let window_start = if window_hours == 0 {
        snapshots
            .first()
            .map(|s| s.snapshot_time)
            .unwrap_or(latest_snapshot.snapshot_time)
    } else {
        Utc::now() - ChronoDuration::hours(window_hours)
    };

    let baseline_snapshot = snapshots
        .iter()
        .find(|snap| snap.snapshot_time >= window_start)
        .or_else(|| snapshots.first())
        .cloned();

    let previous_sol_balance = baseline_snapshot.as_ref().map(|snap| snap.sol_balance);
    let sol_change =
        latest_snapshot.sol_balance - previous_sol_balance.unwrap_or(latest_snapshot.sol_balance);
    let sol_change_percent = previous_sol_balance
        .and_then(|prev| calc_change_percent(latest_snapshot.sol_balance, prev));

    let mut trend: Vec<WalletBalancePoint> = snapshots
        .iter()
        .filter(|snap| snap.snapshot_time >= window_start)
        .map(|snap| WalletBalancePoint {
            timestamp: snap.snapshot_time.timestamp(),
            sol_balance: snap.sol_balance,
        })
        .collect();

    if trend.is_empty() {
        trend.push(WalletBalancePoint {
            timestamp: latest_snapshot.snapshot_time.timestamp(),
            sol_balance: latest_snapshot.sol_balance,
        });
    }

    let mut tokens = Vec::new();
    if let Some(snapshot_id) = latest_snapshot.id {
        let balances = get_snapshot_token_balances(snapshot_id).await?;
        tokens = enrich_token_overview(balances, max_tokens).await;
    }

    let flows = compute_flow_metrics(window_hours).await?;

    // Compute daily flows for chart
    let daily_flows = compute_daily_flows(window_hours).await.unwrap_or_else(|e| {
        log(
            LogTag::Wallet,
            "DAILY_FLOW_ERR",
            &format!("Failed to compute daily flows: {}", e),
        );
        Vec::new()
    });

    let summary = WalletSummarySnapshot {
        window_hours,
        current_sol_balance: latest_snapshot.sol_balance,
        previous_sol_balance,
        sol_change,
        sol_change_percent,
        token_count: latest_snapshot.total_tokens_count,
        last_snapshot_time: Some(latest_snapshot.snapshot_time.to_rfc3339()),
    };

    Ok(WalletDashboardData {
        summary,
        flows,
        balance_trend: trend,
        daily_flows,
        tokens,
        last_updated: Some(latest_snapshot.snapshot_time.to_rfc3339()),
    })
}

// =============================================================================
// WALLET DATABASE MANAGER
// =============================================================================

/// Database manager for wallet balance monitoring
pub struct WalletDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}

impl WalletDatabase {
    /// Create new WalletDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        // Database should be at data/wallet.db
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("wallet.db");
        let database_path_str = database_path.to_string_lossy().to_string();

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "INIT",
                &format!("Initializing wallet database at: {}", database_path_str),
            );
        }

        // Configure connection manager
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(3) // Small pool for wallet monitoring
            .min_idle(Some(1))
            .build(manager)
            .map_err(|e| format!("Failed to create wallet connection pool: {}", e))?;

        let mut db = WalletDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: WALLET_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema().await?;

        log(
            LogTag::Wallet,
            "READY",
            "Wallet database initialized successfully",
        );
        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Configure database settings
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;

        // Create all tables
        conn.execute(SCHEMA_WALLET_SNAPSHOTS, [])
            .map_err(|e| format!("Failed to create wallet_snapshots table: {}", e))?;

        conn.execute(SCHEMA_TOKEN_BALANCES, [])
            .map_err(|e| format!("Failed to create token_balances table: {}", e))?;

        conn.execute(SCHEMA_WALLET_METADATA, [])
            .map_err(|e| format!("Failed to create wallet_metadata table: {}", e))?;

        // Flow cache tables
        conn.execute(SCHEMA_SOL_FLOW_CACHE, [])
            .map_err(|e| format!("Failed to create sol_flow_cache table: {}", e))?;

        // Create all indexes
        for index_sql in WALLET_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create wallet index: {}", e))?;
        }
        for index_sql in FLOW_CACHE_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create flow cache index: {}", e))?;
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set wallet schema version: {}", e))?;

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SCHEMA",
                "Wallet database schema initialized with all tables and indexes",
            );
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get wallet database connection: {}", e))
    }

    /// Aggregate pre-cached SOL flows for a given time window
    pub fn aggregate_cached_flows_sync(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN sol_delta > 0 THEN sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN sol_delta < 0 THEN -sol_delta ELSE 0 END), 0), \
                COUNT(signature) \
             FROM sol_flow_cache \
             WHERE timestamp >= ?1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(from.to_rfc3339())];
        if let Some(to_ts) = to {
            query.push_str(&format!(" AND timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare cached flow aggregation query: {}", e))?;
        let (inflow, outflow, count) = stmt
            .query_row(params_refs.as_slice(), |row| {
                let inflow = row.get::<_, Option<f64>>(0)?.unwrap_or(0.0);
                let outflow = row.get::<_, Option<f64>>(1)?.unwrap_or(0.0);
                let count = row.get::<_, i64>(2)?.max(0) as usize;
                Ok((inflow, outflow, count))
            })
            .map_err(|e| format!("Failed to aggregate cached SOL flows: {}", e))?;
        Ok((inflow, outflow, count))
    }

    /// Upsert a batch of flow rows into cache
    pub fn upsert_flow_rows_sync(
        &self,
        rows: &[(String, DateTime<Utc>, f64)],
    ) -> Result<usize, String> {
        if rows.is_empty() {
            return Ok(0);
        }
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start flow cache transaction: {}", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO sol_flow_cache(signature, timestamp, sol_delta) VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| format!("Failed to prepare flow cache upsert: {}", e))?;
            for (sig, ts, delta) in rows.iter() {
                stmt.execute(params![sig, ts.to_rfc3339(), *delta])
                    .map_err(|e| format!("Failed to upsert flow row: {}", e))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit flow cache upserts: {}", e))?;
        Ok(rows.len())
    }

    /// Get the max timestamp present in the flow cache
    pub fn get_flow_cache_max_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MAX(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare max timestamp query: {}", e))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query max timestamp: {}", e))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached max timestamp: {}", e))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get the minimum timestamp present in the flow cache (earliest record)
    pub fn get_flow_cache_min_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MIN(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare min timestamp query: {}", e))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query min timestamp: {}", e))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached min timestamp: {}", e))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get flow cache stats (row count and latest timestamp)
    pub fn get_flow_cache_stats_sync(&self) -> Result<WalletFlowCacheStats, String> {
        let conn = self.get_connection()?;
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM sol_flow_cache", [], |row| row.get(0))
            .unwrap_or(0);
        let max_ts = self.get_flow_cache_max_ts_sync()?.map(|dt| dt.to_rfc3339());
        Ok(WalletFlowCacheStats {
            rows: rows.max(0) as u64,
            max_timestamp: max_ts,
        })
    }

    /// Save wallet snapshot with token balances (synchronous version)
    pub fn save_wallet_snapshot_sync(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            ) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SAVE",
                &format!(
                    "Saved wallet snapshot ID {} with {} tokens for {}",
                    snapshot_id,
                    snapshot.token_balances.len(),
                    &snapshot.wallet_address[..8]
                ),
            );
        }

        Ok(snapshot_id)
    }

    /// Save wallet snapshot with token balances (async version)
    pub async fn save_wallet_snapshot(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            ) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SAVE",
                &format!(
                    "Saved wallet snapshot ID {} with {} tokens for {}",
                    snapshot_id,
                    snapshot.token_balances.len(),
                    &snapshot.wallet_address[..8]
                ),
            );
        }

        Ok(snapshot_id)
    }

    /// Get recent wallet snapshots (synchronous version)
    pub fn get_recent_snapshots_sync(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    token_balances: Vec::new(), // Will be loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?);
        }

        Ok(snapshots)
    }

    /// Get recent wallet snapshots (async version)
    pub async fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    token_balances: Vec::new(), // Will be loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?);
        }

        Ok(snapshots)
    }

    /// Get wallet monitoring statistics (synchronous version)
    pub fn get_monitor_stats_sync(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_string(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get wallet monitoring statistics (async version)
    pub async fn get_monitor_stats(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_string(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get token balances for a specific snapshot (synchronous version)
    pub fn get_token_balances_sync(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get(5)?,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?,
            );
        }

        Ok(balances)
    }

    /// Get token balances for a specific snapshot (async version)
    pub async fn get_token_balances(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get(5)?,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?,
            );
        }

        Ok(balances)
    }

    /// Cleanup old snapshots (keep last 1000) - synchronous version
    pub fn cleanup_old_snapshots_sync(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Wallet,
                "CLEANUP",
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }

    /// Cleanup old snapshots (keep last 1000) - async version
    pub async fn cleanup_old_snapshots(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Wallet,
                "CLEANUP",
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }
}

// =============================================================================
// GLOBAL WALLET DATABASE INSTANCE
// =============================================================================

/// Global wallet database instance
pub(crate) static GLOBAL_WALLET_DB: Lazy<Arc<Mutex<Option<WalletDatabase>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global wallet database
pub async fn initialize_wallet_database() -> Result<(), String> {
    let mut db_lock = GLOBAL_WALLET_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = WalletDatabase::new().await?;
    *db_lock = Some(db);

    log(
        LogTag::Wallet,
        "INIT",
        "Global wallet database initialized successfully",
    );
    Ok(())
}

// Helper functions removed to avoid lifetime issues - using direct database access instead

// =============================================================================
// WALLET MONITORING SERVICE
// =============================================================================

/// Collect current wallet balance and token balances
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    // Get wallet address
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;

    let rpc_client = get_rpc_client();
    let snapshot_time = Utc::now();

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "COLLECT",
            &format!("Collecting wallet snapshot for {}", &wallet_address[..8]),
        );
    }

    // Add small delay to avoid overwhelming RPC client
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get SOL balance
    let sol_balance = rpc_client
        .get_sol_balance(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get SOL balance: {}", e))?;

    let sol_balance_lamports = crate::utils::sol_to_lamports(sol_balance);

    // Add another small delay before token accounts fetch
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get all token accounts
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    // Convert to TokenBalance format
    let mut token_balances = Vec::new();
    for account_info in &token_accounts {
        // Skip accounts with zero balance
        if account_info.balance == 0 {
            continue;
        }

        let balance_ui = if let Some(decimals) =
            crate::tokens::decimals::get_cached_decimals(&account_info.mint)
        {
            (account_info.balance as f64) / (10_f64).powi(decimals as i32)
        } else {
            account_info.balance as f64 // Fallback without decimals
        };

        token_balances.push(TokenBalance {
            id: None,
            snapshot_id: None,
            mint: account_info.mint.clone(),
            balance: account_info.balance,
            balance_ui,
            decimals: crate::tokens::decimals::get_cached_decimals(&account_info.mint),
            is_token_2022: account_info.is_token_2022,
        });
    }

    let total_tokens_count = token_balances.len() as u32;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "SNAPSHOT",
            &format!(
                "Collected snapshot: SOL {:.6}, {} tokens",
                sol_balance, total_tokens_count
            ),
        );
    }

    Ok(WalletSnapshot {
        id: None,
        wallet_address,
        snapshot_time,
        sol_balance,
        sol_balance_lamports,
        total_tokens_count,
        token_balances,
    })
}

/// Background service for wallet monitoring
pub async fn start_wallet_monitoring_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        monitor.instrument(async move {
            log(LogTag::Wallet, "START", "Wallet monitoring service started (instrumented)");

            // Initialize database
            if let Err(e) = initialize_wallet_database().await {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("Failed to initialize wallet database: {}", e)
                );
                return;
            }

            let snapshot_interval = with_config(|cfg| cfg.wallet.snapshot_interval_secs);
            let cache_interval_secs = with_config(|cfg| cfg.wallet.flow_cache_update_secs);
            let mut interval = tokio::time::interval(Duration::from_secs(snapshot_interval.max(10)));
            let mut flow_sync_interval = tokio::time::interval(Duration::from_secs(cache_interval_secs.max(1)));
            let mut cleanup_counter = 0;

            loop {
                tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Wallet, "SHUTDOWN", "Wallet monitoring service shutting down");
                    break;
                }
                _ = interval.tick() => {
                    // Collect wallet snapshot
                    match collect_wallet_snapshot().await {
                        Ok(snapshot) => {
                            // Save to database
                            let db_guard = GLOBAL_WALLET_DB.lock().await;
                            match db_guard.as_ref() {
                                Some(db) => {
                                    match db.save_wallet_snapshot_sync(&snapshot) {
                                        Ok(snapshot_id) => {
                                            if is_debug_wallet_enabled() {
                                                log(
                                                    LogTag::Wallet,
                                                    "SAVED",
                                                    &format!(
                                                        "Saved snapshot ID {} - SOL: {:.6}, Tokens: {}",
                                                        snapshot_id,
                                                        snapshot.sol_balance,
                                                        snapshot.total_tokens_count
                                                    )
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log(LogTag::Wallet, "ERROR", &format!("Failed to save wallet snapshot: {}", e));
                                        }
                                    }
                                }
                                None => {
                                    log(LogTag::Wallet, "ERROR", "Wallet database not initialized");
                                }
                            }
                        }
                        Err(e) => {
                            log(LogTag::Wallet, "ERROR", &format!("Failed to collect wallet snapshot: {}", e));
                        }
                    }

                    // Cleanup old snapshots every 60 intervals (1 hour)
                    cleanup_counter += 1;
                    if cleanup_counter >= 60 {
                        cleanup_counter = 0;

                        let db_guard = GLOBAL_WALLET_DB.lock().await;
                        match db_guard.as_ref() {
                            Some(db) => {
                                if let Err(e) = db.cleanup_old_snapshots_sync() {
                                    log(LogTag::Wallet, "WARN", &format!("Failed to cleanup old snapshots: {}", e));
                                }
                            }
                            None => {
                                log(LogTag::Wallet, "WARN", "Wallet database not initialized for cleanup");
                            }
                        }
                    }
                }
                _ = flow_sync_interval.tick() => {
                    // Periodically sync SOL flow cache from transactions DB
                    let (batch_size, lookback_secs) = with_config(|cfg| (cfg.wallet.flow_cache_backfill_batch, cfg.wallet.flow_cache_lookback_secs));
                    // Step 1: read current max cached ts under short lock
                    let start_ts = {
                        let db_guard = GLOBAL_WALLET_DB.lock().await;
                        if let Some(wallet_db) = db_guard.as_ref() {
                            match wallet_db.get_flow_cache_max_ts_sync() {
                                Ok(Some(ts)) => ts - ChronoDuration::seconds(lookback_secs as i64),
                                Ok(None) => Utc::now() - ChronoDuration::hours(24),
                                Err(_) => Utc::now() - ChronoDuration::hours(24),
                            }
                        } else {
                            // Wallet DB not ready yet
                            continue;
                        }
                    };

                    // Step 2: export rows from transactions DB without holding wallet lock
                    let rows = if let Some(tx_db) = get_transaction_database().await {
                        match tx_db.export_processed_for_wallet_flow(start_ts, batch_size).await {
                            Ok(rows) => rows,
                            Err(e) => {
                                log(LogTag::Wallet, "FLOW_SYNC_ERR", &format!("Failed to export processed rows: {}", e));
                                Vec::new()
                            }
                        }
                    } else { Vec::new() };

                    if rows.is_empty() { continue; }

                    // Step 3: upsert into wallet cache under short lock
                    let mapped: Vec<(String, DateTime<Utc>, f64)> = rows
                        .into_iter()
                        .map(|r| (r.signature, r.timestamp, r.sol_delta))
                        .collect();
                    let db_guard = GLOBAL_WALLET_DB.lock().await;
                    if let Some(wallet_db) = db_guard.as_ref() {
                        if let Err(e) = wallet_db.upsert_flow_rows_sync(&mapped) {
                            log(LogTag::Wallet, "FLOW_SYNC_ERR", &format!("Failed to upsert flow cache rows: {}", e));
                        } else if is_debug_wallet_enabled() {
                            log(LogTag::Wallet, "FLOW_SYNC", &format!("Upserted {} flow cache rows", mapped.len()));
                        }
                    }
                }
            }
            }

            log(LogTag::Wallet, "STOPPED", "Wallet monitoring service stopped");
        })
    )
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Get recent wallet snapshots
pub async fn get_recent_wallet_snapshots(limit: usize) -> Result<Vec<WalletSnapshot>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            // Use the synchronous version to avoid lifetime issues
            db.get_recent_snapshots_sync(limit)
        }
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get wallet monitoring statistics
pub async fn get_wallet_monitor_stats() -> Result<WalletMonitorStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_monitor_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get token balances for a snapshot
pub async fn get_snapshot_token_balances(snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_balances_sync(snapshot_id),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get current wallet status (latest snapshot data)
pub async fn get_current_wallet_status() -> Result<Option<WalletSnapshot>, String> {
    let snapshots = get_recent_wallet_snapshots(1).await?;
    Ok(snapshots.into_iter().next())
}

/// Public accessor for flow cache stats
pub async fn get_flow_cache_stats() -> Result<WalletFlowCacheStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_flow_cache_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}
