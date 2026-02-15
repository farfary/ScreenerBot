/// Token service - ServiceManager integration
///
/// Orchestrates all token system background tasks:
/// - Database initialization
/// - Cache setup  
/// - Update loops (priority-based)
/// - Cleanup tasks
///
/// This service coordinates the new architecture with proper lifecycle management.
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
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

// Global rate limit coordinator for force update API
static RATE_COORDINATOR: OnceCell<Arc<RateLimitCoordinator>> = OnceCell::new();

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

    async fn initialize(&mut self) -> Result<(), String> {
        // Initialize database (schema initialized automatically in new())
        let db_path = crate::paths::get_tokens_db_path();
        let db = TokenDatabase::new(&db_path.to_string_lossy())
            .map_err(|e| format!("Failed to create database: {}", e))?;

        let db_arc = Arc::new(db);

        // Initialize global database for decimals module and other components
        crate::tokens::database::init_global_database(db_arc.clone())
            .map_err(|e| format!("Failed to init global database: {}", e))?;

        self.db = Some(db_arc.clone());

        // Preload all known decimals into memory cache for synchronous pool decoder access
        // This is CRITICAL: pool decoders run synchronously and need decimals available in cache
        let preload_start = std::time::Instant::now();
        let all_decimals =
            tokio::task::spawn_blocking(move || db_arc.get_all_tokens_with_decimals())
                .await
                .map_err(|e| format!("Failed to spawn decimals preload task: {}", e))?
                .map_err(|e| format!("Failed to fetch decimals from database: {}", e))?;

        let mut preloaded_count = 0;
        for (mint, decimals) in all_decimals {
            if decimals > 0 {
                crate::tokens::decimals::cache(&mint, decimals);
                preloaded_count += 1;
            }
        }

        logger::info(
            LogTag::Tokens,
            &format!(
                "Preloaded {} token decimals into memory cache in {:.2}ms",
                preloaded_count,
                preload_start.elapsed().as_secs_f64() * 1000.0
            ),
        );

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
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let db = self.db.as_ref().ok_or("Database not initialized")?.clone();

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
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
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
        let cleanup_handle = cleanup::start_cleanup_loop(db.clone(), shutdown);
        handles.push(cleanup_handle);

        logger::info(
            LogTag::Tokens,
            &format!("Service started with {} background tasks", handles.len()),
        );
        
        // Mark tokens system ready after all update loops are started
        TOKENS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
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
            "dexscreener_cache_hit_rate".to_string(),
            dex_metrics.hit_rate(),
        );
        metrics
            .custom_metrics
            .insert("dexscreener_cache_entries".to_string(), dex_size as f64);
        metrics.custom_metrics.insert(
            "geckoterminal_cache_hit_rate".to_string(),
            gecko_metrics.hit_rate(),
        );
        metrics
            .custom_metrics
            .insert("geckoterminal_cache_entries".to_string(), gecko_size as f64);
        metrics.custom_metrics.insert(
            "rugcheck_cache_hit_rate".to_string(),
            rug_metrics.hit_rate(),
        );
        metrics
            .custom_metrics
            .insert("rugcheck_cache_entries".to_string(), rug_size as f64);
        metrics
    }
}
