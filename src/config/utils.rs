// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Configuration helper utilities — validation, default values, and config access functions.

use super::schemas::{Config, SolanaChainConfig};
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
use std::sync::Mutex;
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

    let ParsedConfig {
        config,
        migrated,
        legacy_partial_exit_switch,
    } = match &raw {
        Some(contents) => parse_config_document(contents).map_err(|e| match e {
            Error::ParseFailed { detail } => Error::ParseFailed {
                detail: format!("Failed to parse config file '{path}': {detail}"),
            },
            other => other,
        })?,
        None => ParsedConfig {
            config: Config::default(),
            migrated: false,
            legacy_partial_exit_switch: None,
        },
    };

    // The initial load does not run validate_config; gate the one condition
    // whose absence would panic later (the process chain must exist) instead
    // of making every other field's validation retroactively fatal here.
    if !config.chains.has_enabled_chain() {
        return Err(ConfigurationError::Generic {
            message: "chains: at least one supported chain must be enabled (supported: solana)"
                .to_owned(),
        }
        .into());
    }

    CONFIG
        .set(RwLock::new(config))
        .map_err(|_| ConfigurationError::Generic {
            message: "Config already initialized".to_owned(),
        })?;
    // Set once, by the load that set CONFIG; a hot reload reads a file this
    // release wrote or migrated with the same rule, so it never sets it.
    let _ = LEGACY_PARTIAL_EXIT_SWITCH.set(legacy_partial_exit_switch);

    // A migrated layout is written only once startup completes, so a core that
    // fails before then leaves the file as the previous release wrote it.
    if migrated {
        *PENDING_MIGRATION_WRITE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(path.to_owned());
        logger::info(
            LogTag::System,
            "Config uses a legacy layout; migrated in memory, written once startup completes",
        );
    }

    Ok(())
}

/// Config file whose legacy layout was migrated in memory at load and still has
/// to be written; set by `load_config_from_path`, taken by
/// `persist_pending_config_migration`.
static PENDING_MIGRATION_WRITE: Mutex<Option<String>> = Mutex::new(None);

/// The retired `positions.partial_exit_enabled` switch as the config file held
/// it at load, before the migration dropped it; set once by
/// `load_config_from_path`.
static LEGACY_PARTIAL_EXIT_SWITCH: OnceLock<Option<bool>> = OnceLock::new();

/// The retired auto-trader partial-exit switch as the config file loaded by this
/// process held it: `None` when the file did not carry it or no config was
/// loaded. Store migrations that must keep the stop-loss behaviour an upgraded
/// install ran with read it; the config itself no longer has the switch.
pub fn legacy_partial_exit_switch() -> Option<bool> {
    LEGACY_PARTIAL_EXIT_SWITCH.get().copied().flatten()
}

/// Write the config migrated at load back to its file. Called once startup has
/// completed; a no-op when the load found nothing to migrate. A failed write is
/// logged and the process keeps running: the next start migrates again.
pub fn persist_pending_config_migration() {
    let pending = PENDING_MIGRATION_WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    let Some(path) = pending else {
        return;
    };
    match with_config(|cfg| write_config_atomic(cfg, &path)) {
        Ok(()) => logger::info(
            LogTag::System,
            &format!("Migrated legacy config layout written to {path}"),
        ),
        Err(e) => logger::error(
            LogTag::System,
            &format!("Failed to write migrated config layout to {path}: {e}"),
        ),
    }
}

/// The config file whose load-time migration is still waiting for
/// [`persist_pending_config_migration`], if any.
pub(super) fn pending_migration_path() -> Option<String> {
    PENDING_MIGRATION_WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Set each dotted key of `keys` (`"trader.enabled"`) to its value in the config file at
/// `path`, read as it is on disk, and leave every other key and section as it is: the
/// layout a load-time migration has not written yet stays the one the file was loaded
/// from. Missing tables on a key's path are created.
pub(super) fn write_config_keys_to_file(path: &str, keys: &[(&str, toml::Value)]) -> Result<()> {
    let raw = std::fs::read_to_string(path).map_err(|e| IoError::Generic {
        message: format!("Failed to read config file '{path}': {e}"),
    })?;
    let mut document: toml::Table = toml::from_str(&raw).map_err(|e| Error::ParseFailed {
        detail: format!("Failed to parse config file '{path}': {e}"),
    })?;
    for (key, value) in keys {
        set_dotted_key(&mut document, key, value.clone())?;
    }
    let body = toml::to_string_pretty(&document).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to serialize config: {e}"),
    })?;
    write_config_body(&body, path)
}

/// Set the dotted `key` of `table` to `value`, creating the tables on its path.
pub(super) fn set_dotted_key(table: &mut toml::Table, key: &str, value: toml::Value) -> Result<()> {
    let mut segments: Vec<&str> = key.split('.').collect();
    let Some(last) = segments.pop().filter(|last| !last.is_empty()) else {
        return Err(ConfigurationError::Generic {
            message: format!("config key '{key}' is empty"),
        }
        .into());
    };
    let mut current = table;
    for segment in segments {
        current = current
            .entry(segment)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .ok_or_else(|| ConfigurationError::Generic {
                message: format!("config key '{key}': '{segment}' is not a table"),
            })?;
    }
    current.insert(last.to_owned(), value);
    Ok(())
}

/// A config document parsed into the canonical shape.
pub struct ParsedConfig {
    pub config: Config,
    /// True when a legacy layout was rewritten in memory or an omitted key's
    /// earlier default was written in; persisting `config` makes the migration
    /// permanent.
    pub migrated: bool,
    /// The retired auto-trader partial-exit switch as `raw` carried it, read
    /// before the migration drops it.
    pub legacy_partial_exit_switch: Option<bool>,
}

/// Parse `raw` TOML: relocate the legacy layout, typed parse, backfill
/// legacy [ai]/[agents], keep the values implied by omitted keys whose default
/// changed, normalize navigation tabs. The single parse used by load, reload
/// and the disk/memory diff.
pub fn parse_config_document(raw: &str) -> Result<ParsedConfig> {
    let table: toml::Table = toml::from_str(raw).map_err(|e| Error::ParseFailed {
        detail: e.to_string(),
    })?;
    let implied = super::migrate::ImpliedDefaults::of(&table);
    let mut document = toml::Value::Table(table);

    let legacy_partial_exit_switch = super::migrate::read_legacy_partial_exit_switch(&document)?;
    let relocated = super::relocate_legacy_layout(&mut document)?;

    // Documents without a legacy layout take the typed TOML parse, which
    // keeps line numbers in its error messages.
    let mut config = if relocated {
        document
            .try_into::<Config>()
            .map_err(|e| Error::ParseFailed {
                detail: e.to_string(),
            })?
    } else {
        toml::from_str::<Config>(raw).map_err(|e| Error::ParseFailed {
            detail: e.to_string(),
        })?
    };

    let legacy_ai = super::migrate::migrate_legacy_sections(raw, &mut config)?;
    let implied = implied.apply(&mut config);

    // Normalize the navigation tabs: renamed ids migrate (wallet -> wallets), new
    // default tabs are added and removed tabs are dropped.
    config.gui.dashboard.navigation.tabs =
        crate::config::schemas::ensure_all_tabs_present(config.gui.dashboard.navigation.tabs);

    Ok(ParsedConfig {
        config,
        migrated: relocated || legacy_ai || implied,
        legacy_partial_exit_switch,
    })
}

/// Serialize `config` and replace `path` through `utils::atomic_file::replace`. Every
/// write of `config.toml` goes through here, so a crash or a full disk mid-write never
/// leaves a truncated file, and a write never widens access to its secrets.
///
/// Permissions: `config.toml` holds provider API keys, so the file is written owner
/// read/write only. When `path` already exists, its owner read/write bits are preserved
/// but nothing wider: an existing `0644`/`0640` is tightened to `0600`, never copied.
pub(crate) fn write_config_atomic(config: &Config, path: &str) -> Result<()> {
    let body = toml::to_string_pretty(config).map_err(|e| Error::WriteFailed {
        detail: format!("Failed to serialize config: {e}"),
    })?;
    write_config_body(&body, path)
}

/// Replace `path` with `body` under the permissions [`write_config_atomic`] describes.
fn write_config_body(body: &str, path: &str) -> Result<()> {
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
    #[cfg(not(unix))]
    let mode: u32 = 0o600;

    crate::utils::atomic_file::replace(dest, body.as_bytes(), mode).map_err(|e| {
        Error::WriteFailed {
            detail: format!("Failed to persist config to '{path}': {e}"),
        }
    })
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
    if !config.chains.has_enabled_chain() {
        return Err(ConfigurationError::Generic {
            message: "chains: at least one supported chain must be enabled (supported: solana)"
                .to_owned(),
        }
        .into());
    }
    config.copy_trading.validate()?;
    config.trader.validate()?;

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

        // Validate duration based on unit (max 30 days in any unit)
        let max_seconds = 30.0 * 86400.0; // 30 days
        let duration = config.trader.time_override_duration;
        let unit = config.trader.time_override_unit;
        let duration_seconds = unit.to_seconds(duration);
        if duration_seconds > max_seconds {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.time_override_duration ({duration} {}, {duration_seconds} seconds) exceeds maximum of 30 days",
                    unit.as_str()
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

    // Partial stop-loss share, checked only while the stop loss may sell part first
    if config.trader.stop_loss_allow_partial
        && !(10.0..=90.0).contains(&config.trader.stop_loss_partial_pct)
    {
        return Err(ConfigurationError::Generic {
            message: "trader.stop_loss_partial_pct must be between 10 and 90".to_owned(),
        }
        .into());
    }

    // Trailing stop validation
    if config.trader.trailing_stop_enabled {
        if config.trader.trailing_stop_activation_pct <= 0.0
            || config.trader.trailing_stop_activation_pct > 100.0
        {
            return Err(ConfigurationError::Generic {
                message:
                    "trader.trailing_stop_activation_pct must be between 0 and 100 (exclusive)"
                        .to_owned(),
            }
            .into());
        }
        if config.trader.trailing_stop_distance_pct <= 0.0
            || config.trader.trailing_stop_distance_pct > 100.0
        {
            return Err(ConfigurationError::Generic {
                message: "trader.trailing_stop_distance_pct must be between 0 and 100 (exclusive)"
                    .to_owned(),
            }
            .into());
        }
        if config.trader.trailing_stop_distance_pct >= config.trader.trailing_stop_activation_pct {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.trailing_stop_distance_pct ({:.1}%) must be less than trailing_stop_activation_pct ({:.1}%)",
                    config.trader.trailing_stop_distance_pct,
                    config.trader.trailing_stop_activation_pct
                ),
            }
            .into());
        }
    }

    // Slippage validation
    if config.trader.slippage.quote_default_pct < 0.0
        || config.trader.slippage.quote_default_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "trader.slippage.quote_default_pct must be between 0 and 100".to_owned(),
        }
        .into());
    }
    if config.trader.slippage.exit_profit_shortfall_pct < 0.0
        || config.trader.slippage.exit_profit_shortfall_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "trader.slippage.exit_profit_shortfall_pct must be between 0 and 100"
                .to_owned(),
        }
        .into());
    }
    if config.trader.slippage.exit_loss_shortfall_pct < 0.0
        || config.trader.slippage.exit_loss_shortfall_pct > 100.0
    {
        return Err(ConfigurationError::Generic {
            message: "trader.slippage.exit_loss_shortfall_pct must be between 0 and 100".to_owned(),
        }
        .into());
    }
    if config.trader.slippage.exit_retry_steps_pct.is_empty() {
        return Err(ConfigurationError::Generic {
            message: "trader.slippage.exit_retry_steps_pct cannot be empty - at least one slippage step is required".to_owned(),
        }
        .into());
    }

    if config.chains.solana.enabled {
        validate_solana_chain(&config.chains.solana)?;
    }

    Ok(())
}

/// Solana's own settings: at least one router, the Direct engine's limits and
/// a non-empty RPC list. Checked only while Solana is enabled; a disabled
/// chain's settings never refuse the config.
fn validate_solana_chain(chain: &SolanaChainConfig) -> Result<()> {
    // At least one router must remain available. Direct-only operation is a
    // supported, deliberate configuration; the runtime has the same defense
    // for a stale/externally-mutated config.
    if !chain.swaps.jupiter.enabled && !chain.swaps.direct.enabled {
        return Err(ConfigurationError::Generic {
            message: "at least one swap router (Jupiter or Direct Pool) must be enabled".to_owned(),
        }
        .into());
    }

    // Matches the field's own declared range. A ceiling of zero would disable
    // the router silently, and one above the slider's maximum would let a
    // hand-edited config trade at a size the UI cannot even express.
    if !chain.swaps.direct.max_price_impact_pct.is_finite()
        || chain.swaps.direct.max_price_impact_pct < 0.1
        || chain.swaps.direct.max_price_impact_pct > 50.0
    {
        return Err(ConfigurationError::Generic {
            message: "chains.solana.swaps.direct.max_price_impact_pct must be between 0.1 and 50"
                .to_owned(),
        }
        .into());
    }

    if !(10..=180).contains(&chain.swaps.direct.confirmation_timeout_secs) {
        return Err(ConfigurationError::Generic {
            message:
                "chains.solana.swaps.direct.confirmation_timeout_secs must be between 10 and 180"
                    .to_owned(),
        }
        .into());
    }

    // Matches the field's own declared range. Zero would turn the comparison
    // into "first valid quote wins", and a deadline beyond the slider's
    // maximum would let one slow router hold every exit again.
    if !(100..=10_000).contains(&chain.swaps.quote_deadline_ms) {
        return Err(ConfigurationError::Generic {
            message: "chains.solana.swaps.quote_deadline_ms must be between 100 and 10000"
                .to_owned(),
        }
        .into());
    }

    // RPC validation
    if chain.rpc.urls.is_empty() {
        return Err(ConfigurationError::Generic {
            message:
                "chains.solana.rpc.urls cannot be empty - at least one RPC endpoint is required"
                    .to_owned(),
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

    let ParsedConfig {
        config: new_config,
        migrated,
        ..
    } = parse_config_document(&contents).map_err(|e| match e {
        Error::ParseFailed { detail } => Error::ParseFailed {
            detail: format!("Failed to parse config file '{path}': {detail}"),
        },
        other => other,
    })?;

    // Validate configuration before applying
    validate_config(&new_config)?;

    if migrated {
        write_config_atomic(&new_config, path)?;
        logger::info(
            LogTag::System,
            &format!("Migrated legacy config layout written to {path} on reload"),
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

/// Read the configuration without waiting: `None` when it is not loaded, the
/// lock is held for writing, or the lock is poisoned. For contexts that must
/// never block, such as the panic hook and fatal startup reporting.
pub fn try_with_config<T>(read: impl FnOnce(&Config) -> T) -> Option<T> {
    let lock = CONFIG.get()?;
    let cfg = lock.try_read().ok()?;
    Some(read(&cfg))
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

    with_config(|cfg| write_config_atomic(cfg, path))
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

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| IoError::Generic {
            message: format!("Failed to create config directory: {e}"),
        })?;
    }

    write_config_atomic(config, path)?;

    logger::info(
        LogTag::System,
        &format!("Config saved to '{path}' with secure permissions"),
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

/// Seed the global config with its defaults unless a test already loaded one,
/// for unit tests that reach code reading the config through `with_config`.
#[cfg(test)]
pub(crate) fn install_default_config() {
    let _ = CONFIG.get_or_init(|| RwLock::new(Config::default()));
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

#[cfg(test)]
mod trader_validation_tests {
    use super::*;
    use crate::config::TimeUnit;

    /// A time-override duration past 30 days is refused with the value and unit that
    /// were entered and the seconds they come to.
    #[test]
    fn an_overlong_time_override_names_the_entered_duration() {
        let mut config = Config::default();
        config.trader.time_override_enabled = true;
        config.trader.time_override_duration = 31.0;
        config.trader.time_override_unit = TimeUnit::Days;
        let message = validate_config(&config)
            .expect_err("31 days is over the maximum")
            .to_string();
        assert!(message.contains("31 days"), "{message}");
        assert!(message.contains("2678400 seconds"), "{message}");

        config.trader.time_override_duration = 30.0;
        assert!(
            validate_config(&config).is_ok(),
            "30 days is on the maximum"
        );
    }
}

#[cfg(test)]
mod solana_chain_validation_tests {
    use super::*;

    /// The route comparison's deadline is refused outside the range the field
    /// declares, at both edges; the default sits inside it.
    #[test]
    fn the_quote_deadline_is_bounded_by_its_declared_range() {
        let mut config = Config::default();
        assert!(validate_config(&config).is_ok());
        for (deadline_ms, valid) in [(99, false), (100, true), (10_000, true), (10_001, false)] {
            config.chains.solana.swaps.quote_deadline_ms = deadline_ms;
            assert_eq!(
                validate_config(&config).is_ok(),
                valid,
                "quote_deadline_ms = {deadline_ms}"
            );
        }
    }
}
