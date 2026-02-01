// GUI configuration schema

use crate::config_struct;

config_struct! {
    /// GUI/Desktop application configuration
    pub struct GuiConfig {
        /// Zoom level for the Electron webview (0.5 = 50%, 1.0 = 100%, 3.0 = 300%)
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

        /// Navigation tab settings
        navigation: NavigationConfig = NavigationConfig::default(),

        /// Lockscreen security settings
        lockscreen: LockscreenConfig = LockscreenConfig::default(),
    }
}

config_struct! {
    /// Lockscreen security settings
    pub struct LockscreenConfig {
        /// Whether lockscreen is enabled
        enabled: bool = false,

        /// Password type: "pin4", "pin6", "text"
        password_type: String = "pin6".to_string(),

        /// Hashed password (BLAKE3 hash, base64-encoded)
        password_hash: String = String::new(),

        /// Password salt (random, base64-encoded)
        password_salt: String = String::new(),

        /// Auto-lock timeout in seconds (0 = never)
        auto_lock_timeout_secs: u64 = 300,

        /// Lock on app minimize/blur (Electron only)
        lock_on_blur: bool = false,
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

        /// Show contextual help hints throughout the dashboard
        show_hints: bool = true,

        /// Show billboard row featuring promoted tokens on Home and Tokens pages
        show_billboard: bool = true,

        /// Enable sound effects throughout the dashboard
        sounds_enabled: bool = true,
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

        /// Whether onboarding has been completed (set true after first-time onboarding)
        onboarding_complete: bool = false,
    }
}

config_struct! {
    /// Navigation configuration for dashboard tabs
    pub struct NavigationConfig {
        /// List of navigation tabs with order and visibility
        tabs: Vec<TabConfig> = default_tabs(),
    }
}

config_struct! {
    /// Single navigation tab configuration
    pub struct TabConfig {
        /// Tab identifier (e.g., "home", "positions")
        id: String = "".to_string(),
        /// Display label
        label: String = "".to_string(),
        /// Icon class name (e.g., "icon-home")
        icon: String = "".to_string(),
        /// Sort order (lower = first)
        order: u32 = 0,
        /// Whether tab is visible/enabled
        enabled: bool = true,
    }
}

/// Returns the default tab configuration
pub fn default_tabs() -> Vec<TabConfig> {
    vec![
        TabConfig {
            id: "home".into(),
            label: "Home".into(),
            icon: "icon-house".into(),
            order: 0,
            enabled: true,
        },
        TabConfig {
            id: "tools".into(),
            label: "Tools".into(),
            icon: "icon-wrench".into(),
            order: 1,
            enabled: true,
        },
        TabConfig {
            id: "positions".into(),
            label: "Positions".into(),
            icon: "icon-chart-candlestick".into(),
            order: 2,
            enabled: true,
        },
        TabConfig {
            id: "tokens".into(),
            label: "Tokens".into(),
            icon: "icon-coins".into(),
            order: 3,
            enabled: true,
        },
        TabConfig {
            id: "filtering".into(),
            label: "Filtering".into(),
            icon: "icon-list-filter".into(),
            order: 4,
            enabled: true,
        },
        TabConfig {
            id: "wallets".into(),
            label: "Wallets".into(),
            icon: "icon-wallet".into(),
            order: 5,
            enabled: true,
        },
        TabConfig {
            id: "trader".into(),
            label: "Auto Trader".into(),
            icon: "icon-bot".into(),
            order: 6,
            enabled: true,
        },
        TabConfig {
            id: "strategies".into(),
            label: "Strategies".into(),
            icon: "icon-target".into(),
            order: 7,
            enabled: true,
        },
        TabConfig {
            id: "transactions".into(),
            label: "Transactions".into(),
            icon: "icon-activity".into(),
            order: 8,
            enabled: true,
        },
        TabConfig {
            id: "services".into(),
            label: "Services".into(),
            icon: "icon-server".into(),
            order: 9,
            enabled: true,
        },
        TabConfig {
            id: "config".into(),
            label: "Config".into(),
            icon: "icon-settings".into(),
            order: 10,
            enabled: true,
        },
        TabConfig {
            id: "events".into(),
            label: "Events".into(),
            icon: "icon-radio-tower".into(),
            order: 11,
            enabled: true,
        },
        TabConfig {
            id: "ai".into(),
            label: "AI".into(),
            icon: "icon-bot".into(),
            order: 12,
            enabled: true,
        },
    ]
}

/// Ensures all default tabs exist in the provided tabs list.
/// Also handles migration from old tab IDs (e.g., "wallet" -> "wallets").
/// Returns the merged list with missing tabs added and old IDs migrated.
pub fn ensure_all_tabs_present(mut tabs: Vec<TabConfig>) -> Vec<TabConfig> {
    let defaults = default_tabs();

    // Migration: rename "wallet" to "wallets" if present
    for tab in &mut tabs {
        if tab.id == "wallet" {
            tab.id = "wallets".into();
            tab.label = "Wallets".into();
        }
    }

    // Find max order to add new tabs after existing ones
    let max_order = tabs.iter().map(|t| t.order).max().unwrap_or(0);
    let mut next_order = max_order + 1;

    // Add any missing tabs from defaults
    for default_tab in defaults {
        let exists = tabs.iter().any(|t| t.id == default_tab.id);
        if !exists {
            let mut new_tab = default_tab;
            new_tab.order = next_order;
            next_order += 1;
            tabs.push(new_tab);
        }
    }

    // Sort by order
    tabs.sort_by_key(|t| t.order);

    tabs
}
