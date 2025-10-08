/// WebSocket health monitoring
///
/// Tracks connection health with heartbeat and timeout management.
use std::time::{Duration, Instant};
use tokio::time::interval;

// ============================================================================
// HEALTH CONFIG
// ============================================================================

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Heartbeat interval (server sends ping)
    pub heartbeat_interval: Duration,

    /// Client idle timeout (no activity)
    pub idle_timeout: Duration,

    /// Pong timeout (after ping sent)
    pub pong_timeout: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(90),
            pong_timeout: Duration::from_secs(10),
        }
    }
}

impl HealthConfig {
    /// Create from config values
    pub fn from_config(heartbeat_secs: u64, idle_timeout_secs: u64) -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(heartbeat_secs),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
            pong_timeout: Duration::from_secs(10), // Fixed for now
        }
    }
}

// ============================================================================
// CONNECTION HEALTH TRACKER
// ============================================================================

/// Connection health state
#[derive(Debug)]
pub struct ConnectionHealth {
    /// Last client activity (any message received)
    last_activity: Instant,

    /// Last ping sent to client
    last_ping: Option<Instant>,

    /// Health config
    config: HealthConfig,
}

impl ConnectionHealth {
    /// Create new health tracker
    pub fn new(config: HealthConfig) -> Self {
        Self {
            last_activity: Instant::now(),
            last_ping: None,
            config,
        }
    }

    /// Record client activity
    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
        self.last_ping = None; // Clear pending ping
    }

    /// Record ping sent
    pub fn record_ping(&mut self) {
        self.last_ping = Some(Instant::now());
    }

    /// Check if client is idle (no activity beyond timeout)
    pub fn is_idle(&self) -> bool {
        self.last_activity.elapsed() > self.config.idle_timeout
    }

    /// Check if pong is overdue (ping sent but no response)
    pub fn is_pong_overdue(&self) -> bool {
        self.last_ping
            .map(|ping_time| ping_time.elapsed() > self.config.pong_timeout)
            .unwrap_or(false)
    }

    /// Check if health check is needed
    pub fn needs_ping(&self) -> bool {
        self.last_activity.elapsed() > self.config.heartbeat_interval && self.last_ping.is_none()
    }

    /// Get seconds since last activity
    pub fn seconds_since_activity(&self) -> u64 {
        self.last_activity.elapsed().as_secs()
    }
}

/// Create a heartbeat ticker
pub fn heartbeat_ticker(heartbeat_secs: u64) -> tokio::time::Interval {
    interval(Duration::from_secs(heartbeat_secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_connection_health() {
        let config = HealthConfig {
            heartbeat_interval: Duration::from_millis(50),
            idle_timeout: Duration::from_millis(100),
            pong_timeout: Duration::from_millis(30),
        };

        let mut health = ConnectionHealth::new(config);

        // Initially not idle
        assert!(!health.is_idle());

        // After activity, still not idle
        health.record_activity();
        assert!(!health.is_idle());

        // Wait and check idle
        sleep(Duration::from_millis(150));
        assert!(health.is_idle());

        // Record ping
        health.record_ping();
        sleep(Duration::from_millis(50));
        assert!(health.is_pong_overdue());

        // Activity clears ping
        health.record_activity();
        assert!(!health.is_pong_overdue());
    }
}
