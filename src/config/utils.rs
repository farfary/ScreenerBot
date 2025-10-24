use super::schemas::Config;
use crate::logger::{self, LogTag};
/// Configuration utilities - loading, reloading, and access helpers
///
/// This module provides utility functions for working with the configuration system:
/// - Loading configuration from disk
/// - Hot-reloading configuration at runtime
/// - Thread-safe access helpers
/// - File watching for automatic reloads
use once_cell::sync::OnceCell;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::RwLock;

/// Global configuration instance
///
/// This is the single source of truth for all configuration values.
/// Access it using the helper functions below.
pub static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();

/// Default configuration file path
pub const CONFIG_FILE_PATH: &str = "data/config.toml";

/// Load configuration from disk and initialize the global CONFIG
///
/// This should be called once at startup. If the config file doesn't exist,
/// it will use default values from the schema definitions.
///
/// # Returns
/// - `Ok(())` - Configuration loaded successfully
/// - `Err(String)` - Error message if loading failed
///
/// # Example
/// ```
/// use screenerbot::config::load_config;
///
/// fn main() -> Result<(), String> {
///     load_config()?;
///     // Config is now available globally
///     Ok(())
/// }
/// ```
pub fn load_config() -> Result<(), String> {
    load_config_from_path(CONFIG_FILE_PATH)
}

/// Load configuration from a specific file path
///
/// # Arguments
/// * `path` - Path to the TOML configuration file
///
/// # Returns
/// - `Ok(())` - Configuration loaded successfully
/// - `Err(String)` - Error message if loading failed
pub fn load_config_from_path(path: &str) -> Result<(), String> {
    let config = if std::path::Path::new(path).exists() {
        // Load from file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        toml::from_str::<Config>(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?
    } else {
        // Use defaults if file doesn't exist
        crate::logger::warning(
            crate::logger::LogTag::System,
            &format!("⚠️  Config file '{}' not found, using default values", path),
        );
        Config::default()
    };

    CONFIG
        .set(RwLock::new(config))
        .map_err(|_| "Config already initialized".to_string())?;

    Ok(())
}

/// Reload configuration from disk
///
/// This allows hot-reloading configuration changes without restarting the application.
/// The configuration is atomically replaced, so reads are always consistent.
///
/// # Returns
/// - `Ok(())` - Configuration reloaded successfully
/// - `Err(String)` - Error message if reloading failed
///
/// # Example
/// ```
/// use screenerbot::config::reload_config;
///
/// // After modifying config.toml
/// reload_config()?;
/// // New values are now active
/// ```
pub fn reload_config() -> Result<(), String> {
    reload_config_from_path(CONFIG_FILE_PATH)
}

/// Validate configuration values before applying
///
/// # Arguments
/// * `config` - Configuration to validate
///
/// # Returns
/// - `Ok(())` - Configuration is valid
/// - `Err(String)` - Validation error message
fn validate_config(config: &Config) -> Result<(), String> {
    // Trader validation
    if config.trader.max_open_positions == 0 {
        return Err("trader.max_open_positions must be greater than 0".to_string());
    }
    if config.trader.trade_size_sol <= 0.0 {
        return Err("trader.trade_size_sol must be greater than 0".to_string());
    }
    if !config.trader.trade_size_sol.is_finite() {
        return Err("trader.trade_size_sol must be a finite number".to_string());
    }
    if config.trader.entry_check_concurrency == 0 {
        return Err("trader.entry_check_concurrency must be at least 1".to_string());
    }

    // DCA validation
    if config.trader.dca_enabled {
        if config.trader.dca_threshold_pct >= 0.0 {
            return Err(
                "trader.dca_threshold_pct must be negative (represents price drop percentage)"
                    .to_string(),
            );
        }
        if config.trader.dca_size_percentage <= 0.0 || config.trader.dca_size_percentage > 100.0 {
            return Err(
                "trader.dca_size_percentage must be between 0 and 100 (exclusive)".to_string(),
            );
        }
        if config.trader.dca_max_count == 0 {
            return Err("trader.dca_max_count must be at least 1 when DCA is enabled".to_string());
        }
    }

    // Positions validation
    if config.positions.profit_extra_needed_sol < 0.0
        || !config.positions.profit_extra_needed_sol.is_finite()
    {
        return Err(
            "positions.profit_extra_needed_sol must be non-negative and finite".to_string(),
        );
    }
    if config.positions.position_open_cooldown_secs < 0 {
        return Err("positions.position_open_cooldown_secs cannot be negative".to_string());
    }

    // Partial exit validation
    if config.positions.partial_exit_enabled {
        if config.positions.partial_exit_default_pct < 10.0
            || config.positions.partial_exit_default_pct > 90.0
        {
            return Err("positions.partial_exit_default_pct must be between 10 and 90".to_string());
        }
    }

    // Trailing stop validation
    if config.positions.trailing_stop_enabled {
        if config.positions.trailing_stop_activation_pct <= 0.0
            || config.positions.trailing_stop_activation_pct > 100.0
        {
            return Err(
                "positions.trailing_stop_activation_pct must be between 0 and 100 (exclusive)"
                    .to_string(),
            );
        }
        if config.positions.trailing_stop_distance_pct <= 0.0
            || config.positions.trailing_stop_distance_pct > 100.0
        {
            return Err(
                "positions.trailing_stop_distance_pct must be between 0 and 100 (exclusive)"
                    .to_string(),
            );
        }
        if config.positions.trailing_stop_distance_pct
            >= config.positions.trailing_stop_activation_pct
        {
            return Err(format!(
                "positions.trailing_stop_distance_pct ({:.1}%) must be less than trailing_stop_activation_pct ({:.1}%)",
                config.positions.trailing_stop_distance_pct,
                config.positions.trailing_stop_activation_pct
            ));
        }
    }

    // Slippage validation
    if config.swaps.slippage.quote_default_pct < 0.0
        || config.swaps.slippage.quote_default_pct > 100.0
    {
        return Err("swaps.slippage.quote_default_pct must be between 0 and 100".to_string());
    }
    if config.swaps.slippage.exit_profit_shortfall_pct < 0.0
        || config.swaps.slippage.exit_profit_shortfall_pct > 100.0
    {
        return Err(
            "swaps.slippage.exit_profit_shortfall_pct must be between 0 and 100".to_string(),
        );
    }
    if config.swaps.slippage.exit_loss_shortfall_pct < 0.0
        || config.swaps.slippage.exit_loss_shortfall_pct > 100.0
    {
        return Err("swaps.slippage.exit_loss_shortfall_pct must be between 0 and 100".to_string());
    }
    if config.swaps.slippage.exit_retry_steps_pct.is_empty() {
        return Err("swaps.slippage.exit_retry_steps_pct cannot be empty - at least one slippage step is required".to_string());
    }

    // Router availability check
    if !config.swaps.gmgn.enabled && !config.swaps.jupiter.enabled {
        return Err("At least one swap router (GMGN or Jupiter) must be enabled".to_string());
    }

    // RPC validation
    if config.rpc.urls.is_empty() {
        return Err("rpc.urls cannot be empty - at least one RPC endpoint is required".to_string());
    }

    Ok(())
}

/// Reload configuration from a specific file path
///
/// # Arguments
/// * `path` - Path to the TOML configuration file
///
/// # Returns
/// - `Ok(())` - Configuration reloaded successfully
/// - `Err(String)` - Error message if reloading failed
pub fn reload_config_from_path(path: &str) -> Result<(), String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

    let new_config = toml::from_str::<Config>(&contents)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

    // Validate configuration before applying
    validate_config(&new_config)?;

    if let Some(config_lock) = CONFIG.get() {
        let mut config = config_lock
            .write()
            .map_err(|e| format!("Failed to acquire config write lock: {}", e))?;
        *config = new_config;
        Ok(())
    } else {
        Err("Config not initialized. Call load_config() first.".to_string())
    }
}

/// Execute a function with read access to the configuration
///
/// This is the recommended way to read configuration values.
/// The closure receives an immutable reference to the Config.
///
/// # Arguments
/// * `f` - Closure that receives a reference to Config
///
/// # Returns
/// The return value of the closure
///
/// # Example
/// ```
/// use screenerbot::config::with_config;
///
/// let max_positions = with_config(|cfg| cfg.trader.max_open_positions);
/// let trade_size = with_config(|cfg| cfg.trader.trade_size_sol);
/// ```
pub fn with_config<F, R>(f: F) -> R
where
    F: FnOnce(&Config) -> R,
{
    let config_lock = CONFIG
        .get()
        .expect("Config not initialized. Call load_config() first.");

    let config = config_lock
        .read()
        .expect("Failed to acquire config read lock");

    f(&config)
}

/// Get a clone of the entire configuration
///
/// This is useful when you need to hold onto config values across await points.
/// Note: This clones the entire config, so use with_config() for simple reads.
///
/// # Returns
/// A cloned copy of the current configuration
///
/// # Example
/// ```
/// use screenerbot::config::get_config_clone;
///
/// async fn process() {
///     let cfg = get_config_clone();
///     // Can use cfg across await points
///     tokio::time::sleep(Duration::from_secs(1)).await;
///     println!("Max positions: {}", cfg.trader.max_open_positions);
/// }
/// ```
pub fn get_config_clone() -> Config {
    with_config(|cfg| cfg.clone())
}

/// Save the current configuration to disk
///
/// This writes the current in-memory configuration to the specified file.
/// Useful for persisting runtime changes.
///
/// # Arguments
/// * `path` - Path where to save the configuration (default: CONFIG_FILE_PATH)
///
/// # Returns
/// - `Ok(())` - Configuration saved successfully
/// - `Err(String)` - Error message if saving failed
pub fn save_config(path: Option<&str>) -> Result<(), String> {
    let path = path.unwrap_or(CONFIG_FILE_PATH);

    let config_str = with_config(|cfg| {
        toml::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize config: {}", e))
    })?;

    std::fs::write(path, config_str)
        .map_err(|e| format!("Failed to write config file '{}': {}", path, e))?;

    Ok(())
}

/// Check if configuration has been initialized
///
/// # Returns
/// `true` if load_config() has been called successfully
pub fn is_config_initialized() -> bool {
    CONFIG.get().is_some()
}

// ============================================================================
// WALLET MANAGEMENT FUNCTIONS
// ============================================================================

/// Load the main wallet keypair from the configuration
///
/// This function supports multiple private key formats:
/// - Base58 encoded string (standard Solana format)
/// - Array format like [1,2,3,4,...] (byte array representation)
///
/// The function performs comprehensive validation to ensure the private key
/// is exactly 64 bytes and can be successfully converted to a Keypair.
///
/// # Returns
/// - `Ok(Keypair)` - Successfully created Solana keypair
/// - `Err(String)` - Invalid format, wrong length, or parsing error
///
/// # Example
/// ```
/// use screenerbot::config::get_wallet_keypair;
///
/// let wallet = get_wallet_keypair()?;
/// println!("Wallet public key: {}", wallet.pubkey());
/// ```
pub fn get_wallet_keypair() -> Result<Keypair, String> {
    with_config(|cfg| {
        let private_key = &cfg.main_wallet_private;

        if private_key.is_empty() {
            return Err("Main wallet private key is empty in config".to_string());
        }

        // Parse the private key from base58 string or array format
        let keypair = if private_key.starts_with('[') && private_key.ends_with(']') {
            // Handle array format like [1,2,3,4,...]
            load_keypair_from_array_format(private_key)?
        } else {
            // Handle base58 format
            load_keypair_from_base58_format(private_key)?
        };

        Ok(keypair)
    })
}

/// Helper function to load keypair from array format
///
/// Parses private key strings in the format "[1,2,3,4,...]" where each
/// number represents a byte value from 0-255.
fn load_keypair_from_array_format(private_key_str: &str) -> Result<Keypair, String> {
    let private_key_str = private_key_str
        .trim_start_matches('[')
        .trim_end_matches(']');

    let private_key_bytes: Result<Vec<u8>, _> = private_key_str
        .split(',')
        .map(|s| s.trim().parse::<u8>())
        .collect();

    match private_key_bytes {
        Ok(bytes) => {
            if bytes.len() != 64 {
                return Err(format!(
                    "Invalid private key length: expected 64 bytes, got {}",
                    bytes.len()
                ));
            }
            Keypair::from_bytes(&bytes)
                .map_err(|e| format!("Failed to create keypair from array: {}", e))
        }
        Err(e) => Err(format!("Failed to parse private key array: {}", e)),
    }
}

/// Helper function to load keypair from base58 format
///
/// Parses private key strings in base58 format, which is the standard
/// Solana wallet format used by most tools and libraries.
fn load_keypair_from_base58_format(private_key_str: &str) -> Result<Keypair, String> {
    let decoded = bs58::decode(private_key_str)
        .into_vec()
        .map_err(|e| format!("Failed to decode base58 private key: {}", e))?;

    if decoded.len() != 64 {
        return Err(format!(
            "Invalid private key length: expected 64 bytes, got {}",
            decoded.len()
        ));
    }

    Keypair::from_bytes(&decoded)
        .map_err(|e| format!("Failed to create keypair from base58: {}", e))
}

/// Get the wallet public key from the configuration
///
/// This loads the keypair and extracts just the public key.
///
/// # Returns
/// - `Ok(Pubkey)` - The wallet's public key
/// - `Err(String)` - Failed to load or parse keypair
///
/// # Example
/// ```
/// use screenerbot::config::get_wallet_pubkey;
///
/// let pubkey = get_wallet_pubkey()?;
/// println!("Wallet address: {}", pubkey);
/// ```
pub fn get_wallet_pubkey() -> Result<Pubkey, String> {
    get_wallet_keypair().map(|kp| kp.pubkey())
}

/// Get the wallet public key as a base58 string
///
/// This is useful for logging or display purposes where you need to show
/// the wallet address but don't need the Pubkey type.
///
/// # Returns
/// - `Ok(String)` - Base58 encoded public key
/// - `Err(String)` - Failed to load or parse keypair
///
/// # Example
/// ```
/// use screenerbot::config::get_wallet_pubkey_string;
///
/// let address = get_wallet_pubkey_string()?;
/// println!("Wallet address: {}", address);
/// ```
pub fn get_wallet_pubkey_string() -> Result<String, String> {
    get_wallet_pubkey().map(|pk| pk.to_string())
}

/// Get a reference to a specific config section
///
/// For simple config access, prefer using `with_config()` directly.
/// Example: `with_config(|cfg| cfg.trader.max_open_positions)`

// ============================================================================
// CONFIG UPDATE HELPERS
// ============================================================================

/// Update a config section in-memory and optionally save to disk
///
/// This is a generic helper that allows updating any config section.
/// It uses a closure to perform the update, ensuring type safety.
///
/// # Arguments
/// * `update_fn` - Closure that receives mutable Config reference and performs updates
/// * `save_to_disk` - Whether to persist changes to config.toml
///
/// # Returns
/// - `Ok(())` - Update successful
/// - `Err(String)` - Update failed with error message
///
/// # Example
/// ```
/// use screenerbot::config::update_config_section;
///
/// // Update trader config
/// update_config_section(
///     |cfg| {
///         cfg.trader.max_open_positions = 3;
///         cfg.trader.trade_size_sol = 0.01;
///     },
///     true  // Save to disk
/// )?;
/// ```
pub fn update_config_section<F>(update_fn: F, save_to_disk: bool) -> Result<(), String>
where
    F: FnOnce(&mut Config),
{
    let config_lock = CONFIG
        .get()
        .ok_or("Config not initialized. Call load_config() first.")?;

    {
        let mut config = config_lock
            .write()
            .map_err(|e| format!("Failed to acquire config write lock: {}", e))?;

        // Apply the update
        update_fn(&mut config);
    } // Lock released here

    // Optionally save to disk (without holding the lock)
    if save_to_disk {
        save_config(None)?;
    }

    Ok(())
}

/// Get a snapshot of config state before and after an update
///
/// Useful for tracking changes and generating diff responses.
///
/// # Arguments
/// * `get_section` - Closure to extract the section to track
/// * `update_fn` - Closure to perform the update
/// * `save_to_disk` - Whether to persist changes
///
/// # Returns
/// - `Ok((old_value, new_value))` - Update successful with before/after snapshots
/// - `Err(String)` - Update failed
///
/// # Example
/// ```
/// use screenerbot::config::update_with_diff;
///
/// let (old, new) = update_with_diff(
///     |cfg| cfg.trader.clone(),
///     |cfg| { cfg.trader.max_open_positions = 3; },
///     true
/// )?;
///
/// println!("Changed from {} to {}", old.max_open_positions, new.max_open_positions);
/// ```
pub fn update_with_diff<F, T>(
    get_section: impl Fn(&Config) -> T,
    update_fn: F,
    save_to_disk: bool,
) -> Result<(T, T), String>
where
    F: FnOnce(&mut Config),
    T: Clone,
{
    let old_value = with_config(|cfg| get_section(cfg));

    update_config_section(update_fn, save_to_disk)?;

    let new_value = with_config(|cfg| get_section(cfg));

    Ok((old_value, new_value))
}
