use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

pub struct SecurityService;

#[async_trait]
impl Service for SecurityService {
    fn name(&self) -> &'static str {
        "security"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing security analyzer...");

        // Initialize security analyzer (not async)
        crate::tokens::security
            ::initialize_security_analyzer()
            .map_err(|e| format!("Failed to initialize security analyzer: {}", e))?;

        log(LogTag::System, "SUCCESS", "Security analyzer initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting security monitoring...");

        let handle = tokio::spawn(async move {
            crate::tokens::security::start_security_monitoring(shutdown).await;
        });

        log(LogTag::System, "SUCCESS", "✅ Security service started");

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::SECURITY_ANALYZER_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
