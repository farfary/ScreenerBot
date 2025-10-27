use crate::config_struct;

// ============================================================================
// CONNECTIVITY MONITORING CONFIGURATION
// ============================================================================

config_struct! {
    /// Connectivity monitoring configuration
    pub struct ConnectivityMonitoringConfig {
        /// Enable connectivity monitoring
        enabled: bool = true,

        /// Health check interval in seconds
        check_interval_secs: u64 = 30,

        /// Timeout for health checks in seconds
        health_check_timeout_secs: u64 = 5,

        /// Number of consecutive failures before marking unhealthy
        failure_threshold: u32 = 3,

        /// Number of consecutive successes to mark healthy again
        recovery_threshold: u32 = 2,

        /// Internet connectivity monitoring
        internet: InternetMonitorConfig = InternetMonitorConfig::default(),

        /// Endpoint-specific configurations
        endpoints: EndpointsMonitorConfig = EndpointsMonitorConfig::default(),
    }
}

config_struct! {
    /// Internet connectivity monitoring configuration
    pub struct InternetMonitorConfig {
        /// Enable internet connectivity checks
        enabled: bool = true,

        /// DNS servers to check (IP addresses)
        dns_servers: Vec<String> = vec![
            "8.8.8.8".to_string(),
            "1.1.1.1".to_string(),
        ],

        /// HTTP endpoints to check for connectivity
        http_checks: Vec<String> = vec![
            "https://www.google.com".to_string(),
            "https://solana.com".to_string(),
        ],
    }
}

config_struct! {
    /// Individual endpoint monitoring configurations
    pub struct EndpointsMonitorConfig {
        /// RPC endpoint monitoring
        rpc: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 5,
        },

        /// DexScreener API monitoring
        dexscreener: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 5,
        },

        /// GeckoTerminal API monitoring
        geckoterminal: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 5,
        },

        /// Rugcheck API monitoring
        rugcheck: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 10,
        },

        /// Jupiter API monitoring
        jupiter: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 5,
        },

        /// GMGN API monitoring
        gmgn: EndpointMonitorConfig = EndpointMonitorConfig {
            enabled: true,
            timeout_secs: 10,
        },
    }
}

config_struct! {
    /// Configuration for a single endpoint monitor
    pub struct EndpointMonitorConfig {
        /// Enable monitoring for this endpoint
        enabled: bool = true,

        /// Timeout for health checks in seconds (overrides global if set)
        timeout_secs: u64 = 5,
    }
}
