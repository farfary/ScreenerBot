//! Per-provider rate limiter using Governor (GCRA algorithm)

use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorLimiter,
};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::rpc::types::RpcMethod;

/// Per-provider rate limiter with adaptive backoff
pub struct ProviderRateLimiter {
    /// Provider identifier
    provider_id: String,

    /// Governor rate limiter (GCRA - Generic Cell Rate Algorithm)
    limiter: GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>,

    /// Base rate limit (calls per second)
    base_rate: u32,

    /// Current effective rate (may be reduced due to 429s)
    current_rate: AtomicU32,

    /// Consecutive 429 errors
    consecutive_429s: AtomicU32,

    /// Last 429 timestamp
    last_429: RwLock<Option<Instant>>,

    /// Backoff multiplier (0.5 = halve rate on each 429)
    backoff_multiplier: f64,

    /// Minimum rate limit
    min_rate: u32,

    /// Recovery rate (increase per success after 429 recovery)
    recovery_rate: f64,

    /// Successes needed for full recovery
    recovery_threshold: u32,

    /// Current recovery progress
    recovery_progress: AtomicU32,
}

impl ProviderRateLimiter {
    /// Create new rate limiter
    pub fn new(provider_id: &str, rate_per_second: u32) -> Self {
        let rate = rate_per_second.max(1);
        let quota = Quota::per_second(NonZeroU32::new(rate).unwrap());

        Self {
            provider_id: provider_id.to_string(),
            limiter: GovernorLimiter::direct(quota),
            base_rate: rate,
            current_rate: AtomicU32::new(rate),
            consecutive_429s: AtomicU32::new(0),
            last_429: RwLock::new(None),
            backoff_multiplier: 0.5,
            min_rate: 1,
            recovery_rate: 0.1, // 10% recovery per success
            recovery_threshold: 10,
            recovery_progress: AtomicU32::new(0),
        }
    }

    /// Create with custom backoff settings
    pub fn with_backoff(
        provider_id: &str,
        rate_per_second: u32,
        backoff_multiplier: f64,
        min_rate: u32,
    ) -> Self {
        let mut limiter = Self::new(provider_id, rate_per_second);
        limiter.backoff_multiplier = backoff_multiplier.clamp(0.1, 0.9);
        limiter.min_rate = min_rate.max(1);
        limiter
    }

    /// Wait until rate limit allows a request
    ///
    /// Takes method cost into account (some methods consume more quota)
    pub async fn acquire(&self, method: &RpcMethod) {
        let cost = method.cost();

        // For higher-cost methods, wait multiple times
        for _ in 0..cost {
            self.limiter.until_ready().await;
        }
    }

    /// Try to acquire without blocking
    ///
    /// Returns true if request can proceed, false if rate limited
    pub fn try_acquire(&self, method: &RpcMethod) -> bool {
        let cost = method.cost();

        for _ in 0..cost {
            if self.limiter.check().is_err() {
                return false;
            }
        }
        true
    }

    /// Record a 429 rate limit error
    ///
    /// Reduces effective rate and tracks for recovery
    pub async fn record_429(&self, retry_after: Option<Duration>) {
        let count = self.consecutive_429s.fetch_add(1, Ordering::SeqCst) + 1;

        // Update last 429 timestamp
        {
            let mut last = self.last_429.write().await;
            *last = Some(Instant::now());
        }

        // Calculate new rate with exponential backoff
        let reduction = self.backoff_multiplier.powi(count as i32);
        let new_rate =
            ((self.base_rate as f64) * reduction).max(self.min_rate as f64) as u32;

        self.current_rate.store(new_rate, Ordering::SeqCst);

        // Reset recovery progress
        self.recovery_progress.store(0, Ordering::SeqCst);

        // If retry_after is provided, we could sleep here
        // But typically the caller handles this
        if let Some(delay) = retry_after {
            if delay > Duration::from_millis(100) && delay < Duration::from_secs(60) {
                tokio::time::sleep(delay).await;
            }
        }
    }

    /// Record a successful request
    ///
    /// Gradually recovers rate limit after 429s
    pub fn record_success(&self) {
        let consecutive = self.consecutive_429s.load(Ordering::SeqCst);

        if consecutive > 0 {
            // In recovery mode
            let progress = self.recovery_progress.fetch_add(1, Ordering::SeqCst) + 1;

            if progress >= self.recovery_threshold {
                // Full recovery - reset everything
                self.consecutive_429s.store(0, Ordering::SeqCst);
                self.current_rate.store(self.base_rate, Ordering::SeqCst);
                self.recovery_progress.store(0, Ordering::SeqCst);
            } else {
                // Partial recovery - gradually increase rate
                let current = self.current_rate.load(Ordering::SeqCst);
                let recovery_amount =
                    ((self.base_rate - current) as f64 * self.recovery_rate) as u32;
                let new_rate = (current + recovery_amount).min(self.base_rate);
                self.current_rate.store(new_rate, Ordering::SeqCst);
            }
        }
    }

    /// Get current effective rate limit
    pub fn current_rate(&self) -> u32 {
        self.current_rate.load(Ordering::SeqCst)
    }

    /// Get base rate limit
    pub fn base_rate(&self) -> u32 {
        self.base_rate
    }

    /// Whether rate limiter is in backoff mode
    pub fn is_backing_off(&self) -> bool {
        self.consecutive_429s.load(Ordering::SeqCst) > 0
    }

    /// Get provider ID
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Time since last 429 error
    pub async fn time_since_last_429(&self) -> Option<Duration> {
        let last = self.last_429.read().await;
        last.map(|t| t.elapsed())
    }

    /// Reset rate limiter to base rate
    pub fn reset(&self) {
        self.consecutive_429s.store(0, Ordering::SeqCst);
        self.current_rate.store(self.base_rate, Ordering::SeqCst);
        self.recovery_progress.store(0, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for ProviderRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRateLimiter")
            .field("provider_id", &self.provider_id)
            .field("base_rate", &self.base_rate)
            .field("current_rate", &self.current_rate.load(Ordering::SeqCst))
            .field(
                "consecutive_429s",
                &self.consecutive_429s.load(Ordering::SeqCst),
            )
            .field("is_backing_off", &self.is_backing_off())
            .finish()
    }
}
