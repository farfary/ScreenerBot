//! Config update helpers — in-memory updates, diffing, and reset-to-defaults.

use super::schemas::Config;
use super::utils::{save_config, validate_config, with_config, CONFIG};
use super::{Error, Result};
use crate::errors::ConfigurationError;
use crate::logger::{self, LogTag};

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
pub fn update_config_section<F>(update_fn: F, save_to_disk: bool) -> Result<()>
where
    F: FnOnce(&mut Config),
{
    let config_lock = CONFIG.get().ok_or(Error::NotLoaded)?;

    {
        let mut config = config_lock
            .write()
            .map_err(|e| ConfigurationError::Generic {
                message: format!("Failed to acquire config write lock: {e}"),
            })?;

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
pub fn update_with_diff<F, T>(
    get_section: impl Fn(&Config) -> T,
    update_fn: F,
    save_to_disk: bool,
) -> Result<(T, T)>
where
    F: FnOnce(&mut Config),
    T: Clone,
{
    let old_value = with_config(|cfg| get_section(cfg));

    update_config_section(update_fn, save_to_disk)?;

    let new_value = with_config(|cfg| get_section(cfg));

    Ok((old_value, new_value))
}

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
pub fn reset_config_to_defaults_preserving_credentials() -> Result<()> {
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
                &format!("Failed to reset configuration: {e}"),
            );
            Err(e)
        }
    }
}
