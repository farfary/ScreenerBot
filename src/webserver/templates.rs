//! HTML template rendering for the webserver dashboard
//!
//! This module provides functions to render HTML pages by combining templates with dynamic data.
//! All embedded assets (HTML, CSS, JS) are imported from the `embeds` module.

use crate::{arguments, version};

// Import all embedded assets from the embeds module
use super::embeds::*;

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    use crate::global;

    let asset_version = option_env!("ASSET_VERSION_TS")
        .map(|ts| format!("{}-{}", version::get_version(), ts))
        .unwrap_or_else(|| version::get_version().to_string());

    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);

    // Inject security credentials for GUI mode
    let is_gui = global::is_gui_mode();
    let security_token = if is_gui {
        global::get_security_token().unwrap_or_default()
    } else {
        String::new()
    };
    let port = global::get_webserver_port();

    html = html.replace("{{SECURITY_TOKEN}}", &security_token);
    html = html.replace("{{WEBSERVER_PORT}}", &port.to_string());
    html = html.replace("{{IS_GUI_MODE}}", if is_gui { "true" } else { "false" });
    html = html.replace("{{ASSET_VERSION}}", asset_version.as_str());

    // Inject initialization state for early DOM setup (prevents dashboard flash).
    // preview mode shows the dashboard immediately, so it does not "need init".
    let needs_initialization = !global::is_initialization_complete() && !global::is_preview_mode();
    html = html.replace(
        "{{NEEDS_INITIALIZATION}}",
        if needs_initialization {
            "true"
        } else {
            "false"
        },
    );

    // Inject splash, onboarding, setup, and lockscreen screens
    html = html.replace("{{SPLASH_SCREEN}}", SPLASH_PAGE);
    html = html.replace("{{ONBOARDING_SCREEN}}", ONBOARDING_PAGE);
    html = html.replace("{{SETUP_SCREEN}}", SETUP_PAGE);
    html = html.replace("{{LOCKSCREEN}}", LOCKSCREEN_PAGE);

    // Prepare Lucide icon font CSS with corrected paths
    let lucide_css = LUCIDE_ICON_CSS
        .replace("url('lucide.eot", "url('/assets/fonts/lucide.eot")
        .replace("url('lucide.woff2", "url('/assets/fonts/lucide.woff2")
        .replace("url('lucide.woff", "url('/assets/fonts/lucide.woff")
        .replace("url('lucide.ttf", "url('/assets/fonts/lucide.ttf")
        .replace("url('lucide.svg", "url('/assets/fonts/lucide.svg");

    let mut combined_styles = vec![
        FOUNDATION_STYLES,
        SCROLLBAR_STYLES, // Global scrollbar system - must be early to set defaults
        FLOATING_STYLES,  // Global floating UI system (dropdowns, popovers, dialogs)
        &lucide_css,
        LAYOUT_STYLES,
        HEADER_STYLES,
        HEADER_RESPONSIVE_STYLES,
        COMPONENT_STYLES,
        DROPDOWN_STYLES,
        COMMON_STYLES,
        FORM_CONTROLS_STYLES,
        NOTIFICATION_STYLES,
        TOAST_STYLES,
        DATA_TABLE_STYLES,
        DATA_TABLE_CORE_STYLES,
        DATA_TABLE_COLUMN_TYPES_STYLES,
        DATA_TABLE_PAGINATION_STYLES,
        TABLE_TOOLBAR_STYLES,
        EVENTS_DIALOG_STYLES,
        TRADE_ACTION_DIALOG_STYLES,
        TAB_BAR_STYLES,
        ACTION_BAR_STYLES,
        TABLE_SETTINGS_DIALOG_STYLES,
        CONFIRMATION_DIALOG_STYLES,
        POSITION_REMOVE_DIALOG_STYLES,
        SETUP_DIALOG_STYLES,
        CONTEXT_MENU_STYLES,
        ADVANCED_CHART_STYLES,
        TOKEN_DETAILS_BASE_STYLES,
        TOKEN_DETAILS_OVERVIEW_STYLES,
        TOKEN_DETAILS_SECURITY_STYLES,
        TOKEN_DETAILS_SECURITY_HOLDERS_STYLES,
        TOKEN_DETAILS_SECURITY_RISKS_STYLES,
        TOKEN_DETAILS_POSITIONS_STYLES,
        TOKEN_DETAILS_POOLS_STYLES,
        TOKEN_DETAILS_LINKS_STYLES,
        TOKEN_DETAILS_TRANSACTIONS_SHARED_STYLES,
        TRANSACTION_DETAILS_DIALOG_STYLES,
        TRANSACTION_DETAILS_TAB_CONTENT_STYLES,
        POSITION_DETAILS_BASE_STYLES,
        POSITION_DETAILS_OVERVIEW_STYLES,
        POSITION_DETAILS_CHART_STYLES,
        POSITION_DETAILS_ACTIVITY_STYLES,
        SETTINGS_BASE_STYLES,
        SETTINGS_TABS_STYLES,
        SETTINGS_SECURITY_STYLES,
        SETTINGS_UPDATES_STYLES,
        SETTINGS_DATA_STYLES,
        HINT_POPOVER_STYLES,
        SEARCH_DIALOG_STYLES,
        CUSTOM_SELECT_STYLES,
        BILLBOARD_DIALOG_STYLES,
        BILLBOARD_ROW_STYLES,
        POOL_SELECTOR_STYLES,
        EXIT_DIALOG_STYLES,
        GLOBAL_CHAT_STYLES,
        CONFIG_IMPORT_EXPORT_DIALOG_STYLES,
        // Splash, onboarding, and setup screens (always included for proper transitions)
        SPLASH_PAGE_STYLES,
        ONBOARDING_PAGE_STYLES,
        SETUP_PAGE_STYLES,
        SETUP_STEPS_STYLES,
        // Lockscreen overlay (security)
        LOCKSCREEN_PAGE_STYLES,
        // Status bar (always visible at bottom)
        STATUS_BAR_STYLES,
        // All page styles included upfront to prevent FOUC (Flash of Unstyled Content)
        // when navigating via SPA router. The CSS is small enough that bundling all
        // is better than risking style injection race conditions in WebView.
        SERVICES_PAGE_STYLES,
        TRANSACTIONS_PAGE_STYLES,
        EVENTS_PAGE_STYLES,
        TOKENS_PAGE_STYLES,
        POSITIONS_PAGE_STYLES,
        FILTERING_BASE_STYLES,
        FILTERING_RESULTS_STYLES,
        FILTERING_RESULTS_EXPLORER_STYLES,
        CONFIG_PAGE_STYLES,
        CONFIG_SIDEBAR_STYLES,
        CONFIG_FIELDS_STYLES,
        CONFIG_RESPONSIVE_STYLES,
        STRATEGIES_PAGE_STYLES,
        STRATEGIES_CONDITION_CARDS_STYLES,
        STRATEGIES_MODALS_STYLES,
        STRATEGIES_EDITOR_STYLES,
        STRATEGIES_UI_COMPONENTS_STYLES,
        TRADER_PAGE_STYLES,
        WALLETS_PAGE_STYLES,
        WALLETS_MAIN_WALLET_STYLES,
        WALLETS_MODALS_STYLES,
        WALLETS_IMPORT_EXPORT_STYLES,
        TOOLS_BASE_STYLES,
        TOOLS_CONTENT_STYLES,
        TOOLS_COMPONENTS_STYLES,
        TOOLS_WALLET_STYLES,
        TOOLS_TOKEN_STYLES,
        TOOLS_TRADING_STYLES,
        AI_BASE_STYLES,
        AI_SETTINGS_TESTING_STYLES,
        AI_PROVIDERS_STYLES,
        AI_INSTRUCTIONS_STYLES,
        AI_CHAT_STYLES,
        AI_CHAT_MESSAGES_STYLES,
        AI_CHAT_INPUT_STYLES,
        AI_AUTOMATION_STYLES,
        HOME_PAGE_STYLES,
        HOME_CALENDAR_STYLES,
        UPDATES_PAGE_STYLES,
        ABOUT_PAGE_STYLES,
    ];
    // Suppress unused variable warning - active_tab was used for conditional style loading
    // but we now include all styles upfront to prevent FOUC in WebView
    let _ = active_tab;
    html = html.replace("/*__INJECTED_STYLES__*/", &combined_styles.join("\n"));
    let mut page_style_injections = String::new();
    for (page, styles) in [
        ("services", SERVICES_PAGE_STYLES),
        ("transactions", TRANSACTIONS_PAGE_STYLES),
        ("events", EVENTS_PAGE_STYLES),
        ("tokens", TOKENS_PAGE_STYLES),
        ("positions", POSITIONS_PAGE_STYLES),
        (
            "filtering",
            &[
                FILTERING_BASE_STYLES,
                FILTERING_RESULTS_STYLES,
                FILTERING_RESULTS_EXPLORER_STYLES,
            ]
            .join("\n"),
        ),
        (
            "config",
            &[
                CONFIG_PAGE_STYLES,
                CONFIG_SIDEBAR_STYLES,
                CONFIG_FIELDS_STYLES,
                CONFIG_RESPONSIVE_STYLES,
            ]
            .join("\n"),
        ),
        (
            "trader",
            &[
                TRADER_PAGE_STYLES,
                TRADER_CONFIG_COMPONENTS_STYLES,
                TRADER_CONTROLS_STYLES,
                TRADER_METRICS_STYLES,
            ]
            .join("\n"),
        ),
        (
            "wallets",
            &[
                WALLETS_PAGE_STYLES,
                WALLETS_MAIN_WALLET_STYLES,
                WALLETS_MODALS_STYLES,
                WALLETS_IMPORT_EXPORT_STYLES,
            ]
            .join("\n"),
        ),
        (
            "tools",
            &[
                TOOLS_BASE_STYLES,
                TOOLS_CONTENT_STYLES,
                TOOLS_COMPONENTS_STYLES,
                TOOLS_WALLET_STYLES,
                TOOLS_TOKEN_STYLES,
                TOOLS_TRADING_STYLES,
            ]
            .join("\n"),
        ),
        (
            "ai",
            &[
                AI_BASE_STYLES,
                AI_SETTINGS_TESTING_STYLES,
                AI_PROVIDERS_STYLES,
                AI_INSTRUCTIONS_STYLES,
                AI_CHAT_STYLES,
                AI_CHAT_MESSAGES_STYLES,
                AI_CHAT_INPUT_STYLES,
                AI_AUTOMATION_STYLES,
            ]
            .join("\n"),
        ),
        ("home", &[HOME_PAGE_STYLES, HOME_CALENDAR_STYLES].join("\n")),
        ("updates", UPDATES_PAGE_STYLES),
        ("about", ABOUT_PAGE_STYLES),
    ] {
        if styles.trim().is_empty() {
            continue;
        }
        let page_json =
            serde_json::to_string(page).expect("failed to serialize page name for style map");
        let styles_json =
            serde_json::to_string(styles).expect("failed to serialize page styles for style map");
        page_style_injections.push_str(&format!(
            "window.__PAGE_STYLES__[{}] = {};\n",
            page_json, styles_json
        ));
    }
    html = html.replace("/*__PAGE_STYLES__*/", &page_style_injections);
    html = html.replace("/*__THEME_SCRIPTS__*/", THEME_SCRIPTS);
    html
}

fn nav_tabs(active: &str) -> String {
    use crate::config;
    use crate::global;

    // In initialization mode (before config is loaded), return minimal nav.
    // preview mode is exempt: the user skipped setup to browse, so they get the
    // full configured navigation (token/filtering pages work without a wallet).
    if !global::is_initialization_complete() && !global::is_preview_mode() {
        // Only show initialization tab during setup
        let active_class = if active == "initialization" {
            " active"
        } else {
            ""
        };
        let aria_current = if active == "initialization" {
            " aria-current=\"page\""
        } else {
            ""
        };
        return format!(
            "<a href=\"/initialization\" data-page=\"initialization\" class=\"tab{}\"{}><i class=\"icon-settings\"></i> Setup</a>",
            active_class, aria_current
        );
    }

    // Get tabs from config, filter enabled ones, and sort by order
    let mut tabs = config::with_config(|cfg| cfg.gui.dashboard.navigation.tabs.clone());
    tabs.retain(|t| t.enabled);
    tabs.sort_by_key(|t| t.order);

    tabs.iter()
        .map(|tab| {
            let active_class = if tab.id == active { " active" } else { "" };
            let aria_current = if tab.id == active {
                " aria-current=\"page\""
            } else {
                ""
            };
            // Real hrefs preserve expected link behavior; data-page keeps SPA routing fast.
            format!(
                "<a href=\"/{}\" data-page=\"{}\" class=\"tab{}\"{}><i class=\"{}\"></i> {}</a>",
                tab.id, tab.id, active_class, aria_current, tab.icon, tab.label
            )
        })
        .collect::<Vec<_>>()
        .join("\n        ")
}

fn render_page(template: &str) -> String {
    template.to_string()
}

pub fn tokens_content() -> String {
    render_page(TOKENS_PAGE)
}

pub fn events_content() -> String {
    render_page(EVENTS_PAGE)
}

pub fn services_content() -> String {
    render_page(SERVICES_PAGE)
}

pub fn transactions_content() -> String {
    render_page(TRANSACTIONS_PAGE)
}

pub fn positions_content() -> String {
    render_page(POSITIONS_PAGE)
}

pub fn filtering_content() -> String {
    render_page(FILTERING_PAGE)
}

pub fn config_content() -> String {
    render_page(CONFIG_PAGE)
}

pub fn trader_content() -> String {
    // The Strategies subtab embeds the full strategy-editor markup (formerly its
    // own page) into the trader page via the {{STRATEGIES_PANEL}} placeholder.
    let html = TRADER_PAGE.replace("{{STRATEGIES_PANEL}}", STRATEGIES_PAGE);
    render_page(&html)
}

pub fn wallets_content() -> String {
    render_page(WALLETS_PAGE)
}

pub fn tools_content() -> String {
    render_page(TOOLS_PAGE)
}

pub fn ai_content() -> String {
    render_page(AI_PAGE)
}

pub fn initialization_content() -> String {
    // Legacy redirect - initialization now uses the setup screen
    render_page(SETUP_PAGE)
}

pub fn updates_content() -> String {
    render_page(UPDATES_PAGE)
}

pub fn about_content() -> String {
    render_page(ABOUT_PAGE)
}

pub fn home_content() -> String {
    render_page(HOME_PAGE)
}

pub fn login_content() -> String {
    render_page(LOGIN_PAGE)
}

/// Render the login page template (minimal template without navigation)
pub fn login_template(title: &str, content: &str) -> String {
    use crate::version;

    let asset_version = option_env!("ASSET_VERSION_TS")
        .map(|ts| format!("{}-{}", version::get_version(), ts))
        .unwrap_or_else(|| version::get_version().to_string());

    // Prepare Lucide icon font CSS with corrected paths
    let lucide_css = LUCIDE_ICON_CSS
        .replace("url('lucide.eot", "url('/assets/fonts/lucide.eot")
        .replace("url('lucide.woff2", "url('/assets/fonts/lucide.woff2")
        .replace("url('lucide.woff", "url('/assets/fonts/lucide.woff")
        .replace("url('lucide.ttf", "url('/assets/fonts/lucide.ttf")
        .replace("url('lucide.svg", "url('/assets/fonts/lucide.svg");

    // Minimal styles for login page
    let combined_styles = [FOUNDATION_STYLES, &lucide_css, LOGIN_PAGE_STYLES].join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ScreenerBot</title>
    <style>{}</style>
</head>
<body>
    {}
    <script type="module" src="/scripts/pages/login.js?v={}"></script>
</body>
</html>"#,
        title, combined_styles, content, asset_version
    )
}
