//! Circuit breaker state machine

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::rpc::types::CircuitState;

use super::config::CircuitBreakerConfig;

/// Individual circuit breaker for a provider
pub struct ProviderCircuitBreaker {
    /// Provider identifier
    provider_id: String,

    /// Current circuit state
    state: RwLock<CircuitState>,

    /// Configuration
    config: CircuitBreakerConfig,

    /// Consecutive failure count
    failures: AtomicU32,

    /// Consecutive success count (in half-open state)
    successes: AtomicU32,

    /// When circuit was opened
    opened_at: RwLock<Option<Instant>>,

    /// When last state change occurred
    last_state_change: RwLock<Instant>,

    /// Total times circuit has opened
    total_opens: AtomicU64,

    /// Last error that caused a failure
    last_error: RwLock<Option<String>>,

    /// Probe requests allowed in half-open
    half_open_requests: AtomicU32,
}

impl ProviderCircuitBreaker {
    /// Create new circuit breaker
    pub fn new(provider_id: &str, config: CircuitBreakerConfig) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            state: RwLock::new(CircuitState::Closed),
            config,
            failures: AtomicU32::new(0),
            successes: AtomicU32::new(0),
            opened_at: RwLock::new(None),
            last_state_change: RwLock::new(Instant::now()),
            total_opens: AtomicU64::new(0),
            last_error: RwLock::new(None),
            half_open_requests: AtomicU32::new(0),
        }
    }

    /// Create with default config
    pub fn with_defaults(provider_id: &str) -> Self {
        Self::new(provider_id, CircuitBreakerConfig::default())
    }

    /// Check if circuit allows execution
    ///
    /// Returns Ok(()) if allowed, Err(Duration) with time until retry if blocked
    pub async fn can_execute(&self) -> Result<(), Duration> {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => Ok(()),

            CircuitState::HalfOpen => {
                // Allow limited probe requests
                let current = self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                if current < self.config.half_open_max_requests {
                    Ok(())
                } else {
                    // Too many probe requests, wait
                    Err(Duration::from_millis(100))
                }
            }

            CircuitState::Open => {
                // Check if we should transition to half-open
                let opened_at = self.opened_at.read().await;
                if let Some(time) = *opened_at {
                    let elapsed = time.elapsed();
                    if elapsed >= self.config.open_duration {
                        // Transition to half-open
                        drop(opened_at);
                        self.transition_to_half_open().await;
                        Ok(())
                    } else {
                        // Still in cooldown
                        Err(self.config.open_duration - elapsed)
                    }
                } else {
                    // No opened_at set, allow (shouldn't happen)
                    Ok(())
                }
            }
        }
    }

    /// Record a successful execution
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failures.store(0, Ordering::SeqCst);
            }

            CircuitState::HalfOpen => {
                let successes = self.successes.fetch_add(1, Ordering::SeqCst) + 1;

                if successes >= self.config.success_threshold {
                    // Recovery confirmed - close circuit
                    *state = CircuitState::Closed;
                    self.failures.store(0, Ordering::SeqCst);
                    self.successes.store(0, Ordering::SeqCst);
                    self.half_open_requests.store(0, Ordering::SeqCst);
                    *self.opened_at.write().await = None;
                    *self.last_state_change.write().await = Instant::now();
                    *self.last_error.write().await = None;
                }
            }

            CircuitState::Open => {
                // Success in open state shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed execution
    pub async fn record_failure(&self, error: &str, is_rate_limit: bool) {
        // Optionally ignore rate limits
        if is_rate_limit && self.config.ignore_rate_limits {
            return;
        }

        // Store last error
        *self.last_error.write().await = Some(error.to_string());

        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;

                if failures >= self.config.failure_threshold {
                    // Check min state duration
                    let last_change = self.last_state_change.read().await;
                    if last_change.elapsed() >= self.config.min_state_duration {
                        drop(last_change);
                        // Trip the circuit
                        *state = CircuitState::Open;
                        *self.opened_at.write().await = Some(Instant::now());
                        *self.last_state_change.write().await = Instant::now();
                        self.total_opens.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }

            CircuitState::HalfOpen => {
                // Probe failed - reopen circuit
                *state = CircuitState::Open;
                *self.opened_at.write().await = Some(Instant::now());
                *self.last_state_change.write().await = Instant::now();
                self.successes.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
                self.total_opens.fetch_add(1, Ordering::SeqCst);
            }

            CircuitState::Open => {
                // Already open, just update opened_at to extend timeout
                *self.opened_at.write().await = Some(Instant::now());
            }
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        if *state == CircuitState::Open {
            *state = CircuitState::HalfOpen;
            self.successes.store(0, Ordering::SeqCst);
            self.half_open_requests.store(0, Ordering::SeqCst);
            *self.last_state_change.write().await = Instant::now();
        }
    }

    /// Force circuit to open state
    pub async fn force_open(&self, reason: &str) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        *self.opened_at.write().await = Some(Instant::now());
        *self.last_state_change.write().await = Instant::now();
        *self.last_error.write().await = Some(reason.to_string());
        self.total_opens.fetch_add(1, Ordering::SeqCst);
    }

    /// Force circuit to closed state (reset)
    pub async fn force_close(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.failures.store(0, Ordering::SeqCst);
        self.successes.store(0, Ordering::SeqCst);
        self.half_open_requests.store(0, Ordering::SeqCst);
        *self.opened_at.write().await = None;
        *self.last_state_change.write().await = Instant::now();
        *self.last_error.write().await = None;
    }

    /// Get current state
    pub async fn current_state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get provider ID
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Get failure count
    pub fn failure_count(&self) -> u32 {
        self.failures.load(Ordering::SeqCst)
    }

    /// Get success count (in half-open)
    pub fn success_count(&self) -> u32 {
        self.successes.load(Ordering::SeqCst)
    }

    /// Get total times circuit has opened
    pub fn total_opens(&self) -> u64 {
        self.total_opens.load(Ordering::SeqCst)
    }

    /// Get last error
    pub async fn last_error(&self) -> Option<String> {
        self.last_error.read().await.clone()
    }

    /// Time until circuit might transition
    pub async fn time_until_transition(&self) -> Option<Duration> {
        let state = *self.state.read().await;
        match state {
            CircuitState::Open => {
                let opened_at = self.opened_at.read().await;
                opened_at.map(|t| {
                    let elapsed = t.elapsed();
                    if elapsed < self.config.open_duration {
                        self.config.open_duration - elapsed
                    } else {
                        Duration::ZERO
                    }
                })
            }
            _ => None,
        }
    }

    /// Get circuit breaker status
    pub async fn status(&self) -> CircuitBreakerStatus {
        let state = *self.state.read().await;
        CircuitBreakerStatus {
            provider_id: self.provider_id.clone(),
            state,
            failure_count: self.failures.load(Ordering::SeqCst),
            success_count: self.successes.load(Ordering::SeqCst),
            total_opens: self.total_opens.load(Ordering::SeqCst),
            last_error: self.last_error.read().await.clone(),
            time_until_transition: self.time_until_transition().await,
        }
    }
}

impl std::fmt::Debug for ProviderCircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCircuitBreaker")
            .field("provider_id", &self.provider_id)
            .field("failures", &self.failures.load(Ordering::SeqCst))
            .field("successes", &self.successes.load(Ordering::SeqCst))
            .field("total_opens", &self.total_opens.load(Ordering::SeqCst))
            .finish()
    }
}

/// Status of a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStatus {
    pub provider_id: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub total_opens: u64,
    pub last_error: Option<String>,
    pub time_until_transition: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = ProviderCircuitBreaker::with_defaults("test");
        assert!(cb.can_execute().await.is_ok());
        assert_eq!(cb.current_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            min_state_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let cb = ProviderCircuitBreaker::new("test", config);

        // Wait for min_state_duration to pass
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Record failures
        for _ in 0..3 {
            cb.record_failure("test error", false).await;
        }

        // Circuit should be open
        assert_eq!(cb.current_state().await, CircuitState::Open);
        assert!(cb.can_execute().await.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            open_duration: Duration::from_millis(10),
            min_state_duration: Duration::from_millis(1),
            half_open_max_requests: 5,
            ..Default::default()
        };
        let cb = ProviderCircuitBreaker::new("test", config);

        // Wait for min_state_duration to pass
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Trip circuit
        cb.record_failure("error", false).await;
        cb.record_failure("error", false).await;
        assert_eq!(cb.current_state().await, CircuitState::Open);

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(15)).await;

        // Should transition to half-open on next check
        assert!(cb.can_execute().await.is_ok());
        assert_eq!(cb.current_state().await, CircuitState::HalfOpen);

        // Record successes
        cb.record_success().await;
        cb.record_success().await;

        // Should be closed now
        assert_eq!(cb.current_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_rate_limit_ignored() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ignore_rate_limits: true,
            min_state_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let cb = ProviderCircuitBreaker::new("test", config);

        // Rate limit errors shouldn't trip circuit
        for _ in 0..10 {
            cb.record_failure("rate limited", true).await;
        }

        assert_eq!(cb.current_state().await, CircuitState::Closed);
    }
}
