/// Token service - ServiceManager integration
///
/// Orchestrates all token system background tasks:
/// - Database initialization
/// - Cache setup  
/// - Update loops (priority-based)
/// - Cleanup tasks
///
/// This service coordinates the new architecture with proper lifecycle management.
use crate::global::{TOKENS_DATABASE, TOKENS_SYSTEM_READY};
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
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
        let db = TokenDatabase::new(TOKENS_DATABASE)
            .map_err(|e| format!("Failed to create database: {}", e))?;

        let db_arc = Arc::new(db);

        // Initialize global database for decimals module and other components
        crate::tokens::database::init_global_database(db_arc.clone())
            .map_err(|e| format!("Failed to init global database: {}", e))?;

        self.db = Some(db_arc);

        logger::info(
            LogTag::Tokens,
            &format!("Service initialized with database at {}", TOKENS_DATABASE),
        );
        // Mark tokens system ready after successful initialization
        TOKENS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let db = self.db.as_ref().ok_or("Database not initialized")?.clone();

        let _ = monitor; // Metrics instrumentation will be wired up in a follow-up

        // Create a single shared rate limit coordinator for all token tasks
        let coordinator = Arc::new(RateLimitCoordinator::new());

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
        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(LogTag::Tokens, "Service stopping...");
        // On stop, mark as not ready
        TOKENS_SYSTEM_READY.store(false, std::sync::atomic::Ordering::SeqCst);
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
