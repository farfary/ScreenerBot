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
/// load_config()?;
/// // Config is now available globally
/// Ok(())
/// }
/// ```
pub fn load_config() -> Result<(), String> {
    let config_path = crate::paths::get_config_path();
    load_config_from_path(&config_path.to_string_lossy())
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
    let mut config = if std::path::Path::new(path).exists() {
        // Load from file
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        toml::from_str::<Config>(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?
    } else {
        // Use defaults if file doesn't exist
        crate::logger::warning(
            crate::logger::LogTag::System,
            &format!("Config file '{}' not found, using default values", path),
        );
        Config::default()
    };

    // Ensure all navigation tabs are present (handles migrations like wallet -> wallets, adds new tabs like tools)
    config.gui.dashboard.navigation.tabs =
        crate::config::schemas::ensure_all_tabs_present(config.gui.dashboard.navigation.tabs);

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
    let config_path = crate::paths::get_config_path();
    reload_config_from_path(&config_path.to_string_lossy())
}

/// Validate configuration values before applying
///
/// # Arguments
/// * `config` - Configuration to validate
///
/// # Returns
/// - `Ok(())` - Configuration is valid
/// - `Err(String)` - Validation error message
pub fn validate_config(config: &Config) -> Result<(), String> {
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

    // ROI exit validation
    if config.trader.roi_target_percent <= 0.0 {
        return Err("trader.roi_target_percent must be greater than 0".to_string());
    }
    if !config.trader.roi_target_percent.is_finite() {
        return Err("trader.roi_target_percent must be a finite number".to_string());
    }

    // Time override validation
    if config.trader.time_override_enabled {
        if config.trader.time_override_duration <= 0.0 {
            return Err("trader.time_override_duration must be greater than 0".to_string());
        }
        if !config.trader.time_override_duration.is_finite() {
            return Err("trader.time_override_duration must be a finite number".to_string());
        }

        // Validate unit
        use crate::config::TimeUnit;
        let unit = TimeUnit::from_str(&config.trader.time_override_unit)
      .ok_or_else(|| format!("Invalid time_override_unit: '{}'. Must be 'seconds', 'minutes', 'hours', or 'days'", config.trader.time_override_unit))?;

        // Validate duration based on unit (max 30 days in any unit)
        let max_seconds = 30.0 * 86400.0; // 30 days
        let duration_seconds = unit.to_seconds(config.trader.time_override_duration);
        if duration_seconds > max_seconds {
            return Err(format!(
                "trader.time_override_duration ({} {}) exceeds maximum of 30 days",
                config.trader.time_override_duration, config.trader.time_override_unit
            ));
        }

        if config.trader.time_override_loss_threshold_percent > 0.0 {
            return Err(
        "trader.time_override_loss_threshold_percent must be <= 0 (represents loss percentage)"
          .to_string(),
      );
        }
        if !config
            .trader
            .time_override_loss_threshold_percent
            .is_finite()
        {
            return Err(
                "trader.time_override_loss_threshold_percent must be a finite number".to_string(),
            );
        }
        if config.trader.time_override_loss_threshold_percent < -100.0 {
            return Err(
        "trader.time_override_loss_threshold_percent must be >= -100 (cannot lose more than 100%)"
          .to_string(),
      );
        }
    }

    // Stop loss validation
    if config.trader.stop_loss_enabled {
        if config.trader.stop_loss_threshold_pct <= 0.0 {
            return Err(
        "trader.stop_loss_threshold_pct must be greater than 0 (represents loss percentage)"
          .to_string(),
      );
        }
        if config.trader.stop_loss_threshold_pct > 100.0 {
            return Err(
                "trader.stop_loss_threshold_pct must be <= 100 (cannot lose more than 100%)"
                    .to_string(),
            );
        }
        if !config.trader.stop_loss_threshold_pct.is_finite() {
            return Err("trader.stop_loss_threshold_pct must be a finite number".to_string());
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

    // Router availability check - Jupiter is the primary user-configurable router
    if !config.swaps.jupiter.enabled {
        return Err("Jupiter router must be enabled (primary swap router)".to_string());
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

    let mut new_config = toml::from_str::<Config>(&contents)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

    // Ensure all navigation tabs are present (handles migrations)
    new_config.gui.dashboard.navigation.tabs =
        crate::config::schemas::ensure_all_tabs_present(new_config.gui.dashboard.navigation.tabs);

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
/// let cfg = get_config_clone();
/// // Can use cfg across await points
/// tokio::time::sleep(Duration::from_secs(1)).await;
/// println!("Max positions: {}", cfg.trader.max_open_positions);
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
/// * `path` - Path where to save the configuration (defaults to config path from paths module)
///
/// # Returns
/// - `Ok(())` - Configuration saved successfully
/// - `Err(String)` - Error message if saving failed
pub fn save_config(path: Option<&str>) -> Result<(), String> {
    let default_path = crate::paths::get_config_path();
    let default_path_str = default_path.to_string_lossy();
    let path = path.unwrap_or(&default_path_str);

    let config_str = with_config(|cfg| {
        toml::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize config: {}", e))
    })?;

    std::fs::write(path, config_str)
        .map_err(|e| format!("Failed to write config file '{}': {}", path, e))?;

    Ok(())
}

/// Save a specific configuration to disk and optionally load it into global CONFIG
///
/// This is used during initialization to create the initial config.toml file
/// with user-provided credentials before loading it into the global state.
///
/// # Arguments
/// * `config` - Configuration to save
/// * `path` - Path where to save the configuration file
/// * `set_global` - If true, also loads this config into the global CONFIG
///
/// # Returns
/// - `Ok(())` - Configuration saved successfully
/// - `Err(String)` - Error message if saving failed
///
/// # Example
/// ```
/// use screenerbot::config::{save_config_to_file, schemas::Config};
///
/// let config = Config {
/// wallet_encrypted: "encrypted_base64".to_string(),
/// wallet_nonce: "nonce_base64".to_string(),
/// ..Default::default()
/// };
/// save_config_to_file(&config, "data/config.toml", true)?;
/// ```
pub fn save_config_to_file(config: &Config, path: &str, set_global: bool) -> Result<(), String> {
    // Validate configuration before saving
    validate_config(config)?;

    // Serialize to TOML
    let config_str =
        toml::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Write to file
    std::fs::write(path, config_str)
        .map_err(|e| format!("Failed to write config file '{}': {}", path, e))?;

    // Set restrictive permissions on Unix systems (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o600); // rw------- (owner read/write only)
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    }

    logger::info(
        LogTag::System,
        &format!("Config saved to '{}'with secure permissions", path),
    );

    // Optionally set as global config
    if set_global {
        if CONFIG.get().is_some() {
            // Config already initialized, reload it
            reload_config_from_path(path)?;
        } else {
            // First-time initialization
            CONFIG
                .set(RwLock::new(config.clone()))
                .map_err(|_| "Config already initialized".to_string())?;
        }
        logger::info(LogTag::System, "Config loaded into global state");
    }

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

/// Load the main wallet keypair from the wallets database
///
/// This function uses the new multi-wallet system. For async code, prefer
/// using `wallets::get_main_keypair()` directly.
///
/// The private key is stored encrypted in the wallets database. This function:
/// 1. Checks if wallets module is initialized
/// 2. Falls back to legacy config.toml if not initialized
/// 3. Returns the main wallet keypair
///
/// # Returns
/// - `Ok(Keypair)` - Successfully retrieved main wallet keypair
/// - `Err(String)` - No main wallet configured or decryption failed
pub fn get_wallet_keypair() -> Result<Keypair, String> {
    // Try the new wallets module first (async -> sync bridge)
    // Use try_read to avoid blocking if we're in an async context
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're in an async context - use block_in_place
        return tokio::task::block_in_place(|| {
            handle.block_on(async {
                // Check if wallets module is initialized
                if crate::wallets::is_initialized().await {
                    return crate::wallets::get_main_keypair().await;
                }
                // Fall back to legacy config
                get_wallet_keypair_from_config()
            })
        });
    }

    // Not in async context - create a temporary runtime
    // This should only happen during early startup
    get_wallet_keypair_from_config()
}

/// Legacy function to get wallet keypair from config.toml
///
/// Used as fallback before wallets module is initialized
fn get_wallet_keypair_from_config() -> Result<Keypair, String> {
    with_config(|cfg| {
        // Check if encrypted wallet data exists
        if cfg.wallet_encrypted.is_empty() || cfg.wallet_nonce.is_empty() {
            return Err("Wallet not configured - encrypted private key is missing".to_string());
        }

        // Decrypt the private key
        let encrypted = crate::secure_storage::EncryptedData {
            ciphertext: cfg.wallet_encrypted.clone(),
            nonce: cfg.wallet_nonce.clone(),
        };

        let private_key = crate::secure_storage::decrypt_private_key(&encrypted)
            .map_err(|e| format!("Failed to decrypt wallet: {}", e))?;

        // Parse the decrypted private key (base58 or array format)
        let keypair = if private_key.starts_with('[') && private_key.ends_with(']') {
            load_keypair_from_array_format(&private_key)?
        } else {
            load_keypair_from_base58_format(&private_key)?
        };

        Ok(keypair)
    })
}

/// Helper function to load keypair from array format
///
/// Parses private key strings in the format "[1,2,3,4,...]"where each
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
/// |cfg| {
/// cfg.trader.max_open_positions = 3;
/// cfg.trader.trade_size_sol = 0.01;
/// },
/// true // Save to disk
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
/// |cfg| cfg.trader.clone(),
/// |cfg| { cfg.trader.max_open_positions = 3; },
/// true
/// )?;
///
/// println!("Changed from {} to {}", old.max_open_positions, new.max_open_positions);
/// ```
/// Reset configuration to defaults while preserving wallet and RPC URLs
///
/// This function:
/// 1. Captures current wallet private key and RPC URLs
/// 2. Creates a fresh Config with all defaults
/// 3. Restores wallet and RPC URLs
/// 4. Forces save to disk
///
/// # Returns
/// - `Ok(())` - Configuration reset successfully
/// - `Err(String)` - Reset failed
///
/// # Example
/// ```
/// use screenerbot::config::reset_config_to_defaults_preserving_credentials;
///
/// reset_config_to_defaults_preserving_credentials()?;
/// ```
pub fn reset_config_to_defaults_preserving_credentials() -> Result<(), String> {
    logger::info(LogTag::System, "Resetting configuration to defaults...");

    // 1. Capture current encrypted wallet and RPC URLs
    let (wallet_encrypted, wallet_nonce, rpc_urls) = with_config(|cfg| {
        (
            cfg.wallet_encrypted.clone(),
            cfg.wallet_nonce.clone(),
            cfg.rpc.urls.clone(),
        )
    });

    // 2. Create fresh config with defaults
    let mut fresh_config = Config::default();

    // 3. Restore preserved values
    if !wallet_encrypted.is_empty() && !wallet_nonce.is_empty() {
        fresh_config.wallet_encrypted = wallet_encrypted;
        fresh_config.wallet_nonce = wallet_nonce;
        logger::info(LogTag::System, "Preserved encrypted wallet");
    }

    if !rpc_urls.is_empty() {
        fresh_config.rpc.urls = rpc_urls;
        logger::info(
            LogTag::System,
            &format!("Preserved {} RPC URL(s)", fresh_config.rpc.urls.len()),
        );
    }

    // 4. Validate the fresh config
    validate_config(&fresh_config)?;

    // 5. Replace current config
    let result = update_config_section(
        |cfg| {
            *cfg = fresh_config;
        },
        true, // Force save to disk
    );

    match result {
        Ok(_) => {
            logger::info(
                LogTag::System,
                "Configuration reset to defaults successfully (wallet + RPC preserved)",
            );
            Ok(())
        }
        Err(e) => {
            logger::error(
                LogTag::System,
                &format!("Failed to reset configuration: {}", e),
            );
            Err(e)
        }
    }
}

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
