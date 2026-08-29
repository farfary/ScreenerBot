//! Configuration helper utilities — validation, default values, and config access functions.

use super::schemas::Config;
use super::{Error, Result};
use crate::errors::{ConfigurationError, IoError};
use crate::logger::{self, LogTag};

/// Configuration utilities - loading, reloading, and access helpers
///
/// This module provides utility functions for working with the configuration system:
/// - Loading configuration from disk
/// - Hot-reloading configuration at runtime
/// - Thread-safe access helpers
/// - File watching for automatic reloads
use std::sync::OnceLock;
use std::sync::RwLock;

/// Global configuration instance
///
/// This is the single source of truth for all configuration values.
/// Access it using the helper functions below.
pub static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

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
/// fn main() -> crate::config::Result<()> {
/// load_config()?;
/// // Config is now available globally
/// Ok(())
/// }
/// ```
pub fn load_config() -> Result<()> {
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
pub fn load_config_from_path(path: &str) -> Result<()> {
    let raw = if std::path::Path::new(path).exists() {
        Some(std::fs::read_to_string(path).map_err(|e| IoError::Generic {
            message: format!("Failed to read config file '{path}': {e}"),
        })?)
    } else {
        // Use defaults if file doesn't exist
        crate::logger::warning(
            crate::logger::LogTag::System,
            &format!("Config file '{path}' not found, using default values"),
        );
        None
    };

    let mut config = match &raw {
        Some(contents) => toml::from_str::<Config>(contents).map_err(|e| Error::ParseFailed {
            detail: format!("Failed to parse config file '{path}': {e}"),
        })?,
        None => Config::default(),
    };

    // One-time migration of the legacy [ai] / [agents] sections. Only runs when
    // the file exists and still carries a legacy table.
    let migrated = match &raw {
        Some(contents) => super::migrate::migrate_legacy_sections(contents, &mut config)?,
        None => false,
    };

    // Ensure all navigation tabs are present (handles migrations like wallet -> wallets, adds new tabs like tools)
    config.gui.dashboard.navigation.tabs =
        crate::config::schemas::ensure_all_tabs_present(config.gui.dashboard.navigation.tabs);

    if migrated {
        write_config_atomic(&config, path)?;
        logger::info(
            LogTag::System,
            "Migrated legacy [ai]/[agents] config into llm/llm_analysis/assistant/agent_control",
        );
    }

    CONFIG
        .set(RwLock::new(config))
        .map_err(|_| ConfigurationError::Generic {
            message: "Config already initialized".to_owned(),
        })?;

    Ok(())
}

/// Serialize `config` and replace `path` as atomically as the platform allows
/// (write a sibling temp file, fsync it, then rename over `path`). Used by the
/// legacy-section migration so the rewrite is crash-safe and never widens access
/// to the secrets in `config.toml`.
///
/// Permissions: `config.toml` holds provider API keys, so the temp file is
/// created with mode `0600` **before its first byte is written** (via
/// `OpenOptionsExt::mode`) — there is no window in which a secret-bearing file
/// exists at the process umask. When `path` already exists, its owner
/// read/write bits are preserved but nothing wider: an existing `0644`/`0640`
/// is tightened to `0600`, never copied.
///
/// Atomicity: on Unix `rename(2)` replaces the destination atomically. On
/// Windows `std::fs::rename` fails if the destination exists, so the existing
/// file is removed first and the replacement is therefore **not** atomic there;
/// a crash between the two steps can leave `path` missing. The migration only
/// runs on the local desktop config and re-derives from the still-present
/// legacy tables on the next start, so this is acceptable but not silently
/// claimed to be atomic.
///
/// Concurrency: the sibling temp path carries the pid **and** a process-monotonic
/// counter, and is opened `O_EXCL` (`create_new`) with a bounded retry. Two
/// concurrent callers in this process therefore never select — let alone
/// truncate — the same temp, and a stale leftover temp is stepped over rather
/// than clobbered.
pub(crate) fn write_config_atomic(config: &Config, path: &str) -> Result<()> {
    use std::io::Write;

    let body = toml::to_string_pretty(config).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to serialize migrated config: {e}"),
    })?;

    let dest = std::path::Path::new(path);

    // Owner read/write only, and never wider — even if `path` is currently
    // group/world readable. Fall back to 0600 when there is no existing file or
    // it somehow carries no owner-read bit (a file the app must be able to
    // rewrite).
    #[cfg(unix)]
    let mode: u32 = {
        use std::os::unix::fs::PermissionsExt;
        let existing = std::fs::metadata(dest)
            .ok()
            .map(|m| m.permissions().mode() & 0o600);
        match existing {
            Some(bits) if bits & 0o400 != 0 => bits,
            _ => 0o600,
        }
    };

    // Create the temp with `O_EXCL` and the restrictive mode already applied, so
    // an existing candidate (a concurrent writer's temp, or a stale leftover) is
    // never opened or truncated. A fresh process-monotonic token per attempt
    // makes a genuine collision improbable; the bounded retry covers the residue.
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }

    let (tmp, mut file) = {
        let mut opened = None;
        let mut last_err = None;
        for _ in 0..MAX_TEMP_ATTEMPTS {
            let seq = TEMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let candidate = sibling_temp_path(dest, seq);
            match opts.open(&candidate) {
                Ok(f) => {
                    opened = Some((candidate, f));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    last_err = Some(e);
                }
                Err(e) => {
                    return Err(Error::WriteFailed {
                        detail: format!(
                            "Failed to create temp config '{}': {e}",
                            candidate.display()
                        ),
                    });
                }
            }
        }
        match opened {
            Some(pair) => pair,
            None => {
                return Err(Error::WriteFailed {
                    detail: format!(
                        "Failed to create a unique temp config beside '{path}' after \
                         {MAX_TEMP_ATTEMPTS} attempts: {}",
                        last_err
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown error".to_owned())
                    ),
                });
            }
        }
    };

    // Any failure past this point must not leave the temp behind.
    let write_and_sync = (|| -> std::io::Result<()> {
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_and_sync {
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::WriteFailed {
            detail: format!("Failed to write temp config '{}': {e}", tmp.display()),
        });
    }

    // `OpenOptionsExt::mode` is subject to the umask; re-assert the exact mode
    // so an existing over-broad file is genuinely tightened, not merely masked.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)) {
            drop(file);
            let _ = std::fs::remove_file(&tmp);
            return Err(IoError::Generic {
                message: format!("Failed to set permissions on '{}': {e}", tmp.display()),
            }
            .into());
        }
    }
    drop(file);

    #[cfg(not(unix))]
    {
        // Windows: rename onto an existing path fails. Best-effort replace; see
        // the doc comment — this is not crash-atomic.
        if dest.exists() {
            if let Err(e) = std::fs::remove_file(dest) {
                let _ = std::fs::remove_file(&tmp);
                return Err(Error::WriteFailed {
                    detail: format!("Failed to replace config '{path}': {e}"),
                });
            }
        }
    }

    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::WriteFailed {
            detail: format!("Failed to persist migrated config to '{path}': {e}"),
        }
    })?;

    // Persist the directory entry for the rename where the platform supports it.
    #[cfg(unix)]
    {
        if let Some(parent) = dest.parent() {
            if let Ok(dir) = std::fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
    }

    Ok(())
}

/// Upper bound on how many temp-name tokens `write_config_atomic` will try
/// before giving up. With a process-monotonic counter a single retry is already
/// enough in practice; the slack absorbs a burst of concurrent writers plus any
/// stale leftover temps.
const MAX_TEMP_ATTEMPTS: u32 = 16;

/// Process-monotonic token that makes every in-process temp path distinct, so
/// two concurrent `write_config_atomic` calls never select the same sibling temp.
static TEMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A never-clobbering temp path in the same directory as `dest` (so `rename`
/// stays within one filesystem), hidden and suffixed to avoid colliding with the
/// real file. `seq` is a per-attempt token from [`TEMP_SEQ`]; combined with the
/// pid it is unique across every writer that can race here.
fn sibling_temp_path(dest: &std::path::Path, seq: u64) -> std::path::PathBuf {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_owned());
    let tmp_name = format!(".{name}.migrate.{}.{seq}.tmp", std::process::id());
    match dest.parent() {
        Some(parent) => parent.join(tmp_name),
        None => std::path::PathBuf::from(tmp_name),
    }
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
pub fn reload_config() -> Result<()> {
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
pub fn validate_config(config: &Config) -> Result<()> {
    config.copy_trading.validate()?;

    // Trader validation
    if config.trader.max_open_positions == 0 {
        return Err(ConfigurationError::Generic {
            message: "trader.max_open_positions must be greater than 0".to_owned(),
        }
        .into());
    }
    if config.trader.trade_size_sol <= 0.0 {
        return Err(ConfigurationError::Generic {
            message: "trader.trade_size_sol must be greater than 0".to_owned(),
        }
        .into());
    }
    if !config.trader.trade_size_sol.is_finite() {
        return Err(ConfigurationError::Generic {
            message: "trader.trade_size_sol must be a finite number".to_owned(),
        }
        .into());
    }
    if config.trader.entry_check_concurrency == 0 {
        return Err(ConfigurationError::Generic {
            message: "trader.entry_check_concurrency must be at least 1".to_owned(),
        }
        .into());
    }

    // DCA validation
    if config.trader.dca_enabled {
        if config.trader.dca_threshold_pct >= 0.0 {
            return Err(ConfigurationError::Generic {
                message:
                    "trader.dca_threshold_pct must be negative (represents price drop percentage)"
                        .to_owned(),
            }
            .into());
        }
        if config.trader.dca_size_percentage <= 0.0 || config.trader.dca_size_percentage > 100.0 {
            return Err(ConfigurationError::Generic {
                message: "trader.dca_size_percentage must be between 0 and 100 (exclusive)"
                    .to_owned(),
            }
            .into());
        }
        if config.trader.dca_max_count == 0 {
            return Err(ConfigurationError::Generic {
                message: "trader.dca_max_count must be at least 1 when DCA is enabled".to_owned(),
            }
            .into());
        }
    }

    // ROI exit validation
    if config.trader.roi_target_percent <= 0.0 {
        return Err(ConfigurationError::Generic {
            message: "trader.roi_target_percent must be greater than 0".to_owned(),
        }
        .into());
    }
    if !config.trader.roi_target_percent.is_finite() {
        return Err(ConfigurationError::Generic {
            message: "trader.roi_target_percent must be a finite number".to_owned(),
        }
        .into());
    }

    // Time override validation
    if config.trader.time_override_enabled {
        if config.trader.time_override_duration <= 0.0 {
            return Err(ConfigurationError::Generic {
                message: "trader.time_override_duration must be greater than 0".to_owned(),
            }
            .into());
        }
        if !config.trader.time_override_duration.is_finite() {
            return Err(ConfigurationError::Generic {
                message: "trader.time_override_duration must be a finite number".to_owned(),
            }
            .into());
        }

        // Validate unit
        use crate::config::TimeUnit;
        let unit = TimeUnit::from_str(&config.trader.time_override_unit).ok_or_else(|| {
            ConfigurationError::Generic {
                message: format!("Invalid time_override_unit: '{}'. Must be 'seconds', 'minutes', 'hours', or 'days'", config.trader.time_override_unit),
            }
        })?;

        // Validate duration based on unit (max 30 days in any unit)
        let max_seconds = 30.0 * 86400.0; // 30 days
        let duration_seconds = unit.to_seconds(config.trader.time_override_duration);
        if duration_seconds > max_seconds {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.time_override_duration ({} {}) exceeds maximum of 30 days",
                    config.trader.time_override_duration, config.trader.time_override_unit
                ),
            }
            .into());
        }

        if config.trader.time_override_loss_threshold_percent > 0.0 {
            return Err(ConfigurationError::Generic {
                message: "trader.time_override_loss_threshold_percent must be <= 0 (represents loss percentage)"
                    .to_owned(),
            }
            .into());
        }
        if !config
            .trader
            .time_override_loss_threshold_percent
            .is_finite()
        {
            return Err(ConfigurationError::Generic {
                message: "trader.time_override_loss_threshold_percent must be a finite number"
                    .to_owned(),
            }
            .into());
        }
        if config.trader.time_override_loss_threshold_percent < -100.0 {
            return Err(ConfigurationError::Generic {
                message: "trader.time_override_loss_threshold_percent must be >= -100 (cannot lose more than 100%)"
                    .to_owned(),
            }
            .into());
        }
    }

    // Stop loss validation
    if config.trader.stop_loss_enabled {
        if config.trader.stop_loss_threshold_pct <= 0.0 {
            return Err(ConfigurationError::Generic {
                message: "trader.stop_loss_threshold_pct must be greater than 0 (represents loss percentage)"
                    .to_owned(),
            }
            .into());
        }
        if config.trader.stop_loss_threshold_pct > 100.0 {
            return Err(ConfigurationError::Generic {
                message:
                    "trader.stop_loss_threshold_pct must be <= 100 (cannot lose more than 100%)"
                        .to_owned(),
            }
            .into());
        }
        if !config.trader.stop_loss_threshold_pct.is_finite() {
            return Err(ConfigurationError::Generic {
                message: "trader.stop_loss_threshold_pct must be a finite number".to_owned(),
            }
            .into());
        }
    }

    // Positions validation
    if config.positions.profit_extra_needed_sol < 0.0
        || !config.positions.profit_extra_needed_sol.is_finite()
    {
        return Err(ConfigurationError::Generic {
            message: "positions.profit_extra_needed_sol must be non-negative and finite".to_owned(),
        }
        .into());
    }
    if config.positions.position_open_cooldown_secs < 0 {
        return Err(ConfigurationError::Generic {
            message: "positions.position_open_cooldown_secs cannot be negative".to_owned(),
        }
        .into());
    }

    // Partial exit validation
    if config.positions.partial_exit_enabled {
        if config.positions.partial_exit_default_pct < 10.0
            || config.positions.partial_exit_default_pct > 90.0
        {
            return Err(ConfigurationError::Generic {
                message: "positions.partial_exit_default_pct must be between 10 and 90".to_owned(),
            }
            .into());
        }
    }

    // Trailing stop validation
    if config.positions.trailing_stop_enabled {
        if config.positions.trailing_stop_activation_pct <= 0.0
            || config.positions.trailing_stop_activation_pct > 100.0
        {
            return Err(ConfigurationError::Generic {
                message:
                    "positions.trailing_stop_activation_pct must be between 0 and 100 (exclusive)"
                        .to_owned(),
            }
            .into());
        }
        if config.positions.trailing_stop_distance_pct <= 0.0
            || config.positions.trailing_stop_distance_pct > 100.0
        {
            return Err(ConfigurationError::Generic {
                message:
                    "positions.trailing_stop_distance_pct must be between 0 and 100 (exclusive)"
                        .to_owned(),
            }
            .into());
        }
        if config.positions.trailing_stop_distance_pct
            >= config.positions.trailing_stop_activation_pct
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "positions.trailing_stop_distance_pct ({:.1}%) must be less than trailing_stop_activation_pct ({:.1}%)",
                    config.positions.trailing_stop_distance_pct,
                    config.positions.trailing_stop_activation_pct
                ),
            }
            .into());
        }
    }

    // Slippage validation
    if config.swaps.slippage.quote_default_pct < 0.0
        || config.swaps.slippage.quote_default_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "swaps.slippage.quote_default_pct must be between 0 and 100".to_owned(),
        }
        .into());
    }
    if config.swaps.slippage.exit_profit_shortfall_pct < 0.0
        || config.swaps.slippage.exit_profit_shortfall_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "swaps.slippage.exit_profit_shortfall_pct must be between 0 and 100"
                .to_owned(),
        }
        .into());
    }
    if config.swaps.slippage.exit_loss_shortfall_pct < 0.0
        || config.swaps.slippage.exit_loss_shortfall_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "swaps.slippage.exit_loss_shortfall_pct must be between 0 and 100".to_owned(),
        }
        .into());
    }
    if config.swaps.slippage.exit_retry_steps_pct.is_empty() {
        return Err(ConfigurationError::Generic {
            message: "swaps.slippage.exit_retry_steps_pct cannot be empty - at least one slippage step is required".to_owned(),
        }
        .into());
    }

    // Router availability check - Jupiter is the primary user-configurable router
    if !config.swaps.jupiter.enabled {
        return Err(ConfigurationError::Generic {
            message: "Jupiter router must be enabled (primary swap router)".to_owned(),
        }
        .into());
    }

    // RPC validation
    if config.rpc.urls.is_empty() {
        return Err(ConfigurationError::Generic {
            message: "rpc.urls cannot be empty - at least one RPC endpoint is required".to_owned(),
        }
        .into());
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
pub fn reload_config_from_path(path: &str) -> Result<()> {
    let contents = std::fs::read_to_string(path).map_err(|e| IoError::Generic {
        message: format!("Failed to read config file '{path}': {e}"),
    })?;

    let mut new_config = toml::from_str::<Config>(&contents).map_err(|e| Error::ParseFailed {
        detail: format!("Failed to parse config file '{path}': {e}"),
    })?;

    // One-time migration of the legacy [ai] / [agents] sections on the reload path too.
    let migrated = super::migrate::migrate_legacy_sections(&contents, &mut new_config)?;

    // Ensure all navigation tabs are present (handles migrations)
    new_config.gui.dashboard.navigation.tabs =
        crate::config::schemas::ensure_all_tabs_present(new_config.gui.dashboard.navigation.tabs);

    // Validate configuration before applying
    validate_config(&new_config)?;

    if migrated {
        write_config_atomic(&new_config, path)?;
        logger::info(
            LogTag::System,
            "Migrated legacy [ai]/[agents] config into llm/llm_analysis/assistant/agent_control on reload",
        );
    }

    if let Some(config_lock) = CONFIG.get() {
        let mut config = config_lock
            .write()
            .map_err(|e| ConfigurationError::Generic {
                message: format!("Failed to acquire config write lock: {e}"),
            })?;
        *config = new_config;
        Ok(())
    } else {
        Err(Error::NotLoaded)
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
pub fn save_config(path: Option<&str>) -> Result<()> {
    let default_path = crate::paths::get_config_path();
    let default_path_str = default_path.to_string_lossy();
    let path = path.unwrap_or(&default_path_str);

    let config_str = with_config(|cfg| {
        toml::to_string_pretty(cfg).map_err(|e| Error::WriteFailed {
            detail: format!("Failed to serialize config: {e}"),
        })
    })?;

    std::fs::write(path, config_str).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to write config file '{path}': {e}"),
    })?;

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
/// wallet_encrypted: "encrypted_base64".to_owned(),
/// wallet_nonce: "nonce_base64".to_owned(),
/// ..Default::default()
/// };
/// save_config_to_file(&config, "data/config.toml", true)?;
/// ```
pub fn save_config_to_file(config: &Config, path: &str, set_global: bool) -> Result<()> {
    // Validate configuration before saving
    validate_config(config)?;

    // Serialize to TOML
    let config_str = toml::to_string_pretty(config).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to serialize config: {e}"),
    })?;

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| IoError::Generic {
            message: format!("Failed to create config directory: {e}"),
        })?;
    }

    // Write to file
    std::fs::write(path, config_str).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to write config file '{path}': {e}"),
    })?;

    // Set restrictive permissions on Unix systems (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| IoError::Generic {
                message: format!("Failed to get file metadata: {e}"),
            })?
            .permissions();
        perms.set_mode(0o600); // rw------- (owner read/write only)
        std::fs::set_permissions(path, perms).map_err(|e| IoError::Generic {
            message: format!("Failed to set file permissions: {e}"),
        })?;
    }

    logger::info(
        LogTag::System,
        &format!("Config saved to '{path}'with secure permissions"),
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
                .map_err(|_| ConfigurationError::Generic {
                    message: "Config already initialized".to_owned(),
                })?;
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

#[cfg(test)]
mod atomic_write_tests {
    use super::*;

    #[test]
    fn round_trips_a_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_config_atomic(&Config::default(), &path.to_string_lossy()).unwrap();

        let reread = std::fs::read_to_string(&path).unwrap();
        toml::from_str::<Config>(&reread).expect("written config parses back");
        // No temp file left in the directory.
        assert!(!has_temp_file(dir.path()));
    }

    #[cfg(unix)]
    fn mode_of(path: &std::path::Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn has_temp_file(dir: &std::path::Path) -> bool {
        std::fs::read_dir(dir).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        })
    }

    #[cfg(unix)]
    #[test]
    fn fresh_file_is_created_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_config_atomic(&Config::default(), &path.to_string_lossy()).unwrap();
        assert_eq!(mode_of(&path), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn overly_broad_existing_mode_is_tightened_not_copied() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "x = 1\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_config_atomic(&Config::default(), &path.to_string_lossy()).unwrap();
        assert_eq!(mode_of(&path), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn narrower_existing_owner_mode_is_preserved() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "x = 1\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();

        write_config_atomic(&Config::default(), &path.to_string_lossy()).unwrap();
        assert_eq!(mode_of(&path), 0o400);
    }

    #[cfg(unix)]
    #[test]
    fn temp_is_removed_when_the_replace_fails() {
        // Renaming the temp file onto an existing *directory* fails; the temp
        // must not survive the error.
        let dir = tempfile::tempdir().unwrap();
        let clash = dir.path().join("config.toml");
        std::fs::create_dir(&clash).unwrap();

        let err = write_config_atomic(&Config::default(), &clash.to_string_lossy());
        assert!(err.is_err());
        assert!(!has_temp_file(dir.path()), "temp file leaked on error path");
    }

    #[test]
    fn existing_temp_candidate_is_never_opened_or_truncated() {
        // Squat a file at a candidate temp path and try to create it with the
        // exact options `write_config_atomic` uses: `O_EXCL` must refuse it and
        // leave its bytes intact — the writer then moves to the next token.
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("config.toml");
        let victim = sibling_temp_path(&dest, 0);
        std::fs::write(&victim, b"PRECIOUS").unwrap();

        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        let err = opts.open(&victim).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&victim).unwrap(), b"PRECIOUS");
    }

    #[test]
    fn concurrent_writes_do_not_share_a_temp_or_corrupt_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let path_str = path.to_string_lossy().into_owned();

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let p = path_str.clone();
                std::thread::spawn(move || write_config_atomic(&Config::default(), &p))
            })
            .collect();
        for h in handles {
            h.join()
                .unwrap()
                .expect("every racing atomic write succeeds");
        }

        // A shared temp would have surfaced as a rename error above or a leaked /
        // half-written temp here; the destination must be one clean document.
        toml::from_str::<Config>(&std::fs::read_to_string(&path).unwrap())
            .expect("final config is a clean, complete document");
        assert!(
            !has_temp_file(dir.path()),
            "a concurrent writer leaked its temp"
        );
    }
}
