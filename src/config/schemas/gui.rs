// GUI configuration schema

use crate::config_struct;

config_struct! {
    /// GUI/Desktop application configuration
    pub struct GuiConfig {
        /// Zoom level for the Tauri webview (0.5 = 50%, 1.0 = 100%, 3.0 = 300%)
        zoom_level: f64 = 1.0,

        /// Dashboard interface settings
        dashboard: DashboardConfig = DashboardConfig::default(),
    }
}

config_struct! {
    /// Dashboard UI settings
    pub struct DashboardConfig {
        /// Interface settings
        interface: InterfaceConfig = InterfaceConfig::default(),

        /// Startup behavior settings
        startup: StartupConfig = StartupConfig::default(),
    }
}

config_struct! {
    /// Interface customization settings
    pub struct InterfaceConfig {
        /// Theme preference (dark, light, system)
        theme: String = "dark".to_string(),

        /// Default polling interval in milliseconds (minimum 1000)
        polling_interval_ms: u64 = 5000,

        /// Show live ticker bar in header
        show_ticker_bar: bool = true,

        /// Enable animations and transitions
        enable_animations: bool = true,

        /// Compact mode reduces padding and spacing
        compact_mode: bool = false,

        /// Auto-expand sidebar categories
        auto_expand_categories: bool = false,

        /// Default table page size
        table_page_size: u32 = 25,
    }
}

config_struct! {
    /// Startup behavior settings
    pub struct StartupConfig {
        /// Auto-start trader on application launch (disabled - for future use)
        auto_start_trader: bool = false,

        /// Default page to show on startup
        default_page: String = "dashboard".to_string(),

        /// Check for updates on startup (disabled - for future use)
        check_updates_on_startup: bool = false,

        /// Show notifications for background events
        show_background_notifications: bool = true,
    }
}
