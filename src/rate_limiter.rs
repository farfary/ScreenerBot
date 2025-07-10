use std::collections::VecDeque;
use std::time::{ Duration, Instant };
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

/// Rate limiter to prevent 429 errors from external APIs
pub struct RateLimiter {
    requests: Mutex<VecDeque<Instant>>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            requests: Mutex::new(VecDeque::new()),
            max_requests,
            window_duration,
        }
    }

    /// Wait until it's safe to make a request, respecting rate limits
    pub async fn wait_for_request(&self) {
        loop {
            let mut requests = self.requests.lock().await;
            let now = Instant::now();

            // Remove old requests outside the window
            while let Some(&front) = requests.front() {
                if now.duration_since(front) <= self.window_duration {
                    break;
                }
                requests.pop_front();
            }

            // If we've hit the limit, wait until the oldest request expires
            if requests.len() >= self.max_requests {
                if let Some(&oldest) = requests.front() {
                    let wait_time = self.window_duration.saturating_sub(now.duration_since(oldest));
                    if !wait_time.is_zero() {
                        drop(requests); // Release the lock before sleeping
                        println!(
                            "⏳ [RATE LIMIT] Waiting {:.1}s to respect API limits",
                            wait_time.as_secs_f64()
                        );
                        tokio::time::sleep(wait_time).await;
                        continue; // Retry after waiting
                    }
                }
            }

            // Record this request
            requests.push_back(now);
            break;
        }
    }
}

// Global rate limiters for different APIs
pub static DEXSCREENER_LIMITER: Lazy<RateLimiter> = Lazy::new(|| {
    // DexScreener: 300 requests per minute (conservative)
    RateLimiter::new(300, Duration::from_secs(60))
});

pub static GECKOTERMINAL_LIMITER: Lazy<RateLimiter> = Lazy::new(|| {
    // GeckoTerminal: 30 requests per minute (conservative, they don't specify)
    RateLimiter::new(30, Duration::from_secs(60))
});

pub static RUGCHECK_LIMITER: Lazy<RateLimiter> = Lazy::new(|| {
    // RugCheck: 60 requests per minute (conservative)
    RateLimiter::new(60, Duration::from_secs(60))
});

/// Trait to add rate limiting to HTTP clients
pub trait RateLimitedRequest {
    async fn get_with_rate_limit(
        &self,
        url: &str,
        limiter: &RateLimiter
    ) -> reqwest::Result<reqwest::Response>;
}

impl RateLimitedRequest for reqwest::Client {
    async fn get_with_rate_limit(
        &self,
        url: &str,
        limiter: &RateLimiter
    ) -> reqwest::Result<reqwest::Response> {
        limiter.wait_for_request().await;
        self.get(url).send().await
    }
}
