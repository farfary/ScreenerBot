use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::Mutex;
use anyhow::{ Context, Result };
use log::{ debug, warn, error };
use crate::config::DexScreenerConfig;

/// Centralized rate limiter for DexScreener API calls
/// Ensures we don't exceed 300 requests per minute across the entire application
#[derive(Debug)]
pub struct DexScreenerRateLimiter {
    last_request_time: Arc<Mutex<Option<Instant>>>,
    config: DexScreenerConfig,
    min_interval: Duration,
    burst_allowance: Arc<Mutex<u32>>,
    burst_reset_time: Arc<Mutex<Instant>>,
}

impl DexScreenerRateLimiter {
    pub fn new(config: DexScreenerConfig) -> Self {
        // Calculate minimum interval between requests
        let requests_per_second = (config.rate_limit_requests_per_minute as f64) / 60.0;
        let min_interval_ms = (1000.0 / requests_per_second) as u64;

        debug!(
            "DexScreener rate limiter initialized: {} req/min = {} ms between requests",
            config.rate_limit_requests_per_minute,
            min_interval_ms
        );

        Self {
            last_request_time: Arc::new(Mutex::new(None)),
            min_interval: Duration::from_millis(min_interval_ms),
            burst_allowance: Arc::new(Mutex::new(config.rate_limit_burst_size)),
            burst_reset_time: Arc::new(Mutex::new(Instant::now())),
            config,
        }
    }

    /// Wait if needed before making a request to respect rate limits
    pub async fn wait_if_needed(&self) -> Result<()> {
        let now = Instant::now();

        // Check if we can use burst allowance
        let mut burst_allowance = self.burst_allowance.lock().await;
        let mut burst_reset_time = self.burst_reset_time.lock().await;

        // Reset burst allowance every minute
        if now.duration_since(*burst_reset_time) >= Duration::from_secs(60) {
            *burst_allowance = self.config.rate_limit_burst_size;
            *burst_reset_time = now;
            debug!("Burst allowance reset to {}", self.config.rate_limit_burst_size);
        }

        // If we have burst allowance, use it
        if *burst_allowance > 0 {
            *burst_allowance -= 1;
            debug!("Using burst allowance, remaining: {}", *burst_allowance);

            let mut last_time = self.last_request_time.lock().await;
            *last_time = Some(now);
            return Ok(());
        }

        drop(burst_allowance);
        drop(burst_reset_time);

        // No burst allowance, follow normal rate limiting
        let mut last_time = self.last_request_time.lock().await;

        if let Some(last) = *last_time {
            let elapsed = now.duration_since(last);

            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                debug!("Rate limiting: waiting {:?} before next DexScreener API call", wait_time);

                drop(last_time); // Release the lock before sleeping
                tokio::time::sleep(wait_time).await;

                // Re-acquire the lock and update the time
                let mut last_time = self.last_request_time.lock().await;
                *last_time = Some(Instant::now());
            } else {
                *last_time = Some(now);
            }
        } else {
            *last_time = Some(now);
        }

        Ok(())
    }

    /// Handle 429 Too Many Requests error with exponential backoff
    pub async fn handle_rate_limit_error(&self, attempt: u32) -> Result<()> {
        if attempt >= self.config.retry_attempts {
            return Err(
                anyhow::anyhow!(
                    "Max retry attempts ({}) reached for DexScreener API",
                    self.config.retry_attempts
                )
            );
        }

        let base_delay = Duration::from_millis(self.config.retry_delay_ms);
        let delay = if self.config.retry_exponential_backoff {
            let exponential_delay = base_delay * (2_u32).pow(attempt);
            let max_delay = Duration::from_millis(self.config.max_retry_delay_ms);
            std::cmp::min(exponential_delay, max_delay)
        } else {
            base_delay
        };

        warn!(
            "DexScreener API rate limit hit (attempt {}), waiting {:?} before retry",
            attempt + 1,
            delay
        );

        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &DexScreenerConfig {
        &self.config
    }

    /// Get current rate limit status
    pub async fn get_status(&self) -> RateLimiterStatus {
        let last_time = self.last_request_time.lock().await;
        let burst_allowance = *self.burst_allowance.lock().await;
        let burst_reset_time = *self.burst_reset_time.lock().await;

        let time_since_last = last_time
            .map(|t| Instant::now().duration_since(t))
            .unwrap_or(Duration::from_secs(0));

        let time_until_burst_reset = if Instant::now() >= burst_reset_time {
            Duration::from_secs(0)
        } else {
            burst_reset_time.duration_since(Instant::now())
        };

        RateLimiterStatus {
            burst_allowance_remaining: burst_allowance,
            time_since_last_request: time_since_last,
            time_until_burst_reset,
            requests_per_minute: self.config.rate_limit_requests_per_minute,
        }
    }
}

#[derive(Debug)]
pub struct RateLimiterStatus {
    pub burst_allowance_remaining: u32,
    pub time_since_last_request: Duration,
    pub time_until_burst_reset: Duration,
    pub requests_per_minute: u32,
}

/// Global singleton instance of the DexScreener rate limiter
/// This ensures all DexScreener API calls across the application use the same rate limiter
static GLOBAL_RATE_LIMITER: tokio::sync::OnceCell<Arc<DexScreenerRateLimiter>> = tokio::sync::OnceCell::const_new();

/// Initialize the global DexScreener rate limiter
pub async fn init_dexscreener_rate_limiter(config: DexScreenerConfig) -> Result<()> {
    let limiter = Arc::new(DexScreenerRateLimiter::new(config));
    GLOBAL_RATE_LIMITER.set(limiter).map_err(|_| {
        anyhow::anyhow!("DexScreener rate limiter already initialized")
    })?;

    debug!("Global DexScreener rate limiter initialized successfully");
    Ok(())
}

/// Get the global DexScreener rate limiter instance
pub fn get_dexscreener_rate_limiter() -> Result<Arc<DexScreenerRateLimiter>> {
    GLOBAL_RATE_LIMITER.get()
        .ok_or_else(||
            anyhow::anyhow!(
                "DexScreener rate limiter not initialized. Call init_dexscreener_rate_limiter() first"
            )
        )
        .map(|limiter| limiter.clone())
}

/// Convenience function to wait for rate limit before making a DexScreener API call
pub async fn wait_for_dexscreener_rate_limit() -> Result<()> {
    let limiter = get_dexscreener_rate_limiter()?;
    limiter.wait_if_needed().await
}

/// Convenience function to handle 429 errors from DexScreener API
pub async fn handle_dexscreener_rate_limit_error(attempt: u32) -> Result<()> {
    let limiter = get_dexscreener_rate_limiter()?;
    limiter.handle_rate_limit_error(attempt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = DexScreenerConfig {
            rate_limit_requests_per_minute: 60, // 1 per second for easy testing
            rate_limit_burst_size: 2,
            ..Default::default()
        };

        let limiter = DexScreenerRateLimiter::new(config);

        // First request should be immediate
        let start = Instant::now();
        limiter.wait_if_needed().await.unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));

        // Second request should use burst
        limiter.wait_if_needed().await.unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));

        // Third request should wait
        let before_third = Instant::now();
        limiter.wait_if_needed().await.unwrap();
        assert!(before_third.elapsed() >= Duration::from_millis(900)); // Close to 1 second
    }
}
