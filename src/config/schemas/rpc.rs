/// RPC endpoint configuration
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// RPC endpoint configuration
    pub struct RpcConfig {
        /// List of RPC URLs to use (round-robin)
        #[metadata(field_metadata! {
            label: "RPC URLs",
            hint: "Comma-separated RPC endpoints (round-robin)",
            impact: "critical",
            category: "Endpoints",
        })]
        urls: Vec<String> = vec!["https://api.mainnet-beta.solana.com".to_string()],

        // Provider Selection
        #[metadata(field_metadata! {
            label: "Selection Strategy",
            hint: "Provider selection strategy: adaptive, round_robin, priority, latency",
            impact: "medium",
            category: "Provider Selection",
        })]
        selection_strategy: String = "adaptive".to_string(),

        // Rate Limiting
        #[metadata(field_metadata! {
            label: "Default Rate Limit",
            hint: "Default requests per second for unknown providers",
            min: 1,
            max: 100,
            step: 1,
            unit: "req/s",
            impact: "high",
            category: "Rate Limiting",
        })]
        default_rate_limit: u32 = 10,
        #[metadata(field_metadata! {
            label: "Helius Rate Limit",
            hint: "Requests per second for Helius provider",
            min: 1,
            max: 200,
            step: 5,
            unit: "req/s",
            impact: "medium",
            category: "Rate Limiting",
        })]
        helius_rate_limit: u32 = 50,
        #[metadata(field_metadata! {
            label: "QuickNode Rate Limit",
            hint: "Requests per second for QuickNode provider",
            min: 1,
            max: 100,
            step: 5,
            unit: "req/s",
            impact: "medium",
            category: "Rate Limiting",
        })]
        quicknode_rate_limit: u32 = 25,
        #[metadata(field_metadata! {
            label: "Triton Rate Limit",
            hint: "Requests per second for Triton provider",
            min: 1,
            max: 200,
            step: 10,
            unit: "req/s",
            impact: "medium",
            category: "Rate Limiting",
        })]
        triton_rate_limit: u32 = 100,
        #[metadata(field_metadata! {
            label: "Public Rate Limit",
            hint: "Requests per second for public endpoints",
            min: 1,
            max: 20,
            step: 1,
            unit: "req/s",
            impact: "high",
            category: "Rate Limiting",
        })]
        public_rate_limit: u32 = 4,
        #[metadata(field_metadata! {
            label: "Rate Limit Burst Factor",
            hint: "Allow burst up to factor * limit",
            min: 1.0,
            max: 2.0,
            step: 0.1,
            impact: "low",
            category: "Rate Limiting",
        })]
        rate_limit_burst_factor: f32 = 1.2,

        // Circuit Breaker
        #[metadata(field_metadata! {
            label: "Circuit Breaker Enabled",
            hint: "Enable circuit breaker pattern for provider failover",
            impact: "high",
            category: "Circuit Breaker",
        })]
        circuit_breaker_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Failure Threshold",
            hint: "Consecutive failures before opening circuit",
            min: 1,
            max: 20,
            step: 1,
            unit: "failures",
            impact: "medium",
            category: "Circuit Breaker",
        })]
        circuit_breaker_failure_threshold: u32 = 5,
        #[metadata(field_metadata! {
            label: "Success Threshold",
            hint: "Successes needed in half-open state to close circuit",
            min: 1,
            max: 10,
            step: 1,
            unit: "successes",
            impact: "medium",
            category: "Circuit Breaker",
        })]
        circuit_breaker_success_threshold: u32 = 3,
        #[metadata(field_metadata! {
            label: "Open Duration",
            hint: "Duration to keep circuit open before testing recovery",
            min: 5,
            max: 120,
            step: 5,
            unit: "seconds",
            impact: "medium",
            category: "Circuit Breaker",
        })]
        circuit_breaker_open_duration_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Half-Open Requests",
            hint: "Number of probe requests in half-open state",
            min: 1,
            max: 10,
            step: 1,
            unit: "requests",
            impact: "low",
            category: "Circuit Breaker",
        })]
        circuit_breaker_half_open_requests: u32 = 3,

        // Timeouts
        #[metadata(field_metadata! {
            label: "Request Timeout",
            hint: "Timeout for individual RPC requests",
            min: 5,
            max: 120,
            step: 5,
            unit: "seconds",
            impact: "high",
            category: "Timeouts",
        })]
        request_timeout_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Connection Timeout",
            hint: "Timeout for establishing connections",
            min: 1,
            max: 30,
            step: 1,
            unit: "seconds",
            impact: "medium",
            category: "Timeouts",
        })]
        connection_timeout_secs: u64 = 10,

        // Retries
        #[metadata(field_metadata! {
            label: "Max Retries",
            hint: "Maximum retry attempts on failure",
            min: 0,
            max: 10,
            step: 1,
            unit: "retries",
            impact: "medium",
            category: "Retries",
        })]
        max_retries: u32 = 3,
        #[metadata(field_metadata! {
            label: "Retry Base Delay",
            hint: "Initial delay before first retry (exponential backoff)",
            min: 50,
            max: 1000,
            step: 50,
            unit: "ms",
            impact: "low",
            category: "Retries",
        })]
        retry_base_delay_ms: u64 = 100,
        #[metadata(field_metadata! {
            label: "Retry Max Delay",
            hint: "Maximum delay between retries",
            min: 1000,
            max: 30000,
            step: 1000,
            unit: "ms",
            impact: "low",
            category: "Retries",
        })]
        retry_max_delay_ms: u64 = 5000,

        // Connection Pooling
        #[metadata(field_metadata! {
            label: "Connections Per Host",
            hint: "HTTP connection pool size per host",
            min: 1,
            max: 50,
            step: 5,
            unit: "connections",
            impact: "medium",
            category: "Connection Pooling",
        })]
        pool_connections_per_host: u32 = 10,
        #[metadata(field_metadata! {
            label: "Pool Idle Timeout",
            hint: "Idle connection timeout in pool",
            min: 30,
            max: 300,
            step: 30,
            unit: "seconds",
            impact: "low",
            category: "Connection Pooling",
        })]
        pool_idle_timeout_secs: u64 = 90,

        // Stats Collection
        #[metadata(field_metadata! {
            label: "Stats Enabled",
            hint: "Enable RPC statistics collection",
            impact: "low",
            category: "Statistics",
        })]
        stats_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Stats Retention Days",
            hint: "Number of days to retain statistics",
            min: 1,
            max: 30,
            step: 1,
            unit: "days",
            impact: "low",
            category: "Statistics",
        })]
        stats_retention_days: u32 = 7,
        #[metadata(field_metadata! {
            label: "Minute Buckets",
            hint: "Enable per-minute time series data",
            impact: "low",
            category: "Statistics",
        })]
        stats_minute_buckets: bool = true,

        // Debug
        #[metadata(field_metadata! {
            label: "Debug RPC",
            hint: "Enable detailed RPC debug logging",
            impact: "low",
            category: "Debug",
        })]
        debug_rpc: bool = false,
    }
}
