/// Configuration schemas - all config structures defined once with defaults
///
/// This module contains all configuration structures for ScreenerBot.
/// Each struct is defined using the config_struct! macro which provides:
/// - Single-source definition (no repetition)
/// - Embedded defaults
/// - Type safety
/// - Serde support
use crate::config_struct;

// ============================================================================
// RPC CONFIGURATION
// ============================================================================

config_struct! {
    /// RPC endpoint configuration
    pub struct RpcConfig {
        /// List of RPC URLs to use (round-robin)
        urls: Vec<String> = vec!["https://api.mainnet-beta.solana.com".to_string()],
    }
}

// ============================================================================
// TRADER CONFIGURATION
// ============================================================================

config_struct! {
    /// Trading system configuration
    pub struct TraderConfig {
        // Trader control
        enabled: bool = true,

        // Core trading parameters
        max_open_positions: usize = 2,
        trade_size_sol: f64 = 0.005,

        // Profit thresholds
        min_profit_threshold_enabled: bool = true,
        min_profit_threshold_percent: f64 = 2.0,
        profit_extra_needed_sol: f64 = 0.00005,

        // Time-based overrides
        time_override_duration_hours: f64 = 168.0,
        time_override_loss_threshold_percent: f64 = -40.0,

        // Slippage configuration
        slippage_quote_default_pct: f64 = 3.0,
        slippage_exit_profit_shortfall_pct: f64 = 8.0,
        slippage_exit_loss_shortfall_pct: f64 = 15.0,
        slippage_exit_retry_steps_pct: Vec<f64> = vec![3.0, 5.0, 8.0, 12.0, 15.0],

        // Debug modes
        debug_force_sell_mode: bool = false,
        debug_force_sell_timeout_secs: f64 = 45.0,
        debug_force_buy_mode: bool = false,
        debug_force_buy_drop_threshold_percent: f64 = 0.5,

        // Position timing
        position_close_cooldown_minutes: i64 = 15,

        // Monitoring intervals
        entry_monitor_interval_secs: u64 = 3,
        position_monitor_interval_secs: u64 = 2,

        // Task timeouts
        semaphore_acquire_timeout_secs: u64 = 60,
        token_check_task_timeout_secs: u64 = 20,
        token_check_collection_timeout_secs: u64 = 30,
        token_check_handle_timeout_secs: u64 = 25,
        sell_operations_collection_timeout_secs: u64 = 240,
        sell_operation_smart_timeout_secs: u64 = 600,
        sell_semaphore_acquire_timeout_secs: u64 = 30,
        sell_task_handle_timeout_secs: u64 = 200,
        entry_cycle_min_wait_ms: u64 = 100,
        token_processing_shutdown_check_ms: u64 = 10,
        task_shutdown_check_ms: u64 = 10,
        sell_operation_shutdown_check_ms: u64 = 10,
        collection_shutdown_check_ms: u64 = 50,
        entry_check_concurrency: usize = 10,
    }
}

// ============================================================================
// POSITIONS CONFIGURATION
// ============================================================================

config_struct! {
    /// Position management configuration
    pub struct PositionsConfig {
        /// Cooldown between position opens (seconds)
        position_open_cooldown_secs: i64 = 5,

        /// TTL for pending open swaps (seconds)
        pending_open_ttl_secs: i64 = 120,

        /// Extra SOL needed for profit calculations (accounts for priority fees, etc.)
        profit_extra_needed_sol: f64 = 0.0002,
    }
}

// ============================================================================
// FILTERING CONFIGURATION
// ============================================================================

config_struct! {
    /// Token filtering configuration
    pub struct FilteringConfig {
        // Cache settings
        filter_cache_ttl_secs: u64 = 15,
        target_filtered_tokens: usize = 1000,
        max_tokens_to_process: usize = 5000,

        // Basic requirements
        require_name_and_symbol: bool = true,
        require_logo_url: bool = false,
        require_website_url: bool = false,

        // Token age
        min_token_age_minutes: i64 = 60,

        // Transaction activity
        min_transactions_5min: i64 = 1,
        min_transactions_1h: i64 = 5,

        // Liquidity
        min_liquidity_usd: f64 = 1.0,
        max_liquidity_usd: f64 = 100_000_000.0,

        // Market cap
        min_market_cap_usd: f64 = 1000.0,
        max_market_cap_usd: f64 = 100_000_000.0,

        // Security requirements
        min_security_score: i32 = 10,
    max_top_holder_pct: f64 = 15.0,
    max_top_3_holders_pct: f64 = 35.0,
        min_pumpfun_lp_lock_pct: f64 = 50.0,
        min_regular_lp_lock_pct: f64 = 50.0,
        min_unique_holders: u32 = 500,
    }
}

// ============================================================================
// SWAPS CONFIGURATION
// ============================================================================

config_struct! {
    /// Swap router configuration
    pub struct SwapsConfig {
        // Router enable/disable
        gmgn_enabled: bool = true,
        jupiter_enabled: bool = true,
        raydium_enabled: bool = false,

        // Common timeouts
        quote_timeout_secs: u64 = 15,
        api_timeout_secs: u64 = 30,
        retry_attempts: u32 = 3,

        // Transaction confirmation timeouts
        transaction_confirmation_timeout_secs: u64 = 300,
        priority_confirmation_timeout_secs: u64 = 30,
        transaction_confirmation_max_attempts: u32 = 20,
        priority_confirmation_max_attempts: u32 = 15,
        transaction_confirmation_retry_delay_ms: u64 = 3000,
        priority_confirmation_retry_delay_ms: u64 = 1000,
        fast_failure_threshold_attempts: u32 = 10,

        // Confirmation delay configuration
        initial_confirmation_delay_ms: u64 = 5000,
        max_confirmation_delay_secs: u64 = 8,
        confirmation_backoff_multiplier: f64 = 1.5,
        confirmation_timeout_secs: u64 = 60,
        priority_confirmation_timeout_secs_mod: u64 = 5,

        // Rate limit handling
        rate_limit_base_delay_secs: u64 = 5,
        rate_limit_increment_secs: u64 = 2,

        // Early attempt delays
        early_attempt_delay_ms: u64 = 1000,
        early_attempts_count: u32 = 3,

        // GMGN specific
        gmgn_quote_api: String = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route".to_string(),
        gmgn_partner: String = "screenerbot".to_string(),
        gmgn_anti_mev: bool = false,
        gmgn_fee_sol: f64 = 0.0,
        gmgn_default_swap_mode: String = "ExactIn".to_string(),

        // Jupiter specific
        jupiter_quote_api: String = "https://lite-api.jup.ag/swap/v1/quote".to_string(),
        jupiter_swap_api: String = "https://lite-api.jup.ag/swap/v1/swap".to_string(),
        jupiter_dynamic_compute_unit_limit: bool = false,
        jupiter_default_priority_fee: u64 = 1000,
        jupiter_default_swap_mode: String = "ExactIn".to_string(),

        // Slippage configuration
        slippage_quote_default_pct: f64 = 1.0,
        slippage_exit_profit_shortfall_pct: f64 = 3.0,
        slippage_exit_loss_shortfall_pct: f64 = 5.0,
        slippage_exit_retry_steps_pct: Vec<f64> = vec![3.0, 10.0, 25.0],
    }
}

// ============================================================================
// TOKENS CONFIGURATION
// ============================================================================

config_struct! {
    /// Token management configuration
    pub struct TokensConfig {
        // API rate limits
        dexscreener_rate_limit_per_minute: usize = 100,
        dexscreener_discovery_rate_limit: usize = 60,
        max_tokens_per_api_call: usize = 30,
        raydium_rate_limit_per_minute: usize = 120,
        geckoterminal_rate_limit_per_minute: usize = 30,
        max_tokens_per_batch: usize = 30,

        // Price validation
        max_price_deviation_percent: f64 = 50.0,

        // Decimals
        max_accounts_per_call: usize = 100,
        max_decimal_retry_attempts: i32 = 3,

        // Blacklist
        low_liquidity_threshold: f64 = 100.0,
        min_age_hours: i64 = 2,
        max_low_liquidity_count: u32 = 5,
        max_no_route_failures: u32 = 5,
        cache_refresh_interval_minutes: i64 = 5,

        // OHLCV
        max_ohlcv_age_hours: i64 = 168,
        max_memory_cache_entries: usize = 500,
        max_ohlcv_limit: u32 = 2000,
        default_ohlcv_limit: u32 = 100,

        // Token monitor
        max_update_interval_hours: i64 = 2,
        new_token_boost_max_age_minutes: i64 = 60,

        // Patterns
        max_pattern_length: usize = 8,
    }
}

// ============================================================================
// SOL PRICE SERVICE
// ============================================================================

config_struct! {
    /// SOL price tracking configuration
    pub struct SolPriceConfig {
        /// Price refresh interval (seconds)
        price_refresh_interval_secs: u64 = 30,
    }
}

// ============================================================================
// SUMMARY DISPLAY
// ============================================================================

config_struct! {
    /// Summary display configuration
    pub struct SummaryConfig {
        /// Display refresh interval (seconds)
        summary_display_interval_secs: u64 = 15,

        /// Maximum recent closed positions to display
        max_recent_closed_positions: usize = 20,
    }
}

// ============================================================================
// EVENTS SYSTEM
// ============================================================================

config_struct! {
    /// Events system configuration
    pub struct EventsConfig {
        /// Batch timeout (milliseconds)
        batch_timeout_ms: u64 = 100,
    }
}

// ============================================================================
// WEBSERVER CONFIGURATION
// ============================================================================

config_struct! {
    /// CORS configuration
    pub struct CorsConfig {
        allowed_origins: Vec<String> = vec!["http://localhost:3000".to_string()],
        allowed_methods: Vec<String> = vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
        ],
    }
}

config_struct! {
    /// Rate limiting configuration
    pub struct RateLimitConfig {
        enabled: bool = true,
        requests_per_minute: u32 = 100,
    }
}

config_struct! {
    /// Authentication configuration
    pub struct AuthConfig {
        enabled: bool = false,
        api_key: String = String::new(),
    }
}

config_struct! {
    /// WebSocket configuration
    pub struct WebSocketConfig {
        enabled: bool = true,
        max_connections: usize = 100,
        heartbeat_interval_secs: u64 = 30,
    }
}

config_struct! {
    /// Tokens tab webserver configuration
    pub struct TokensTabConfig {
        /// Default page size for token lists
        default_page_size: usize = 50,

        /// Maximum page size (enforced limit)
        max_page_size: usize = 200,

        /// Auto-refresh interval (milliseconds)
        auto_refresh_interval_ms: u64 = 2000,

        /// Price staleness warning threshold (seconds)
        price_staleness_threshold_seconds: u64 = 60,

        /// Security score threshold for "secure" view
        secure_token_score_threshold: i32 = 500,

        /// Recent token lookback period (hours)
        recent_token_hours: i64 = 24,

        /// Enable OHLCV charts
        enable_ohlcv_charts: bool = true,

        /// Enable token detail page
        enable_detail_page: bool = true,
    }
}

config_struct! {
    /// Webserver configuration
    pub struct WebserverConfig {
        enabled: bool = true,
        host: String = "127.0.0.1".to_string(),
        port: u16 = 8080,
        cors: CorsConfig = CorsConfig::default(),
        rate_limit: RateLimitConfig = RateLimitConfig::default(),
        auth: AuthConfig = AuthConfig::default(),
        websocket: WebSocketConfig = WebSocketConfig::default(),
        tokens_tab: TokensTabConfig = TokensTabConfig::default(),
    }
}

// ============================================================================
// SERVICES CONFIGURATION
// ============================================================================

config_struct! {
    /// Individual service configuration
    pub struct ServiceConfig {
        enabled: bool = true,
        priority: i32 = 50,
    }
}

config_struct! {
    /// Services configuration
    pub struct ServicesConfig {
        events: ServiceConfig = ServiceConfig { enabled: true, priority: 10 },
        blacklist: ServiceConfig = ServiceConfig { enabled: true, priority: 15 },
        tokens: ServiceConfig = ServiceConfig { enabled: true, priority: 20 },
        positions: ServiceConfig = ServiceConfig { enabled: true, priority: 50 },
        pools: ServiceConfig = ServiceConfig { enabled: true, priority: 30 },
        trader: ServiceConfig = ServiceConfig { enabled: true, priority: 100 },
    }
}

// ============================================================================
// MONITORING CONFIGURATION
// ============================================================================

config_struct! {
    /// System monitoring configuration
    pub struct MonitoringConfig {
        metrics_interval_secs: u64 = 30,
        health_check_interval_secs: u64 = 60,
    }
}

// ============================================================================
// ROOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Root configuration structure containing all sub-configurations
    pub struct Config {
        /// Main wallet private key (base58 or array format)
        main_wallet_private: String = String::new(),

        /// RPC configuration
        rpc: RpcConfig = RpcConfig::default(),

        /// Trader configuration
        trader: TraderConfig = TraderConfig::default(),

        /// Positions configuration
        positions: PositionsConfig = PositionsConfig::default(),

        /// Filtering configuration
        filtering: FilteringConfig = FilteringConfig::default(),

        /// Swaps configuration
        swaps: SwapsConfig = SwapsConfig::default(),

        /// Tokens configuration
        tokens: TokensConfig = TokensConfig::default(),

        /// SOL price service configuration
        sol_price: SolPriceConfig = SolPriceConfig::default(),

        /// Summary display configuration
        summary: SummaryConfig = SummaryConfig::default(),

        /// Events system configuration
        events: EventsConfig = EventsConfig::default(),

        /// Webserver configuration
        webserver: WebserverConfig = WebserverConfig::default(),

        /// Services configuration
        services: ServicesConfig = ServicesConfig::default(),

        /// Monitoring configuration
        monitoring: MonitoringConfig = MonitoringConfig::default(),
    }
}

// ============================================================================
// IMPLEMENTATIONS
// ============================================================================

impl WebserverConfig {
    /// Validate webserver configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }

        if self.port == 0 {
            return Err("Port cannot be 0".to_string());
        }

        if self.rate_limit.enabled && self.rate_limit.requests_per_minute == 0 {
            return Err("Rate limit requests_per_minute must be > 0 when enabled".to_string());
        }

        if self.websocket.enabled {
            if self.websocket.max_connections == 0 {
                return Err("WebSocket max_connections must be > 0 when enabled".to_string());
            }
            if self.websocket.heartbeat_interval_secs == 0 {
                return Err("WebSocket heartbeat_interval_secs must be > 0".to_string());
            }
        }

        Ok(())
    }

    /// Get the full bind address (host:port)
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
