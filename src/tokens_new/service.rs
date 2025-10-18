// tokens_new/service.rs
// Service scaffold for tokens_new background tasks (not wired yet)

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::services::Service;
use tokio_metrics::TaskMonitor;
use tokio::sync::Notify;
use crate::tokens_new::provider::TokenDataProvider;
use crate::tokens_new::{discovery, pools, decimals};
use crate::tokens_new::blacklist as bl;
use crate::tokens_new::store;
use log::{info, warn, error};

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

    async fn initialize(&mut self) -> Result<(), String> {
        // Hydrate blacklist into memory for fast checks
        let db = self.provider.database();
        match bl::hydrate_from_db(&db) {
            Ok(count) => info!("[TOKENS_NEW] Blacklist hydrated: {} entries", count),
            Err(e) => warn!("[TOKENS_NEW] Blacklist hydrate failed: {}", e),
        }
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>, monitor: TaskMonitor) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let mut handles = Vec::new();

        // Discovery loop (every 20s)
        {
            let provider = self.provider.clone();
            let shutdown_c = shutdown.clone();
            let fut = async move {
                loop {
                    tokio::select! {
                        _ = shutdown_c.notified() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(20)) => {
                            match discovery::discover_from_sources(&provider).await {
                                Ok(mints) => {
                                    if !mints.is_empty() {
                                        info!("[TOKENS_NEW] Discovery found {} candidates", mints.len());
                                        discovery::process_new_mints(&provider, mints).await;
                                    }
                                }
                                Err(e) => warn!("[TOKENS_NEW] Discovery error: {}", e),
                            }
                        }
                    }
                }
            };
            handles.push(tokio::spawn(monitor.instrument(fut)));
        }

        // Pools refresh loop (priority-based TTL)
        {
            let provider = self.provider.clone();
            let shutdown_c = shutdown.clone();
            let fut = async move {
                loop {
                    tokio::select! {
                        _ = shutdown_c.notified() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                            let snapshots = store::all_snapshots();
                            for s in snapshots {
                                if bl::is(&s.mint) { continue; }
                                if let Err(e) = pools::refresh_for(&provider, &s.mint).await {
                                    warn!("[TOKENS_NEW] Pools refresh failed: mint={} err={}", s.mint, e);
                                }
                            }
                        }
                    }
                }
            };
            handles.push(tokio::spawn(monitor.instrument(fut)));
        }

        // Decimals ensure loop (lazy fill for known mints)
        {
            let provider = self.provider.clone();
            let shutdown_c = shutdown.clone();
            let fut = async move {
                loop {
                    tokio::select! {
                        _ = shutdown_c.notified() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                            let mints = store::list_mints();
                            for mint in mints {
                                if bl::is(&mint) { continue; }
                                if let Err(e) = decimals::ensure(&provider, &mint).await {
                                    warn!("[TOKENS_NEW] Decimals ensure failed: mint={} err={}", mint, e);
                                }
                            }
                        }
                    }
                }
            };
            handles.push(tokio::spawn(monitor.instrument(fut)));
        }

        Ok(handles)
    }
}
