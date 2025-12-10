/// Configuration module - organized config system with zero repetition
///
/// This module provides a clean, type-safe configuration system for ScreenerBot.
///
/// # Architecture
///
/// - `macros.rs` - The `config_struct!` macro for defining configs with embedded defaults
/// - `schemas.rs` - All configuration structures defined once with defaults
/// - `utils.rs` - Loading, reloading, and access utilities
///
/// # Usage
///
/// ## Loading configuration at startup:
/// ```
/// use screenerbot::config::load_config;
///
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     load_config()?;
///     // Config is now available globally
///     Ok(())
/// }
/// ```
///
/// ## Accessing configuration (one-liner):
/// ```
/// use screenerbot::config::with_config;
///
/// // Read a single value
/// let max_positions = with_config(|cfg| cfg.trader.max_open_positions);
///
/// // Read multiple values
/// with_config(|cfg| {
///     println!("Max positions: {}", cfg.trader.max_open_positions);
///     println!("Trade size: {}", cfg.trader.trade_size_sol);
/// });
/// ```
///
/// ## Hot-reloading configuration:
/// ```
/// use screenerbot::config::reload_config;
///
/// // After modifying data/config.toml
/// reload_config()?;
/// // New values are now active
/// ```
///
/// # Adding new configuration parameters
///
/// 1. Edit `schemas.rs` and add your field to the appropriate struct:
///    ```
///    config_struct! {
///        pub struct TraderConfig {
///            max_open_positions: usize = 2,
///            new_param: bool = false,  // ← Add this line
///        }
///    }
///    ```
///
/// 2. (Optional) Add to `data/config.toml`:
///    ```toml
///    [trader]
///    new_param = true
///    ```
///
/// 3. Use it anywhere:
///    ```
///    with_config(|cfg| cfg.trader.new_param)
///    ```
///
/// That's it! No helper functions, no boilerplate, no repetition.
// Metadata helpers (must be declared before macros so macro expansions can use them)
pub mod metadata;

// Export the macro
#[macro_use]
mod macros;

// Export schemas (all config structures)
pub mod schemas;

// Export utilities (loading, reloading, access)
pub mod utils;

// Re-export commonly used items for convenience
pub use metadata::{
    collect_config_metadata, ConfigMetadata, FieldMetadata, FieldMetadataExtras, FieldType,
};

pub use schemas::{
    Config, DashboardConfig, EventsConfig, FilteringConfig, GuiConfig, InterfaceConfig,
    MonitoringConfig, OhlcvConfig, PositionsConfig, RpcConfig, ServicesConfig, SolPriceConfig,
    StartupConfig, SwapsConfig, TimeUnit, TokensConfig, TraderConfig, WebserverConfig,
};

pub use utils::{
    get_config_clone, get_wallet_keypair, get_wallet_pubkey, get_wallet_pubkey_string,
    is_config_initialized, load_config, load_config_from_path, reload_config,
    reload_config_from_path, reset_config_to_defaults_preserving_credentials, save_config,
    update_config_section, with_config, CONFIG,
};
