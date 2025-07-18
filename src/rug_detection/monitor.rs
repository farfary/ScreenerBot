use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::sync::RwLock;
use anyhow::Result;
use log;

use crate::marketdata::MarketDatabase;
use crate::rug_detection::{ RugDetectionEngine, RugDetectionConfig, RugAction };

/// Real-time rug detection monitoring service
pub struct RugDetectionMonitor {
    database: Arc<MarketDatabase>,
    rug_engine: Arc<RugDetectionEngine>,
    config: RugDetectionConfig,
    is_running: Arc<RwLock<bool>>,
    monitoring_stats: Arc<RwLock<MonitoringStats>>,
}

#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub tokens_scanned: u64,
    pub rugs_detected: u64,
    pub tokens_blacklisted: u64,
    pub last_scan_duration_ms: u64,
    pub scan_cycles_completed: u64,
    pub last_scan_time: chrono::DateTime<chrono::Utc>,
}

impl Default for MonitoringStats {
    fn default() -> Self {
        Self {
            tokens_scanned: 0,
            rugs_detected: 0,
            tokens_blacklisted: 0,
            last_scan_duration_ms: 0,
            scan_cycles_completed: 0,
            last_scan_time: chrono::Utc::now(),
        }
    }
}

impl RugDetectionMonitor {
    /// Create new rug detection monitor
    pub fn new(
        database: Arc<MarketDatabase>,
        rug_engine: Arc<RugDetectionEngine>,
        config: RugDetectionConfig
    ) -> Self {
        Self {
            database,
            rug_engine,
            config,
            is_running: Arc::new(RwLock::new(false)),
            monitoring_stats: Arc::new(RwLock::new(MonitoringStats::default())),
        }
    }

    /// Start the monitoring service
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            log::warn!("Rug detection monitor is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        log::info!("🚨 Starting rug detection monitor...");

        // Start background monitoring loop
        let monitor = self.clone();
        tokio::spawn(async move {
            monitor.run_monitoring_loop().await;
        });

        Ok(())
    }

    /// Stop the monitoring service
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        log::info!("🔻 Rug detection monitor stopped");
    }

    /// Check if monitoring is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get monitoring statistics
    pub async fn get_stats(&self) -> MonitoringStats {
        self.monitoring_stats.read().await.clone()
    }

    /// Main monitoring loop
    async fn run_monitoring_loop(&self) {
        // Stagger initial scan to avoid startup conflicts
        tokio::time::sleep(Duration::from_secs(10)).await;

        let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes between scans

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            if let Err(e) = self.run_rug_detection_scan().await {
                log::error!("Rug detection scan failed: {}", e);
                // Continue monitoring despite errors
                continue;
            }
        }
    }

    /// Perform comprehensive rug detection scan
    async fn run_rug_detection_scan(&self) -> Result<()> {
        let scan_start = std::time::Instant::now();

        log::info!("🔍 Starting rug detection scan cycle...");

        // Get all active tokens from database
        let active_tokens = self.database.get_active_tokens()?;
        let total_tokens = active_tokens.len();

        if total_tokens == 0 {
            log::debug!("No active tokens to scan for rug detection");
            return Ok(());
        }

        log::info!("📊 Scanning {} active tokens for rug indicators", total_tokens);

        let mut tokens_scanned = 0u64;
        let mut rugs_detected = 0u64;
        let mut tokens_blacklisted = 0u64;

        // Process tokens in batches to avoid overwhelming APIs
        for chunk in active_tokens.chunks(10) {
            for token_address in chunk {
                // Check if still running
                if !*self.is_running.read().await {
                    log::info!("Rug detection scan interrupted by shutdown");
                    return Ok(());
                }

                match self.scan_token_for_rug(token_address).await {
                    Ok(rug_detected) => {
                        tokens_scanned += 1;
                        if rug_detected {
                            rugs_detected += 1;
                            tokens_blacklisted += 1;
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to scan token {}: {}", token_address, e);
                        // Continue with other tokens
                    }
                }
            }

            // Rate limiting: small delay between batches
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let scan_duration = scan_start.elapsed();

        // Update monitoring statistics
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.tokens_scanned += tokens_scanned;
            stats.rugs_detected += rugs_detected;
            stats.tokens_blacklisted += tokens_blacklisted;
            stats.last_scan_duration_ms = scan_duration.as_millis() as u64;
            stats.scan_cycles_completed += 1;
            stats.last_scan_time = chrono::Utc::now();
        }

        log::info!(
            "✅ Rug detection scan completed: {}/{} tokens scanned, {} rugs detected, {} blacklisted ({}ms)",
            tokens_scanned,
            total_tokens,
            rugs_detected,
            tokens_blacklisted,
            scan_duration.as_millis()
        );

        Ok(())
    }

    /// Scan individual token for rug indicators
    async fn scan_token_for_rug(&self, token_address: &str) -> Result<bool> {
        // Get current token data including liquidity
        let token_data = match self.database.get_token(token_address)? {
            Some(data) => data,
            None => {
                log::debug!("No data available for token {}", token_address);
                return Ok(false);
            }
        };

        // Skip if already blacklisted
        if self.database.is_blacklisted(token_address)? {
            return Ok(false);
        }

        // Perform rug detection analysis
        let result = self.rug_engine.analyze_token(token_address, token_data.liquidity_sol).await?;

        match result.recommended_action {
            RugAction::Blacklist | RugAction::SellImmediately => {
                log::warn!(
                    "🚨 RUG DETECTED: {} - Confidence: {:.1}% - Reasons: {:?}",
                    token_address,
                    result.confidence * 100.0,
                    result.reasons
                );

                // Auto-blacklist if enabled
                if self.config.auto_blacklist {
                    use crate::marketdata::TokenBlacklist;

                    let blacklist_entry = TokenBlacklist {
                        token_address: token_address.to_string(),
                        reason: format!("Auto-detected rug: {:?}", result.reasons),
                        blacklisted_at: chrono::Utc::now(),
                        peak_liquidity: None, // Could be enhanced to track peak
                        final_liquidity: Some(token_data.liquidity_sol),
                        drop_percentage: None, // Could be calculated if we have peak
                    };

                    self.database.add_to_blacklist(&blacklist_entry)?;
                    log::info!("🚫 Auto-blacklisted token: {}", token_address);
                }

                return Ok(true);
            }
            RugAction::Monitor => {
                log::warn!(
                    "⚠️  MONITORING: {} - Confidence: {:.1}% - Reasons: {:?}",
                    token_address,
                    result.confidence * 100.0,
                    result.reasons
                );
                return Ok(false);
            }
            RugAction::Continue => {
                log::debug!("Token {} passed rug detection scan", token_address);
                return Ok(false);
            }
        }
    }
}

impl Clone for RugDetectionMonitor {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            rug_engine: self.rug_engine.clone(),
            config: self.config.clone(),
            is_running: self.is_running.clone(),
            monitoring_stats: self.monitoring_stats.clone(),
        }
    }
}
