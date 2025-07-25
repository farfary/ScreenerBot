/// Token monitoring system for periodic price updates with rate limiting
use crate::logger::{ log, LogTag };
use crate::tokens::api::DexScreenerApi;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::blacklist::{ check_and_track_liquidity, is_token_blacklisted };
use crate::tokens::types::*;
use tokio::time::{ sleep, Duration };
use tokio::sync::Semaphore;
use std::sync::Arc;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Rate limit for DexScreener info API (per minute)
pub const INFO_RATE_LIMIT: usize = 200;

/// API calls to use per monitoring cycle (50% of rate limit)
pub const INFO_CALLS_PER_CYCLE: usize = 100;

/// Monitoring cycle duration in minutes
pub const CYCLE_DURATION_MINUTES: u64 = 1;

/// Maximum tokens to process per API call
pub const MAX_TOKENS_PER_BATCH: usize = 30;

// =============================================================================
// TOKEN MONITOR
// =============================================================================

pub struct TokenMonitor {
    api: DexScreenerApi,
    database: TokenDatabase,
    rate_limiter: Arc<Semaphore>,
    current_cycle: usize,
}

impl TokenMonitor {
    /// Create new token monitor instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api = DexScreenerApi::new();
        let database = TokenDatabase::new()?;
        let rate_limiter = Arc::new(Semaphore::new(INFO_CALLS_PER_CYCLE));

        Ok(Self {
            api,
            database,
            rate_limiter,
            current_cycle: 0,
        })
    }

    /// Start continuous token monitoring loop
    pub async fn start_monitoring_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::System, "START", "Token monitor started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Token monitor stopping");
                    break;
                }
                
                _ = sleep(Duration::from_secs(CYCLE_DURATION_MINUTES * 60)) => {
                    self.current_cycle += 1;
                    
                    log(LogTag::System, "MONITOR", 
                        &format!("Starting monitoring cycle #{}", self.current_cycle));
                    
                    if let Err(e) = self.monitor_tokens().await {
                        log(LogTag::System, "ERROR", 
                            &format!("Monitoring cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::System, "STOP", "Token monitor stopped");
    }

    /// Monitor and update token prices
    async fn monitor_tokens(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get all tokens from database
        let all_tokens = self.database.get_all_tokens().await?;

        if all_tokens.is_empty() {
            log(LogTag::System, "MONITOR", "No tokens in database to monitor");
            return Ok(());
        }

        // Filter out blacklisted tokens
        let mut tokens_to_check: Vec<ApiToken> = all_tokens
            .into_iter()
            .filter(|token| !is_token_blacklisted(&token.mint))
            .collect();

        if tokens_to_check.is_empty() {
            log(LogTag::System, "MONITOR", "No non-blacklisted tokens to monitor");
            return Ok(());
        }

        // Sort by liquidity (highest first) for prioritization
        tokens_to_check.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Implement 50/50 split: high liquidity vs others
        let total_tokens = tokens_to_check.len();
        let high_liquidity_count = std::cmp::min(INFO_CALLS_PER_CYCLE / 2, total_tokens);
        let remaining_calls = INFO_CALLS_PER_CYCLE - high_liquidity_count;

        let high_liquidity_tokens = tokens_to_check[..high_liquidity_count].to_vec();
        let other_tokens: Vec<ApiToken> = tokens_to_check[high_liquidity_count..]
            .iter()
            .take(remaining_calls)
            .cloned()
            .collect();

        // Combine priority tokens (high liquidity first, then others)
        let priority_tokens: Vec<ApiToken> = high_liquidity_tokens
            .into_iter()
            .chain(other_tokens.into_iter())
            .collect();

        log(
            LogTag::System,
            "MONITOR",
            &format!(
                "Monitoring {} tokens (from {} total, {} blacklisted)",
                priority_tokens.len(),
                total_tokens,
                total_tokens - tokens_to_check.len()
            )
        );

        // Process tokens in batches
        self.process_tokens_in_batches(priority_tokens).await?;

        log(
            LogTag::System,
            "MONITOR",
            &format!("Completed monitoring cycle #{}", self.current_cycle)
        );

        Ok(())
    }

    /// Process tokens in batches with rate limiting
    async fn process_tokens_in_batches(
        &mut self,
        tokens: Vec<ApiToken>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut processed = 0;
        let mut updated = 0;
        let mut errors = 0;

        for chunk in tokens.chunks(MAX_TOKENS_PER_BATCH) {
            // Acquire rate limit permit
            let _permit = self.rate_limiter.acquire().await.unwrap();

            let mints: Vec<String> = chunk
                .iter()
                .map(|t| t.mint.clone())
                .collect();

            match self.api.get_tokens_info(&mints).await {
                Ok(updated_tokens) => {
                    // Update database with new token information
                    if !updated_tokens.is_empty() {
                        match self.database.update_tokens(&updated_tokens).await {
                            Ok(_) => {
                                // Track liquidity for blacklisting
                                for token in &updated_tokens {
                                    if let Some(liquidity) = &token.liquidity {
                                        if let Some(usd_liquidity) = liquidity.usd {
                                            let age_hours = self.calculate_token_age(&token);
                                            check_and_track_liquidity(
                                                &token.mint,
                                                &token.symbol,
                                                usd_liquidity,
                                                age_hours
                                            );
                                        }
                                    }
                                }

                                updated += updated_tokens.len();
                                log(
                                    LogTag::System,
                                    "UPDATE",
                                    &format!("Updated {} tokens", updated_tokens.len())
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::System,
                                    "ERROR",
                                    &format!("Failed to update tokens in database: {}", e)
                                );
                                errors += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to fetch token info for batch: {}", e)
                    );
                    errors += 1;
                }
            }

            processed += chunk.len();

            // Rate limiting delay between batches
            if processed < tokens.len() {
                sleep(Duration::from_millis(500)).await;
            }
        }

        log(
            LogTag::System,
            "STATS",
            &format!("Processed: {}, Updated: {}, Errors: {}", processed, updated, errors)
        );

        Ok(())
    }

    /// Calculate token age in hours
    fn calculate_token_age(&self, token: &ApiToken) -> i64 {
        if let Some(created_at) = token.pair_created_at {
            let now = chrono::Utc::now().timestamp();
            let age_seconds = now - created_at;
            age_seconds / 3600 // Convert to hours
        } else {
            // If no creation time, assume very old
            24 * 7 // 7 days
        }
    }
}

// =============================================================================
// MONITOR HELPER FUNCTIONS
// =============================================================================

/// Start token monitoring in background task
pub async fn start_token_monitoring(
    shutdown: Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let mut monitor = TokenMonitor::new()?;

    let handle = tokio::spawn(async move {
        monitor.start_monitoring_loop(shutdown).await;
    });

    Ok(handle)
}

/// Manual token monitoring trigger (for testing)
pub async fn monitor_tokens_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = TokenMonitor::new()?;
    monitor.monitor_tokens().await
}

/// Get monitoring statistics
pub async fn get_monitoring_stats() -> Result<MonitoringStats, Box<dyn std::error::Error>> {
    let database = TokenDatabase::new()?;
    let total_tokens = database.get_all_tokens().await?.len();

    let blacklisted_count = if let Some(stats) = crate::tokens::blacklist::get_blacklist_stats() {
        stats.total_blacklisted
    } else {
        0
    };

    Ok(MonitoringStats {
        total_tokens,
        blacklisted_count,
        active_tokens: total_tokens - blacklisted_count,
        last_cycle: chrono::Utc::now(),
    })
}

/// Monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub total_tokens: usize,
    pub blacklisted_count: usize,
    pub active_tokens: usize,
    pub last_cycle: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_monitor_creation() {
        let monitor = TokenMonitor::new();
        assert!(monitor.is_ok());
    }

    #[tokio::test]
    async fn test_manual_monitoring() {
        let result = monitor_tokens_once().await;
        // Should not fail even if no tokens to monitor
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_stats() {
        let result = get_monitoring_stats().await;
        assert!(result.is_ok());
    }
}
