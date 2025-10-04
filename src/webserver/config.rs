/// Webserver configuration management
///
/// Handles loading, validation, and runtime configuration for the webserver dashboard.
/// Configuration can come from environment variables or the main configs.json file.

use serde::{Deserialize, Serialize};

/// Complete webserver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebserverConfig {
    /// Enable/disable the webserver
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Host to bind to (e.g., "127.0.0.1" for localhost only, "0.0.0.0" for all interfaces)
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// WebSocket configuration
    #[serde(default)]
    pub websocket: WebSocketConfig,
}

/// CORS (Cross-Origin Resource Sharing) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (e.g., ["http://localhost:3000"])
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods
    #[serde(default = "default_allowed_methods")]
    pub allowed_methods: Vec<String>,

    /// Max age for preflight cache (seconds)
    #[serde(default = "default_max_age")]
    pub max_age: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    /// Burst size (immediate requests allowed)
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable/disable authentication
    #[serde(default = "default_auth_enabled")]
    pub enabled: bool,

    /// API key for simple authentication (optional)
    #[serde(default)]
    pub api_key: Option<String>,

    /// JWT secret for token-based auth (optional)
    #[serde(default)]
    pub jwt_secret: Option<String>,
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Maximum concurrent WebSocket connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Ping interval for keep-alive (seconds)
    #[serde(default = "default_ping_interval")]
    pub ping_interval_seconds: u64,

    /// Maximum message size in bytes
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
}

// ================================================================================================
// Default Value Functions
// ================================================================================================

fn default_enabled() -> bool {
    false // Disabled by default for safety
}

fn default_host() -> String {
    "127.0.0.1".to_string() // Localhost only by default
}

fn default_port() -> u16 {
    8080
}

fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:3000".to_string()]
}

fn default_allowed_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
    ]
}

fn default_max_age() -> u64 {
    3600 // 1 hour
}

fn default_requests_per_minute() -> u32 {
    60
}

fn default_burst_size() -> u32 {
    10
}

fn default_auth_enabled() -> bool {
    false // No auth by default for development
}

fn default_max_connections() -> usize {
    100
}

fn default_ping_interval() -> u64 {
    30
}

fn default_max_message_size() -> usize {
    65536 // 64KB
}

// ================================================================================================
// Implementation
// ================================================================================================

impl Default for WebserverConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            host: default_host(),
            port: default_port(),
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            auth: AuthConfig::default(),
            websocket: WebSocketConfig::default(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_allowed_origins(),
            allowed_methods: default_allowed_methods(),
            max_age: default_max_age(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            burst_size: default_burst_size(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_auth_enabled(),
            api_key: None,
            jwt_secret: None,
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            ping_interval_seconds: default_ping_interval(),
            max_message_size: default_max_message_size(),
        }
    }
}

impl WebserverConfig {
    /// Load configuration from environment variables (overrides defaults)
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Override with environment variables if present
        if let Ok(enabled) = std::env::var("WEBSERVER_ENABLED") {
            config.enabled = enabled.parse().unwrap_or(false);
        }

        if let Ok(host) = std::env::var("WEBSERVER_HOST") {
            config.host = host;
        }

        if let Ok(port) = std::env::var("WEBSERVER_PORT") {
            config.port = port.parse().unwrap_or(8080);
        }

        if let Ok(api_key) = std::env::var("WEBSERVER_API_KEY") {
            config.auth.enabled = true;
            config.auth.api_key = Some(api_key);
        }

        config
    }

    /// Validate configuration (check for invalid values)
    pub fn validate(&self) -> Result<(), String> {
        // Validate host
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }

        // Validate port
        if self.port == 0 {
            return Err("Port cannot be 0".to_string());
        }

        // Validate rate limiting
        if self.rate_limit.requests_per_minute == 0 {
            return Err("Requests per minute must be > 0".to_string());
        }

        if self.rate_limit.burst_size == 0 {
            return Err("Burst size must be > 0".to_string());
        }

        // Validate WebSocket config
        if self.websocket.max_connections == 0 {
            return Err("Max WebSocket connections must be > 0".to_string());
        }

        if self.websocket.max_message_size == 0 {
            return Err("Max WebSocket message size must be > 0".to_string());
        }

        // Validate auth config
        if self.auth.enabled && self.auth.api_key.is_none() && self.auth.jwt_secret.is_none() {
            return Err("Authentication enabled but no API key or JWT secret provided".to_string());
        }

        Ok(())
    }

    /// Get the full bind address (host:port)
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebserverConfig::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bind_address() {
        let config = WebserverConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_validation() {
        let mut config = WebserverConfig::default();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid port
        config.port = 0;
        assert!(config.validate().is_err());

        // Reset and test auth validation
        config = WebserverConfig::default();
        config.auth.enabled = true;
        assert!(config.validate().is_err()); // No API key or JWT secret

        config.auth.api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok());
    }
}
