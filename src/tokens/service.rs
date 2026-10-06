// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token service - ServiceManager integration
//!
//! Orchestrates all token system background tasks:
//! - Database initialization
//! - Cache setup  
//! - Update loops (priority-based)
//! - Cleanup tasks
//!
//! This service coordinates the new architecture with proper lifecycle management.

use crate::chains::{enabled_chains, ChainId, PerChain};
use crate::global::TOKENS_SYSTEM_READY;
use crate::logger::{self, LogTag};
use crate::paths::{chain_db_path, DbKind};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::cleanup;
use crate::tokens::database::TokenDatabase;
use crate::tokens::discovery;
use crate::tokens::updates;
use crate::tokens::updates::RateLimitCoordinator;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Each chain's share of the provider rate budget, built on first use.
static RATE_BUDGETS: PerChain<Arc<RateLimitCoordinator>> = PerChain::new(new_rate_budget);

fn new_rate_budget(chain: ChainId) -> Arc<RateLimitCoordinator> {
    Arc::new(RateLimitCoordinator::for_chain(chain))
}

/// The rate budget every provider call made for `chain` draws from.
pub fn rate_budget(chain: ChainId) -> Arc<RateLimitCoordinator> {
    RATE_BUDGETS.get(chain).clone()
}

/// New tokens service using clean architecture
#[derive(Default)]
pub struct TokensServiceNew {
    /// One open database per enabled chain, in `enabled_chains()` order.
    databases: Vec<Arc<TokenDatabase>>,
}

/// Open one chain's token database, publish it, and warm that chain's caches.
async fn open_chain_database(chain: ChainId) -> crate::Result<Arc<TokenDatabase>> {
    // Schema is initialized automatically in new()
    let db_path = chain_db_path(DbKind::Tokens, chain);
    let db = TokenDatabase::new(&db_path.to_string_lossy(), chain).map_err(|e| {
        crate::Error::Service(crate::errors::ServiceError::Initialize {
            service: "tokens_new".to_owned(),
            message: format!("Failed to create database: {e}"),
        })
    })?;
    let db_arc = Arc::new(db);

    // Publish the database for decimals and every other per-chain reader
    crate::tokens::database::install_database(db_arc.clone());

    // Preload known decimals into memory cache for synchronous pool decoder access.
    // This is CRITICAL: pool decoders run synchronously and need decimals available in
    // cache — a miss makes them skip the pool, so the token has no live price. The query
    // is capped at the cache capacity and ordered so pool-backed mints are inserted last;
    // loading the whole table (405k rows against a 100k cache) evicted most of them.
    let preload_start = std::time::Instant::now();
    let db_for_preload = db_arc.clone();
    let all_decimals = tokio::task::spawn_blocking(move || {
        db_for_preload
            .get_tokens_with_decimals_for_preload(crate::tokens::decimals::PRELOAD_CAPACITY)
    })
    .await
    .map_err(|e| {
        crate::Error::Service(crate::errors::ServiceError::Initialize {
            service: "tokens_new".to_owned(),
            message: format!("Failed to spawn decimals preload task: {e}"),
        })
    })?
    .map_err(|e| {
        crate::Error::Service(crate::errors::ServiceError::Initialize {
            service: "tokens_new".to_owned(),
            message: format!("Failed to fetch decimals from database: {e}"),
        })
    })?;

    // The query already excluded invalid values, and its order is load-bearing —
    // insert as returned so pool-backed mints end up most recently used.
    let preloaded_count = all_decimals.len();
    for (mint, decimals) in all_decimals {
        crate::tokens::decimals::cache(chain, &mint, decimals);
    }

    logger::info(
        LogTag::Tokens,
        &format!(
            "Preloaded {} token decimals into memory cache in {:.2}ms",
            preloaded_count,
            preload_start.elapsed().as_secs_f64() * 1000.0
        ),
    );

    // Load blocked authorities from DB into this chain's memory cache
    let db_for_auth_loader = db_arc.clone();
    match tokio::task::spawn_blocking(move || db_for_auth_loader.load_blocked_authorities()).await {
        Ok(Ok(blocked)) => {
            let count = blocked.len();
            crate::tokens::authority_cache::refresh_blocked_from_db(chain, blocked);
            if count > 0 {
                logger::info(
                    LogTag::Tokens,
                    &format!("Loaded {count} blocked authorities from DB"),
                );
            }
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to load blocked authorities: {e}"),
            );
        }
        Err(e) => {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to spawn authority load task: {e}"),
            );
        }
    }

    logger::info(
        LogTag::Tokens,
        &format!("Service initialized with database at {}", db_path.display()),
    );
    Ok(db_arc)
}

/// Authority reputation discovery for one chain's database (every 5 minutes).
fn start_authority_discovery_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Wait 60s before first run to let other systems warm up
        tokio::select! {
            _ = shutdown.notified() => return,
            _ = tokio::time::sleep(Duration::from_secs(60)) => {}
        }

        let interval = Duration::from_secs(300); // 5 minutes
        loop {
            // Run authority discovery analysis
            let db_clone = db.clone();
            match tokio::task::spawn_blocking(move || db_clone.run_authority_discovery(5, 0.8))
                .await
            {
                Ok(Ok(newly_blocked)) => {
                    if newly_blocked > 0 {
                        logger::info(
                            LogTag::Filtering,
                            &format!(
                                "Authority discovery: {} new blocked authorities",
                                newly_blocked
                            ),
                        );
                    }
                    // Refresh this chain's in-memory blocked set from DB
                    let db_reload = db.clone();
                    if let Ok(Ok(blocked)) =
                        tokio::task::spawn_blocking(move || db_reload.load_blocked_authorities())
                            .await
                    {
                        crate::tokens::authority_cache::refresh_blocked_from_db(
                            db.chain(),
                            blocked,
                        );
                    }
                }
                Ok(Err(e)) => {
                    logger::warning(
                        LogTag::Filtering,
                        &format!("Authority discovery error: {e}"),
                    );
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Filtering,
                        &format!("Authority discovery task panic: {e}"),
                    );
                }
            }

            tokio::select! {
                _ = shutdown.notified() => break,
                _ = tokio::time::sleep(interval) => {}
            }
        }
    })
}

#[async_trait]
impl Service for TokensServiceNew {
    fn name(&self) -> &'static str {
        "tokens_new"
    }

    fn priority(&self) -> i32 {
        40 // Before webserver and trader; after core infra
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["events", "transactions", "pools"]
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        let mut databases = Vec::with_capacity(enabled_chains().len());
        for &chain in enabled_chains() {
            match open_chain_database(chain).await {
                Ok(db) => databases.push(db),
                Err(e) => {
                    // Earlier chains are already installed; withdraw them so a
                    // failed start leaves no database reachable.
                    crate::tokens::database::uninstall_databases();
                    return Err(e);
                }
            }
        }
        self.databases = databases;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        if self.databases.is_empty() {
            return Err(crate::Error::Service(crate::errors::ServiceError::Start {
                service: "tokens_new".to_owned(),
                message: "Database not initialized".to_owned(),
            }));
        }

        // TODO: Wire up metrics instrumentation
        logger::warning(
            LogTag::Tokens,
            "Metrics collection not yet implemented - TaskMonitor not instrumented",
        );
        let _ = monitor;

        // A single refill task (every minute) tops up every chain's rate budget. Each
        // chain holds a fair share of one provider budget, so the total stays one budget
        // however many chains run.
        let shutdown_refill = shutdown.clone();
        let refill_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_refill.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {
                        for (_, budget) in RATE_BUDGETS.built() {
                            budget.top_up_all();
                        }
                    }
                }
            }
        });
        let mut handles = vec![refill_handle];

        for db in &self.databases {
            let coordinator = rate_budget(db.chain());

            // Start update loops (critical, high, low priority)
            handles.extend(updates::start_update_loop(
                db.clone(),
                shutdown.clone(),
                coordinator.clone(),
            ));

            // Start discovery loop (new token discovery)
            handles.push(discovery::start_discovery_loop(
                db.clone(),
                shutdown.clone(),
                coordinator.clone(),
            ));

            // Start cleanup loop (hourly)
            handles.push(cleanup::start_cleanup_loop(db.clone(), shutdown.clone()));

            // Keep on-chain logos and published banners current from the data service
            handles.push(crate::tokens::media::start_media_sync_loop(
                db.clone(),
                shutdown.clone(),
            ));

            // Start authority reputation discovery loop
            handles.push(start_authority_discovery_loop(db.clone(), shutdown.clone()));
        }

        logger::info(
            LogTag::Tokens,
            &format!("Service started with {} background tasks", handles.len()),
        );

        // Mark tokens system ready after every chain's loops are started
        TOKENS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Tokens, "Service stopping...");
        // On stop, mark as not ready
        TOKENS_SYSTEM_READY.store(false, std::sync::atomic::Ordering::SeqCst);
        // Withdraw every chain's published database
        crate::tokens::database::uninstall_databases();
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if self.databases.is_empty() {
            ServiceHealth::Starting
        } else {
            ServiceHealth::Healthy
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        // Return cache metrics
        let dex_metrics = crate::tokens::market::dexscreener::get_cache_metrics();
        let dex_size = crate::tokens::market::dexscreener::get_cache_size();
        let gecko_metrics = crate::tokens::market::geckoterminal::get_cache_metrics();
        let gecko_size = crate::tokens::market::geckoterminal::get_cache_size();
        let rug_metrics = crate::tokens::security::rugcheck::get_cache_metrics();
        let rug_size = crate::tokens::security::rugcheck::get_cache_size();

        let mut metrics = ServiceMetrics::default();
        metrics.custom_metrics.insert(
            "dexscreener_cache_hit_rate".to_owned(),
            dex_metrics.hit_rate(),
        );
        metrics
            .custom_metrics
            .insert("dexscreener_cache_entries".to_owned(), dex_size as f64);
        metrics.custom_metrics.insert(
            "geckoterminal_cache_hit_rate".to_owned(),
            gecko_metrics.hit_rate(),
        );
        metrics
            .custom_metrics
            .insert("geckoterminal_cache_entries".to_owned(), gecko_size as f64);
        metrics
            .custom_metrics
            .insert("rugcheck_cache_hit_rate".to_owned(), rug_metrics.hit_rate());
        metrics
            .custom_metrics
            .insert("rugcheck_cache_entries".to_owned(), rug_size as f64);
        metrics
    }
}
