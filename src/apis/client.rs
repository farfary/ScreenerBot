/// Base HTTP client with rate limiting
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

/// Rate limiter for API clients
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    last_request: Arc<Mutex<Option<Instant>>>,
    min_interval: Duration,
    max_per_minute: usize,
}

impl RateLimiter {
    pub fn new(max_per_minute: usize) -> Self {
        let min_interval = if max_per_minute > 0 {
            Duration::from_secs_f64(60.0 / max_per_minute as f64)
        } else {
            Duration::ZERO
        };

        Self {
            semaphore: Arc::new(Semaphore::new(1)), // Only 1 concurrent request
            last_request: Arc::new(Mutex::new(None)),
            min_interval,
            max_per_minute,
        }
    }

    /// Wait until we can make a request (respects rate limits)
    pub async fn acquire(&self) -> Result<RateLimitGuard, String> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Failed to acquire rate limiter permit: {}", e))?;

        if !self.min_interval.is_zero() {
            let mut last = self.last_request.lock().await;
            let now = Instant::now();

            if let Some(last_time) = *last {
                let elapsed = last_time.elapsed();
                if elapsed < self.min_interval {
                    let sleep_duration = self.min_interval - elapsed;
                    drop(last);
                    tokio::time::sleep(sleep_duration).await;
                    let mut last_relocked = self.last_request.lock().await;
                    *last_relocked = Some(Instant::now());
                } else {
                    *last = Some(now);
                }
            } else {
                *last = Some(now);
            }
        }

        Ok(RateLimitGuard { _permit: permit })
    }

    pub fn max_per_minute(&self) -> usize {
        self.max_per_minute
    }

    pub fn min_interval(&self) -> Duration {
        self.min_interval
    }
}

/// RAII guard returned by [`RateLimiter::acquire`]
pub struct RateLimitGuard {
    _permit: OwnedSemaphorePermit,
}

/// HTTP client wrapper with timeout and retry logic
pub struct HttpClient {
    client: Client,
    timeout: Duration,
}

impl HttpClient {
    pub fn new(timeout_secs: u64) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}
