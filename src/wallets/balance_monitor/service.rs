//! Wallet monitoring service and public API functions

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::nfts::fetch_nft_metadata_batch;
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::utils::get_wallet_address;

use crate::transactions::get_transaction_database;

use super::cache::{
    canonical_window, circuit_record_failure, circuit_reset, compute_and_cache_metrics,
    compute_and_cache_metrics_internal, hydrate_wallet_snapshot_status,
    update_wallet_snapshot_status, warmup_dashboard_metrics, API_RESPONSE_CACHE, CACHE_METRICS,
};
use super::dashboard::clamp_window_hours;
use super::database::{
    get_wallet_service_metrics, increment_errors, increment_flow_syncs, increment_operations,
    increment_snapshots, GLOBAL_WALLET_DB,
};
use super::types::*;

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize the global wallet database
pub async fn initialize_wallet_database() -> Result<(), String> {
    let mut db_lock = GLOBAL_WALLET_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = super::database::WalletDatabase::new().await?;
    let latest_snapshot_time = db.get_latest_snapshot_time()?;
    *db_lock = Some(db);

    hydrate_wallet_snapshot_status(latest_snapshot_time);

    logger::info(
        LogTag::Wallet,
        "Global wallet database initialized successfully",
    );
    Ok(())
}

// =============================================================================
// WALLET MONITORING SERVICE
// =============================================================================

/// Collect current wallet balance and token balances
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    // Get wallet address
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {e}"))?;

    let rpc_client = get_rpc_client();
    let snapshot_time = Utc::now();

    logger::debug(
        LogTag::Wallet,
        &format!("Collecting wallet snapshot for {}", &wallet_address[..8]),
    );

    // Add small delay to avoid overwhelming RPC client
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get SOL balance
    let sol_balance = rpc_client
        .get_sol_balance(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get SOL balance: {e}"))?;

    let sol_balance_lamports = crate::utils::sol_to_lamports(sol_balance);

    // Add another small delay before token accounts fetch
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get all token accounts (includes both tokens and NFTs)
    let token_accounts = rpc_client
        .get_all_token_accounts_str(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {e}"))?;

    // Separate fungible tokens and NFTs
    let mut token_balances = Vec::new();
    let mut nft_mints_with_accounts: Vec<(String, String, bool)> = Vec::new(); // (mint, account, is_token_2022)

    for account_info in &token_accounts {
        // Skip accounts with zero balance
        if account_info.balance == 0 {
            continue;
        }

        // Check if this is an NFT (decimals=0 and balance=1)
        if account_info.is_nft {
            nft_mints_with_accounts.push((
                account_info.mint.clone(),
                account_info.account.clone(),
                account_info.is_token_2022,
            ));
        } else {
            // Fungible token - use decimals from RPC response
            let decimals = account_info.decimals;
            let balance_ui = (account_info.balance as f64) / (10_f64).powi(decimals as i32);

            token_balances.push(TokenBalance {
                id: None,
                snapshot_id: None,
                mint: account_info.mint.clone(),
                balance: account_info.balance,
                balance_ui,
                decimals,
                is_token_2022: account_info.is_token_2022,
            });
        }
    }

    // Fetch NFT metadata from Metaplex
    let mut nft_balances = Vec::new();
    if !nft_mints_with_accounts.is_empty() {
        let nft_mints: Vec<String> = nft_mints_with_accounts
            .iter()
            .map(|(mint, _, _)| mint.clone())
            .collect();

        logger::debug(
            LogTag::Wallet,
            &format!("Fetching metadata for {} NFTs", nft_mints.len()),
        );

        let metadata_results = fetch_nft_metadata_batch(&nft_mints).await;

        for (mint, account, is_token_2022) in nft_mints_with_accounts {
            let (name, symbol, image_url) = metadata_results
                .get(&mint)
                .and_then(|result| result.as_ref().ok())
                .map(|meta| {
                    (
                        meta.name.clone(),
                        meta.symbol.clone(),
                        meta.image_url.clone(),
                    )
                })
                .unwrap_or((None, None, None));

            nft_balances.push(NftBalance {
                id: None,
                snapshot_id: None,
                mint,
                account_address: account,
                name,
                symbol,
                image_url,
                is_token_2022,
            });
        }
    }

    let total_tokens_count = token_balances.len() as u32;
    let total_nfts_count = nft_balances.len() as u32;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Collected snapshot: SOL {:.6}, {} tokens, {} NFTs",
            sol_balance, total_tokens_count, total_nfts_count
        ),
    );

    Ok(WalletSnapshot {
        id: None,
        wallet_address,
        snapshot_time,
        sol_balance,
        sol_balance_lamports,
        total_tokens_count,
        total_nfts_count,
        token_balances,
        nft_balances,
    })
}

pub async fn start_wallet_monitoring_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        monitor.instrument(async move {
            logger::info(LogTag::Wallet, "Wallet monitoring service started (instrumented)");

            // Initialize database
            if let Err(e) = initialize_wallet_database().await {
                logger::error(
                    LogTag::Wallet,
                    &format!("Failed to initialize wallet database: {e}")
                );
                return;
            }

            warmup_dashboard_metrics().await;

            let snapshot_interval = with_config(|cfg| cfg.wallet.snapshot_interval_secs);
            let cache_interval_secs = with_config(|cfg| cfg.wallet.flow_cache_update_secs);
            let mut interval = tokio::time::interval(Duration::from_secs(snapshot_interval.max(10)));
            let mut flow_sync_interval = tokio::time::interval(Duration::from_secs(cache_interval_secs.max(1)));
            let (metrics_24h_secs, metrics_7d_secs, metrics_30d_secs, metrics_all_secs) =
                with_config(|cfg| {
                    (
                        cfg.wallet.dashboard_metrics_24h_interval_secs.max(30),
                        cfg.wallet.dashboard_metrics_7d_interval_secs.max(60),
                        cfg.wallet.dashboard_metrics_30d_interval_secs.max(300),
                        cfg.wallet.dashboard_metrics_alltime_interval_secs.max(300),
                    )
                });
            let mut metrics_24h_interval =
                tokio::time::interval(Duration::from_secs(metrics_24h_secs));
            let mut metrics_7d_interval =
                tokio::time::interval(Duration::from_secs(metrics_7d_secs));
            let mut metrics_30d_interval =
                tokio::time::interval(Duration::from_secs(metrics_30d_secs));
            let mut metrics_all_interval =
                tokio::time::interval(Duration::from_secs(metrics_all_secs));
            let mut cleanup_counter = 0;

            loop {
                tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Wallet, "Wallet monitoring service shutting down");
                    break;
                }
                _ = interval.tick() => {
                    // Collect wallet snapshot
                    match collect_wallet_snapshot().await {
                        Ok(snapshot) => {
                            // Track metrics
                            increment_operations();
                            increment_snapshots();

                            // Save to database
                            let db_guard = GLOBAL_WALLET_DB.lock().await;
                            match db_guard.as_ref() {
                                Some(db) => {
                                    match db.save_wallet_snapshot_sync(&snapshot) {
                                        Ok(snapshot_id) => {
                                            logger::debug(
                                                LogTag::Wallet,
                                                &format!(
                                                    "Saved snapshot ID {} - SOL: {:.6}, Tokens: {}",
                                                    snapshot_id,
                                                    snapshot.sol_balance,
                                                    snapshot.total_tokens_count
                                                )
                                            );
                                        }
                                        Err(e) => {
                                            logger::error(LogTag::Wallet, &format!("Failed to save wallet snapshot: {e}"));
                                        }
                                    }
                                }
                                None => {
                                    logger::error(LogTag::Wallet, "Wallet database not initialized");
                                }
                            }
                        }
                        Err(e) => {
                            increment_errors();
                            logger::error(LogTag::Wallet, &format!("Failed to collect wallet snapshot: {e}"));
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
                                    logger::warning(LogTag::Wallet, &format!("Failed to cleanup old snapshots: {e}"));
                                }
                                if let Err(e) = db.cleanup_expired_metrics() {
                                    logger::warning(LogTag::Wallet, &format!(
                                        "Failed to cleanup expired dashboard metrics: {}",
                                        e
                                    ));
                                }
                            }
                            None => {
                                logger::warning(LogTag::Wallet, "Wallet database not initialized for cleanup");
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
                                logger::error(LogTag::Wallet, &format!("Failed to export processed rows: {e}"));
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
                            increment_errors();
                            logger::error(LogTag::Wallet, &format!("Failed to upsert flow cache rows: {e}"));
                        } else {
                            increment_flow_syncs();
                            logger::debug(LogTag::Wallet, &format!("Upserted {} flow cache rows", mapped.len()));
                        }
                    }
                }
                _ = metrics_24h_interval.tick() => {
                    compute_and_cache_metrics_internal("24h", 24).await;
                }
                _ = metrics_7d_interval.tick() => {
                    compute_and_cache_metrics_internal("7d", 168).await;
                }
                _ = metrics_30d_interval.tick() => {
                    compute_and_cache_metrics_internal("30d", 720).await;
                }
                _ = metrics_all_interval.tick() => {
                    compute_and_cache_metrics_internal("all_time", 0).await;
                }
            }
            }

            logger::info(LogTag::Wallet, "Wallet monitoring service stopped");
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

/// Get NFT balances for a snapshot
pub async fn get_snapshot_nft_balances(snapshot_id: i64) -> Result<Vec<NftBalance>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_nft_balances_sync(snapshot_id),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get current wallet status (latest snapshot data)
pub async fn get_current_wallet_status() -> Result<Option<WalletSnapshot>, String> {
    let snapshots = get_recent_wallet_snapshots(1).await?;
    Ok(snapshots.into_iter().next())
}

/// Get SOL balance at or before a specific time (optimized single-value query)
pub async fn get_balance_at_time(target_time: DateTime<Utc>) -> Result<Option<f64>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_balance_at_time_sync(target_time),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Public accessor for flow cache stats
pub async fn get_flow_cache_stats() -> Result<WalletFlowCacheStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_flow_cache_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}

pub async fn refresh_dashboard_cache(window_hours: i64) -> Result<(), String> {
    let window_hours = clamp_window_hours(window_hours);
    let (window_key, canonical_hours) =
        canonical_window(window_hours).ok_or_else(|| "Unsupported window".to_string())?;

    {
        let db_guard = GLOBAL_WALLET_DB.lock().await;
        if let Some(db) = db_guard.as_ref() {
            db.invalidate_dashboard_metrics(window_key)?;
        }
    }

    if let Err(err) = compute_and_cache_metrics(window_key, canonical_hours).await {
        circuit_record_failure(window_key).await;
        Err(err)
    } else {
        circuit_reset(window_key).await;
        Ok(())
    }
}

pub async fn get_dashboard_cache_metrics() -> CachePerformanceMetrics {
    CACHE_METRICS.read().await.clone()
}

pub async fn clear_dashboard_api_cache() {
    API_RESPONSE_CACHE.invalidate_all();
}
