/// Runtime logger configuration system
///
/// This module manages the logger's runtime state, including:
/// - Which log levels to show
/// - Which modules have debug mode enabled (from --debug-<module> flags)
/// - Output settings (console, file, colors)

use super::levels::LogLevel;
use super::tags::LogTag;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Logger runtime configuration
#[derive(Clone)]
pub struct LoggerConfig {
    /// Minimum log level to display (filters out lower priority logs)
    pub min_level: LogLevel,

    /// Per-module debug mode flags (populated from command-line arguments)
    /// Key: LogTag, Value: whether debug mode is enabled for that tag
    pub debug_modes: HashMap<String, bool>,

    /// Per-module verbose mode flags (populated from command-line arguments)
    /// Key: LogTag, Value: whether verbose mode is enabled for that tag
    pub verbose_modes: HashMap<String, bool>,

    /// Specific tags to enable (empty = all enabled)
    pub enabled_tags: HashSet<String>,

    /// Console output enabled
    pub console_enabled: bool,

    /// File output enabled
    pub file_enabled: bool,

    /// Color output enabled
    pub colors_enabled: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            debug_modes: HashMap::new(),
            verbose_modes: HashMap::new(),
            enabled_tags: HashSet::new(), // Empty = all enabled
            console_enabled: true,
            file_enabled: true,
            colors_enabled: true,
        }
    }
}

/// Global logger configuration singleton
static LOGGER_CONFIG: Lazy<Arc<RwLock<LoggerConfig>>> =
    Lazy::new(|| Arc::new(RwLock::new(LoggerConfig::default())));

/// Get a copy of the current logger configuration
pub fn get_logger_config() -> LoggerConfig {
    LOGGER_CONFIG
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Set the logger configuration (replaces entire config)
pub fn set_logger_config(config: LoggerConfig) {
    *LOGGER_CONFIG.write().unwrap_or_else(|e| e.into_inner()) = config;
}

/// Update logger configuration with a closure
pub fn update_logger_config<F>(f: F)
where
    F: FnOnce(&mut LoggerConfig),
{
    let mut config = LOGGER_CONFIG.write().unwrap_or_else(|e| e.into_inner());
    f(&mut config);
}

/// Initialize logger configuration from command-line arguments
/// This is called automatically during logger::init()
pub fn init_from_args() {
    use crate::arguments::{get_cmd_args, has_arg};

    let mut config = LoggerConfig::default();
    let args = get_cmd_args();

    // Parse all --debug-<module> flags dynamically
    for arg in &args {
        if let Some(module) = arg.strip_prefix("--debug-") {
            // Map argument to tag name
            let tag_name = match module {
                "api" => "api",
                "tokens" => "tokens",
                "pool-service" => "pool_service",
                "pool-calculator" => "pool_calculator",
                "pool-discovery" => "pool_discovery",
                "pool-fetcher" => "pool_fetcher",
                "pool-analyzer" => "pool_analyzer",
                "pool-cache" => "pool_cache",
                "pool-decoders" => "pool_decoder",
                "pool-prices" => "pool",
                "pool-cleanup" => "pool",
                "pool-monitor" => "monitor",
                "pool-tokens" => "pool",
                "trader" => "trader",
                "monitor" => "monitor",
                "discovery" => "discovery",
                "price-service" => "price_service",
                "sol-price" => "sol_price",
                "entry" => "entry",
                "ohlcv" => "ohlcv",
                "wallet" => "wallet",
                "swaps" => "swap",
                "decimals" => "decimals",
                "transactions" => "transactions",
                "websocket" => "websocket",
                "rpc" => "rpc",
                "positions" => "positions",
                "ata" => "system",
                "blacklist" => "blacklist",
                "security" => "security",
                "webserver" => "webserver",
                "filtering" => "filtering",
                "profit" => "profit",
                "system" => "system",
                _ => continue, // Unknown debug flag, skip
            };

            config.debug_modes.insert(tag_name.to_string(), true);
        } else if let Some(module) = arg.strip_prefix("--verbose-") {
            // Map argument to tag name (same mapping as debug)
            let tag_name = match module {
                "api" => "api",
                "tokens" => "tokens",
                "pool-service" => "pool_service",
                "pool-calculator" => "pool_calculator",
                "pool-discovery" => "pool_discovery",
                "pool-fetcher" => "pool_fetcher",
                "pool-analyzer" => "pool_analyzer",
                "pool-cache" => "pool_cache",
                "pool-decoders" => "pool_decoder",
                "pool-prices" => "pool",
                "pool-cleanup" => "pool",
                "pool-monitor" => "monitor",
                "pool-tokens" => "pool",
                "trader" => "trader",
                "monitor" => "monitor",
                "discovery" => "discovery",
                "price-service" => "price_service",
                "sol-price" => "sol_price",
                "entry" => "entry",
                "ohlcv" => "ohlcv",
                "wallet" => "wallet",
                "swaps" => "swap",
                "decimals" => "decimals",
                "transactions" => "transactions",
                "websocket" => "websocket",
                "rpc" => "rpc",
                "positions" => "positions",
                "ata" => "system",
                "blacklist" => "blacklist",
                "security" => "security",
                "webserver" => "webserver",
                "filtering" => "filtering",
                "profit" => "profit",
                "system" => "system",
                _ => continue, // Unknown verbose flag, skip
            };

            config.verbose_modes.insert(tag_name.to_string(), true);
        }
    }

    // Check for --verbose flag
    if has_arg("--verbose") || has_arg("-v") {
        config.min_level = LogLevel::Verbose;
    }

    // Check for --quiet flag (only errors and warnings)
    if has_arg("--quiet") || has_arg("-q") {
        config.min_level = LogLevel::Warning;
    }

    // Store configuration
    set_logger_config(config);
}

/// Check if debug mode is enabled for a specific tag
pub fn is_debug_enabled_for_tag(tag: &LogTag) -> bool {
    let config = get_logger_config();
    let tag_name = tag.to_debug_key();
    config.debug_modes.get(&tag_name).copied().unwrap_or(false)
}

/// Check if verbose mode is enabled for a specific tag
pub fn is_verbose_enabled_for_tag(tag: &LogTag) -> bool {
    let config = get_logger_config();
    let tag_name = tag.to_debug_key();
    config.verbose_modes.get(&tag_name).copied().unwrap_or(false)
}
