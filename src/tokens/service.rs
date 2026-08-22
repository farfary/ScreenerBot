//! Token service - ServiceManager integration
//!
//! Orchestrates all token system background tasks:
//! - Database initialization
//! - Cache setup  
//! - Update loops (priority-based)
//! - Cleanup tasks
//!
//! This service coordinates the new architecture with proper lifecycle management.

use crate::global::TOKENS_SYSTEM_READY;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::cleanup;
use crate::tokens::database::TokenDatabase;
use crate::tokens::discovery;
use crate::tokens::schema;
use crate::tokens::updates;
use crate::tokens::updates::RateLimitCoordinator;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

// Global rate limit coordinator for force update API
static RATE_COORDINATOR: OnceLock<Arc<RateLimitCoordinator>> = OnceLock::new();

/// Get global rate limit coordinator (for force update API)
pub fn get_rate_coordinator() -> Option<Arc<RateLimitCoordinator>> {
    RATE_COORDINATOR.get().cloned()
}

/// New tokens service using clean architecture
pub struct TokensServiceNew {
    db: Option<Arc<TokenDatabase>>,
}

impl Default for TokensServiceNew {
    fn default() -> Self {
        Self { db: None }
    }
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
        // Initialize database (schema initialized automatically in new())
        let db_path = crate::paths::get_tokens_db_path();
        let db = TokenDatabase::new(&db_path.to_string_lossy(), crate::chains::active_chain())
            .map_err(|e| {
                crate::Error::Service(crate::errors::ServiceError::Initialize {
                    service: "tokens_new".to_owned(),
                    message: format!("Failed to create database: {e}"),
                })
            })?;

        let db_arc = Arc::new(db);

        // Initialize global database for decimals module and other components
        crate::tokens::database::init_global_database(db_arc.clone()).map_err(|e| {
            crate::Error::Service(crate::errors::ServiceError::Initialize {
                service: "tokens_new".to_owned(),
                message: format!("Failed to init global database: {e}"),
            })
        })?;

        self.db = Some(db_arc.clone());

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
        let chain = db_arc.chain();
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

        // Load blocked authorities from DB into memory cache
        {
            let db_for_auth = self
                .db
                .as_ref()
                .ok_or_else(|| {
                    crate::Error::Service(crate::errors::ServiceError::Initialize {
                        service: "TokenService".to_owned(),
                        message: "Database not initialized for authority loading".to_owned(),
                    })
                })?
                .clone();
            let db_for_auth_loader = db_for_auth.clone();
            match tokio::task::spawn_blocking(move || db_for_auth_loader.load_blocked_authorities())
                .await
            {
                Ok(Ok(blocked)) => {
                    let count = blocked.len();
                    crate::tokens::authority_cache::refresh_blocked_from_db(
                        db_for_auth.chain(),
                        blocked,
                    );
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
        }

        logger::info(
            LogTag::Tokens,
            &format!("Service initialized with database at {}", db_path.display()),
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| {
                crate::Error::Service(crate::errors::ServiceError::Start {
                    service: "tokens_new".to_owned(),
                    message: "Database not initialized".to_owned(),
                })
            })?
            .clone();

        // TODO: Wire up metrics instrumentation
        logger::warning(
            LogTag::Tokens,
            "Metrics collection not yet implemented - TaskMonitor not instrumented",
        );
        let _ = monitor;

        // Create a single shared rate limit coordinator for all token tasks
        let coordinator = Arc::new(RateLimitCoordinator::new());

        // Store coordinator globally for force update API
        let _ = RATE_COORDINATOR.set(coordinator.clone());

        // Start a single refill task (every minute) shared by all loops
        let coord_refill = coordinator.clone();
        let shutdown_refill = shutdown.clone();
        let refill_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_refill.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {
                        coord_refill.refill_all();
                    }
                }
            }
        });

        // Start update loops (critical, high, low priority)
        let mut handles =
            updates::start_update_loop(db.clone(), shutdown.clone(), coordinator.clone());
        handles.push(refill_handle);

        // Start discovery loop (new token discovery)
        let discovery_handle =
            discovery::start_discovery_loop(db.clone(), shutdown.clone(), coordinator.clone());
        handles.push(discovery_handle);

        // Start cleanup loop (hourly)
        let cleanup_handle = cleanup::start_cleanup_loop(db.clone(), shutdown.clone());
        handles.push(cleanup_handle);

        // Start authority reputation discovery loop (every 5 minutes)
        let auth_db = db.clone();
        let auth_shutdown = shutdown;
        let auth_handle = tokio::spawn(async move {
            // Wait 60s before first run to let other systems warm up
            tokio::select! {
                _ = auth_shutdown.notified() => return,
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }

            let interval = Duration::from_secs(300); // 5 minutes
            loop {
                // Run authority discovery analysis
                let db_clone = auth_db.clone();
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
                        // Refresh in-memory blocked set from DB
                        let db_reload = auth_db.clone();
                        if let Ok(Ok(blocked)) = tokio::task::spawn_blocking(move || {
                            db_reload.load_blocked_authorities()
                        })
                        .await
                        {
                            crate::tokens::authority_cache::refresh_blocked_from_db(
                                auth_db.chain(),
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
                    _ = auth_shutdown.notified() => break,
                    _ = tokio::time::sleep(interval) => {}
                }
            }
        });
        handles.push(auth_handle);

        logger::info(
            LogTag::Tokens,
            &format!("Service started with {} background tasks", handles.len()),
        );

        // Mark tokens system ready after all update loops are started
        TOKENS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Tokens, "Service stopping...");
        // On stop, mark as not ready
        TOKENS_SYSTEM_READY.store(false, std::sync::atomic::Ordering::SeqCst);
        // Clear global database reference
        crate::tokens::database::clear_global_database();
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if self.db.is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
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
