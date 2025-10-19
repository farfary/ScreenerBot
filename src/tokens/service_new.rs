/// Token service - ServiceManager integration
/// 
/// Orchestrates all token system background tasks:
/// - Database initialization
/// - Cache setup  
/// - Update loops (priority-based)
/// - Cleanup tasks
/// 
/// This service coordinates the new architecture with proper lifecycle management.

use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::cleanup;
use crate::tokens::database::TokenDatabase;
use crate::tokens::schema;
use crate::tokens::updates;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

const DB_PATH: &str = "data/tokens_new.db";

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
        let db = TokenDatabase::new(DB_PATH)
            .map_err(|e| format!("Failed to create database: {}", e))?;
        
        let db_arc = Arc::new(db);
        
        // Initialize global database for decimals module and other components
        crate::tokens::database::init_global_database(db_arc.clone())
            .map_err(|e| format!("Failed to init global database: {}", e))?;
        
        self.db = Some(db_arc);
        
        println!("[TOKENS_NEW] Service initialized with database at {}", DB_PATH);
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let db = self
            .db
            .as_ref()
            .ok_or("Database not initialized")?
            .clone();
        
        let mut handles = Vec::new();
        
        // Start update loops (critical, high, low priority + semaphore refill)
        let update_handles = updates::start_update_loop(db.clone(), shutdown.clone());
        for handle in update_handles {
            // Wrap each handle with monitor for metrics
            let monitored = monitor.instrument(handle);
            handles.push(tokio::spawn(monitored));
        }
        
        // Start cleanup loop (hourly)
        let cleanup_handle = cleanup::start_cleanup_loop(db.clone(), shutdown.clone());
        let monitored_cleanup = monitor.instrument(cleanup_handle);
        handles.push(tokio::spawn(monitored_cleanup));
        
        println!("[TOKENS_NEW] Service started with {} background tasks", handles.len());
        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        println!("[TOKENS_NEW] Service stopping...");
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
        let gecko_metrics = crate::tokens::market::geckoterminal::get_cache_metrics();
        let rug_metrics = crate::tokens::security::rugcheck::get_cache_metrics();
        
        let mut metrics = ServiceMetrics::default();
        metrics.custom_metrics.insert("dexscreener_cache".to_string(), dex_metrics);
        metrics.custom_metrics.insert("geckoterminal_cache".to_string(), gecko_metrics);
        metrics.custom_metrics.insert("rugcheck_cache".to_string(), rug_metrics);
        metrics
    }
}

/// Get the global database handle for external access
/// 
/// This allows other parts of the system to access the token database
/// without needing to pass it around everywhere.
pub fn get_global_database() -> Option<Arc<TokenDatabase>> {
    // This will be populated by the service on initialization
    // For now, we'll need to implement global state management
    None
}
