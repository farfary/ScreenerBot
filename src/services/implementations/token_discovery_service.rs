use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct TokenDiscoveryService;

#[async_trait]
impl Service for TokenDiscoveryService {
    fn name(&self) -> &'static str {
        "token_discovery"
    }

    fn priority(&self) -> i32 {
        41
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing tokens system...");

        // Initialize tokens system (database, API clients, OHLCV, etc.)
        // This was previously in TokensService, but that service was empty/useless
        crate::tokens
            ::initialize_tokens_system().await
            .map_err(|e| format!("Failed to initialize tokens system: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ Tokens system initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting token discovery service...");

        // Start token discovery task
        let handle = crate::tokens::discovery
            ::start_token_discovery(shutdown).await
            .map_err(|e| format!("Failed to start token discovery: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ Token discovery service started");

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Token discovery service stopping (via shutdown signal)");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Token discovery is healthy if tokens system is ready
        if crate::global::TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded("Tokens system not yet ready".to_string())
        }
    }
}
