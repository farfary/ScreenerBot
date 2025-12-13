//! Adaptive rate limiting strategies

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Exponential backoff calculator with jitter
pub struct ExponentialBackoff {
    /// Base delay
    base_delay: Duration,
    /// Maximum delay
    max_delay: Duration,
    /// Current attempt number
    attempt: AtomicU64,
    /// Jitter percentage (0.0 to 1.0)
    jitter: f64,
}

impl ExponentialBackoff {
    /// Create new exponential backoff
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            attempt: AtomicU64::new(0),
            jitter: 0.1, // 10% jitter by default
        }
    }

    /// Create with custom jitter
    pub fn with_jitter(base_delay: Duration, max_delay: Duration, jitter: f64) -> Self {
        Self {
            base_delay,
            max_delay,
            attempt: AtomicU64::new(0),
            jitter: jitter.clamp(0.0, 0.5),
        }
    }

    /// Get next delay duration
    pub fn next_delay(&self) -> Duration {
        let attempt = self.attempt.fetch_add(1, Ordering::SeqCst);
        self.calculate_delay(attempt)
    }

    /// Calculate delay for a specific attempt
    pub fn calculate_delay(&self, attempt: u64) -> Duration {
        // 2^attempt * base_delay
        let multiplier = 2u64.saturating_pow(attempt.min(10) as u32);
        let delay_ms = self.base_delay.as_millis() as u64 * multiplier;
        let delay = Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as u64));

        // Add jitter
        if self.jitter > 0.0 {
            let jitter_range = (delay.as_millis() as f64 * self.jitter) as u64;
            let jitter_value = rand_jitter(jitter_range);
            Duration::from_millis(delay.as_millis() as u64 + jitter_value)
        } else {
            delay
        }
    }

    /// Reset attempt counter
    pub fn reset(&self) {
        self.attempt.store(0, Ordering::SeqCst);
    }

    /// Current attempt number
    pub fn attempt(&self) -> u64 {
        self.attempt.load(Ordering::SeqCst)
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new(Duration::from_millis(100), Duration::from_secs(30))
    }
}

/// Simple jitter generation without full RNG dependency
fn rand_jitter(max: u64) -> u64 {
    if max == 0 {
        return 0;
    }
    // Use current time nanoseconds as simple randomness source
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    nanos % max
}

/// Sliding window rate tracker
///
/// Tracks request count within a sliding time window
pub struct SlidingWindowTracker {
    /// Window size
    window_size: Duration,
    /// Request timestamps (circular buffer)
    timestamps: std::sync::Mutex<Vec<Instant>>,
    /// Maximum capacity
    capacity: usize,
}

impl SlidingWindowTracker {
    /// Create new tracker
    pub fn new(window_size: Duration, capacity: usize) -> Self {
        Self {
            window_size,
            timestamps: std::sync::Mutex::new(Vec::with_capacity(capacity)),
            capacity,
        }
    }

    /// Record a request
    pub fn record(&self) {
        let now = Instant::now();
        let mut timestamps = self.timestamps.lock().unwrap();

        // Remove old entries
        let cutoff = now - self.window_size;
        timestamps.retain(|t| *t > cutoff);

        // Add new entry if not at capacity
        if timestamps.len() < self.capacity {
            timestamps.push(now);
        }
    }

    /// Get current request rate (requests per second)
    pub fn rate(&self) -> f64 {
        let now = Instant::now();
        let timestamps = self.timestamps.lock().unwrap();

        let cutoff = now - self.window_size;
        let count = timestamps.iter().filter(|t| **t > cutoff).count();

        count as f64 / self.window_size.as_secs_f64()
    }

    /// Get count in current window
    pub fn count(&self) -> usize {
        let now = Instant::now();
        let timestamps = self.timestamps.lock().unwrap();

        let cutoff = now - self.window_size;
        timestamps.iter().filter(|t| **t > cutoff).count()
    }

    /// Clear all timestamps
    pub fn clear(&self) {
        let mut timestamps = self.timestamps.lock().unwrap();
        timestamps.clear();
    }
}

impl Default for SlidingWindowTracker {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), 1000)
    }
}
