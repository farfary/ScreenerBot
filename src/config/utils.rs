/// Configuration utilities - loading, reloading, and access helpers
///
/// This module provides utility functions for working with the configuration system:
/// - Loading configuration from disk
/// - Hot-reloading configuration at runtime
/// - Thread-safe access helpers
/// - File watching for automatic reloads

use once_cell::sync::OnceCell;
use std::sync::RwLock;
use super::schemas::Config;

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
        let contents = std::fs
            ::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        toml
            ::from_str::<Config>(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?
    } else {
        // Use defaults if file doesn't exist
        eprintln!("⚠️  Config file '{}' not found, using default values", path);
        Config::default()
    };

    CONFIG.set(RwLock::new(config)).map_err(|_| "Config already initialized".to_string())?;

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

/// Reload configuration from a specific file path
///
/// # Arguments
/// * `path` - Path to the TOML configuration file
///
/// # Returns
/// - `Ok(())` - Configuration reloaded successfully
/// - `Err(String)` - Error message if reloading failed
pub fn reload_config_from_path(path: &str) -> Result<(), String> {
    let contents = std::fs
        ::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

    let new_config = toml
        ::from_str::<Config>(&contents)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

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
pub fn with_config<F, R>(f: F) -> R where F: FnOnce(&Config) -> R {
    let config_lock = CONFIG.get().expect("Config not initialized. Call load_config() first.");

    let config = config_lock.read().expect("Failed to acquire config read lock");

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

    std::fs
        ::write(path, config_str)
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

/// Get a reference to a specific config section
///
/// For simple config access, prefer using `with_config()` directly.
/// Example: `with_config(|cfg| cfg.trader.max_open_positions)`

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.trader.max_open_positions, 2);
        assert_eq!(config.trader.trade_size_sol, 0.005);
        assert_eq!(config.filtering.target_filtered_tokens, 1000);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[trader]"));
        assert!(toml_str.contains("[filtering]"));
    }
}
