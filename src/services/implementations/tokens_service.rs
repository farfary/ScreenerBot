use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

pub struct TokensService;

#[async_trait]
impl Service for TokensService {
    fn name(&self) -> &'static str {
        "tokens"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing tokens system...");

        // Initialize tokens system
        crate::tokens
            ::initialize_tokens_system().await
            .map_err(|e| format!("Failed to initialize tokens system: {}", e))?;

        log(LogTag::System, "SUCCESS", "Tokens system initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting tokens monitoring and discovery...");

        let shutdown_monitor = shutdown.clone();
        let monitor_handle = tokio::spawn(async move {
            crate::tokens::monitor::start_token_monitoring(shutdown_monitor).await;
        });

        let shutdown_discovery = shutdown.clone();
        let discovery_handle = crate::tokens::discovery
            ::start_token_discovery(shutdown_discovery).await
            .map_err(|e| format!("Failed to start token discovery: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ Tokens service started");

        Ok(vec![monitor_handle, discovery_handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
