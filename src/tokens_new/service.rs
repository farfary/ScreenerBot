// tokens_new/service.rs
// Service scaffold for tokens_new background tasks (not wired yet)

use std::sync::Arc;

use chrono::Utc;

use crate::services::Service;
use tokio_metrics::TaskMonitor;
use tokio::sync::Notify;
use crate::tokens_new::provider::TokenDataProvider;

pub struct TokensNewService {
    provider: Arc<TokenDataProvider>,
}

impl TokensNewService {
    pub async fn new() -> Result<Self, String> {
        let provider = Arc::new(TokenDataProvider::new().await?);
        Ok(Self { provider })
    }
}

#[async_trait::async_trait]
impl Service for TokensNewService {
    fn name(&self) -> &'static str { "tokens_new_service" }
    fn priority(&self) -> i32 { 50 }
    fn dependencies(&self) -> Vec<&'static str> { vec!["rpc_service", "pool_service"] }

    async fn start(&mut self, _shutdown: Arc<Notify>, monitor: TaskMonitor) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let provider = self.provider.clone();
        let fut = monitor.instrument(async move {
            // Placeholder loop; real scheduler will go here
            let _ = provider.token_exists("So11111111111111111111111111111111111111112");
        });
        // Spawn one placeholder task to satisfy return type
        let handle = tokio::spawn(fut);
        Ok(vec![handle])
    }
}
